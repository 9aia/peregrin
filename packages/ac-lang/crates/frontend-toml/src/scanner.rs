//! TOML-specific source location scanner.
//!
//! Walks a parsed `serde_json::Value` tree and scans the raw TOML text
//! to build a `SourceMap` mapping dotted paths to `SourceLocation`s.
//!
//! TOML syntax differs from YAML/JSON:
//! - Keys use `=` not `:` (`key = value`)
//! - Nested tables use `[section]` or `[section.subsection]` headers
//! - Array of tables use `[[section]]` headers

use ac_source::{SourceLocation, SourceMap};
use serde_json::Value;

/// Builds a `SourceMap` for TOML source text.
pub fn build_source_map(input: &str, value: &Value) -> SourceMap {
    let mut map = SourceMap::new();
    let lines: Vec<usize> = compute_line_starts(input);

    // First pass: find all section headers and their line positions
    let sections = find_sections(input, &lines);

    // Second pass: walk the value tree and resolve each key
    walk_value(value, "", input, &lines, &sections, &mut map);
    map
}

fn compute_line_starts(source: &str) -> Vec<usize> {
    let mut starts = vec![0];
    for (i, b) in source.bytes().enumerate() {
        if b == b'\n' {
            starts.push(i + 1);
        }
    }
    starts
}

fn at_byte_offset(offset: usize, lines: &[usize], source: &str) -> SourceLocation {
    let offset = offset.min(source.len());
    let line_idx = lines
        .partition_point(|&start| start <= offset)
        .saturating_sub(1);
    let line_start = lines[line_idx];
    SourceLocation::new(line_idx + 1, offset - line_start + 1)
}

struct Section {
    /// Dotted path of the section (e.g. "server" or "server.ssl")
    path: String,
    /// Byte offset of the section header line start
    line_start: usize,
}

/// Finds all `[section]` and `[[section]]` headers in the TOML text.
fn find_sections(source: &str, lines: &[usize]) -> Vec<Section> {
    let mut sections = Vec::new();

    for (i, &line_start) in lines.iter().enumerate() {
        let line_end = lines
            .get(i + 1)
            .copied()
            .unwrap_or(source.len());
        let line = &source[line_start..line_end];
        let trimmed = line.trim_start();

        // [section] or [[section]]
        if trimmed.starts_with('[') {
            let inner = trimmed.trim_start_matches('[').trim_end_matches(']').trim();
            // Remove surrounding quotes if present
            let path = inner
                .trim_matches('"')
                .trim_matches('\'')
                .to_string();
            sections.push(Section {
                path,
                line_start,
            });
        }
    }

    sections
}

fn walk_value(
    value: &Value,
    path: &str,
    source: &str,
    lines: &[usize],
    sections: &[Section],
    map: &mut SourceMap,
) {
    match value {
        Value::Object(obj) => {
            // Find the section that matches this path (if any)
            let section_line = sections
                .iter()
                .find(|s| s.path == path)
                .map(|s| s.line_start);

            for (key, val) in obj {
                let child_path = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{path}.{key}")
                };

                // For TOML, keys under a section are after the section header
                let search_from = section_line.unwrap_or(0);

                if let Some(loc) = find_toml_key(source, lines, key, search_from) {
                    map.insert(&child_path, loc);
                }

                walk_value(val, &child_path, source, lines, sections, map);
            }
        }
        Value::Array(arr) => {
            for (i, val) in arr.iter().enumerate() {
                let child_path = format!("{path}[{i}]");
                walk_value(val, &child_path, source, lines, sections, map);
            }
        }
        _ => {}
    }
}

/// Finds a `key = value` line in the TOML source after the given offset.
fn find_toml_key(
    source: &str,
    lines: &[usize],
    key: &str,
    from: usize,
) -> Option<SourceLocation> {
    let start_line = lines
        .partition_point(|&start| start <= from)
        .saturating_sub(1);

    for (i, &line_start) in lines[start_line..].iter().enumerate() {
        let line_end = lines
            .get(start_line + i + 1)
            .copied()
            .unwrap_or(source.len());
        let line = &source[line_start..line_end];
        let indent = leading_whitespace(line);
        let trimmed = &line[indent..];

        // Skip empty lines and comments
        if trimmed.trim().is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Stop at the next section header
        if trimmed.starts_with('[') {
            break;
        }

        if let Some(key_offset) = match_toml_key(trimmed, key) {
            return Some(at_byte_offset(line_start + indent + key_offset, lines, source));
        }
    }

    None
}

fn leading_whitespace(s: &str) -> usize {
    s.chars().take_while(|c| *c == ' ' || *c == '\t').count()
}

/// Matches a TOML key (`key = ` or `"key" = ` or `'key' = `).
/// Returns the offset of the key start within the line (always 0 if matched).
fn match_toml_key(line: &str, key: &str) -> Option<usize> {
    // Unquoted: key =
    if line.starts_with(key) {
        let after = &line[key.len()..];
        let after_trimmed = after.trim_start();
        if after_trimmed.starts_with('=') {
            return Some(0);
        }
    }

    // Double-quoted: "key" =
    let quoted_key = format!("\"{key}\"");
    if line.starts_with(&quoted_key) {
        let after = &line[quoted_key.len()..];
        let after_trimmed = after.trim_start();
        if after_trimmed.starts_with('=') {
            return Some(0);
        }
    }

    // Single-quoted: 'key' =
    let single_quoted_key = format!("'{key}'");
    if line.starts_with(&single_quoted_key) {
        let after = &line[single_quoted_key.len()..];
        let after_trimmed = after.trim_start();
        if after_trimmed.starts_with('=') {
            return Some(0);
        }
    }

    // Dotted key: section.key = (match the last segment)
    if key.contains('.') {
        // Not a direct match — this is handled via section headers
        return None;
    }

    None
}
