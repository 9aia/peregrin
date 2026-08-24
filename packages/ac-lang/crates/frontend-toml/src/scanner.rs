//! TOML-specific source location scanner.
//!
//! Walks a parsed `serde_json::Value` tree and scans the raw TOML text
//! to build a `SourceMap` mapping dotted paths to `SourceLocation`s.
//!
//! TOML syntax differs from YAML/JSON:
//! - Keys use `=` not `:` (`key = value`)
//! - Nested tables use `[section]` or `[section.subsection]` headers
//! - Array of tables use `[[section]]` headers

use ac_source::{KeyMatcher, LineIndex, SourceMap, walk_value};
use serde_json::Value;

const MATCHER: KeyMatcher = KeyMatcher { separator: '=' };

/// Builds a `SourceMap` for TOML source text.
pub fn build_source_map(input: &str, value: &Value) -> SourceMap {
    let mut map = SourceMap::new();
    let lines = LineIndex::new(input);

    // First pass: find all section headers and their line positions
    let sections = find_sections(input, &lines);

    // Second pass: walk the value tree and resolve each key
    walk_value(
        value,
        "",
        &mut |_path, key, parent_path| {
            let section_line = sections
                .iter()
                .find(|s| s.path == parent_path)
                .map(|s| s.line_start);

            let search_from = section_line.unwrap_or(0);
            find_toml_key(input, &lines, key, search_from)
        },
        &mut map,
    );

    map
}

struct Section {
    /// Dotted path of the section (e.g. "server" or "server.ssl")
    path: String,
    /// Byte offset of the section header line start
    line_start: usize,
}

/// Finds all `[section]` and `[[section]]` headers in the TOML text.
fn find_sections(source: &str, lines: &LineIndex) -> Vec<Section> {
    let mut sections = Vec::new();
    let starts = lines.starts();

    for (i, &line_start) in starts.iter().enumerate() {
        let line_end = lines.line_end(i);
        let line = &source[line_start..line_end];
        let trimmed = line.trim_start();

        // [section] or [[section]]
        if trimmed.starts_with('[') {
            let inner = trimmed.trim_start_matches('[').trim_end_matches(']').trim();
            // Remove surrounding quotes if present
            let path = inner.trim_matches('"').trim_matches('\'').to_string();
            sections.push(Section { path, line_start });
        }
    }

    sections
}

/// Finds a `key = value` line in the TOML source after the given offset.
fn find_toml_key(
    source: &str,
    lines: &LineIndex,
    key: &str,
    from: usize,
) -> Option<ac_source::SourceLocation> {
    let starts = lines.starts();
    let start_line = starts
        .partition_point(|&start| start <= from)
        .saturating_sub(1);

    for (i, &line_start) in starts[start_line..].iter().enumerate() {
        let line_end = lines.line_end(start_line + i);
        let line = &source[line_start..line_end];
        let indent = ac_source::leading_whitespace(line);
        let trimmed = &line[indent..];

        // Skip empty lines and comments
        if trimmed.trim().is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Stop at the next section header
        if trimmed.starts_with('[') {
            break;
        }

        if MATCHER.match_key(trimmed, key).is_some() {
            return Some(lines.at_offset(line_start + indent));
        }
    }

    None
}
