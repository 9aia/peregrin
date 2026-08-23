//! Shared source-location scanning utilities.
//!
//! Frontend scanners (YAML, JSON, TOML) share common line-indexing and
//! value-walking logic. This module provides the reusable primitives;
//! each frontend supplies its format-specific key-matching behavior.

use crate::{SourceLocation, SourceMap};
use serde_json::Value;

/// Index of line-start byte offsets within a source string.
///
/// Precomputed from the source text so that `O(log n)` line/column
/// lookups are possible from any byte offset.
pub struct LineIndex {
    starts: Vec<usize>,
    source_len: usize}

impl LineIndex {
    /// Build a line index from source text.
    pub fn new(source: &str) -> Self {
        let mut starts = vec![0];
        for (i, b) in source.bytes().enumerate() {
            if b == b'\n' {
                starts.push(i + 1);
            }
        }
        LineIndex {
            starts,
            source_len: source.len()}
    }

    /// Convert a byte offset into a 1-indexed `SourceLocation`.
    pub fn at_offset(&self, offset: usize) -> SourceLocation {
        let offset = offset.min(self.source_len);
        let line_idx = self
            .starts
            .partition_point(|&start| start <= offset)
            .saturating_sub(1);
        let line_start = self.starts[line_idx];
        SourceLocation::new(line_idx + 1, offset - line_start + 1)
    }

    /// Returns the line-start byte offsets.
    pub fn starts(&self) -> &[usize] {
        &self.starts
    }

    /// Returns the byte offset where the given line (0-indexed) begins.
    pub fn line_start(&self, line_idx: usize) -> usize {
        self.starts
            .get(line_idx)
            .copied()
            .unwrap_or(self.source_len)
    }

    /// Returns the byte offset where the given line (0-indexed) ends
    /// (exclusive — i.e. the start of the next line, or `source_len`
    /// for the last line).
    pub fn line_end(&self, line_idx: usize) -> usize {
        self.starts
            .get(line_idx + 1)
            .copied()
            .unwrap_or(self.source_len)
    }
}

/// Count leading space and tab characters in a string.
pub fn leading_whitespace(s: &str) -> usize {
    s.chars().take_while(|c| *c == ' ' || *c == '\t').count()
}

/// Walk a parsed `serde_json::Value` tree, calling `on_key` for each
/// object key with its dotted path and byte-offset search context.
///
/// The `on_key` callback receives:
/// - `path` — the dotted path to the key (e.g. `"server.port"`)
/// - `key` — the key name
/// - `parent_path` — the dotted path of the parent (empty for root)
///
/// If `on_key` returns `Some(SourceLocation)`, it is inserted into the
/// `SourceMap`.
pub fn walk_value(
    value: &Value,
    path: &str,
    on_key: &mut impl FnMut(&str, &str, &str) -> Option<SourceLocation>,
    map: &mut SourceMap,
) {
    match value {
        Value::Object(obj) => {
            for (key, val) in obj {
                let child_path = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{path}.{key}")
                };

                if let Some(loc) = on_key(&child_path, key, path) {
                    map.insert(&child_path, loc);
                }

                walk_value(val, &child_path, on_key, map);
            }
        }
        Value::Array(arr) => {
            for (i, val) in arr.iter().enumerate() {
                let child_path = format!("{path}[{i}]");
                walk_value(val, &child_path, on_key, map);
            }
        }
        _ => {}
    }
}

/// Result of finding a key on a line.
pub struct KeyFound {
    /// Byte offset of the key start within the source.
    pub key_offset: usize,
    /// Byte offset of the value start (after the separator).
    pub value_offset: usize,
    /// Indentation level (leading whitespace) of the line.
    pub indent: usize}

/// Configuration for key matching on a line.
pub struct KeyMatcher {
    /// The separator character after the key (`:` for YAML/JSON, `=` for TOML).
    pub separator: char}

impl KeyMatcher {
    /// Match a key at the start of a line, supporting unquoted,
    /// double-quoted, and single-quoted keys.
    ///
    /// Returns the byte length consumed by the key + separator if matched.
    pub fn match_key(&self, line: &str, key: &str) -> Option<usize> {
        let sep = self.separator;

        // Unquoted: key<sep>
        if let Some(after) = line.strip_prefix(key) {
            let after_trimmed = after.trim_start();
            if after_trimmed.starts_with(sep) {
                let leading = after.len() - after_trimmed.len();
                return Some(key.len() + leading + 1);
            }
        }

        // Double-quoted: "key"<sep>
        let dq = format!("\"{key}\"");
        if line.starts_with(&dq) {
            let after = &line[dq.len()..];
            let after_trimmed = after.trim_start();
            if after_trimmed.starts_with(sep) {
                let leading = after.len() - after_trimmed.len();
                return Some(dq.len() + leading + 1);
            }
        }

        // Single-quoted: 'key'<sep>
        let sq = format!("'{key}'");
        if line.starts_with(&sq) {
            let after = &line[sq.len()..];
            let after_trimmed = after.trim_start();
            if after_trimmed.starts_with(sep) {
                let leading = after.len() - after_trimmed.len();
                return Some(sq.len() + leading + 1);
            }
        }

        None
    }

    /// Search for a key in the source text starting from a given byte offset,
    /// respecting indentation constraints.
    ///
    /// - `from` — byte offset to start searching from.
    /// - `min_indent` — if `Some`, only consider lines with at least this
    ///   indentation, and stop at the first line with less indentation.
    /// - `skip_from_line` — if `true`, skip the line containing the `from`
    ///   offset (used for nested lookups).
    pub fn find_key_in_lines(
        &self,
        source: &str,
        lines: &LineIndex,
        key: &str,
        from: usize,
        min_indent: Option<usize>,
        skip_from_line: bool,
    ) -> Option<KeyFound> {
        let starts = lines.starts();
        let start_line = starts
            .partition_point(|&start| start <= from)
            .saturating_sub(1);

        for (i, &line_start) in starts[start_line..].iter().enumerate() {
            let line_end = lines.line_end(start_line + i);

            if skip_from_line && line_start <= from && from < line_end {
                continue;
            }

            let line = &source[line_start..line_end];
            let indent = leading_whitespace(line);
            let trimmed = &line[indent..];

            if trimmed.trim().is_empty() || trimmed.starts_with('#') {
                continue;
            }

            if let Some(min) = min_indent
                && indent < min
            {
                break;
            }

            if let Some(key_len) = self.match_key(trimmed, key) {
                return Some(KeyFound {
                    key_offset: line_start + indent,
                    value_offset: line_start + indent + key_len,
                    indent});
            }
        }

        None
    }
}
