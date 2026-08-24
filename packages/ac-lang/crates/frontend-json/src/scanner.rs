//! JSON-specific source location scanner.
//!
//! Walks a parsed `serde_json::Value` tree and scans the raw JSON text
//! to build a `SourceMap` mapping dotted paths to `SourceLocation`s.

use ac_source::{KeyMatcher, LineIndex, SourceMap, walk_value};
use serde_json::Value;

const MATCHER: KeyMatcher = KeyMatcher { separator: ':' };

/// Builds a `SourceMap` for JSON source text.
pub fn build_source_map(input: &str, value: &Value) -> SourceMap {
    let mut map = SourceMap::new();
    let lines = LineIndex::new(input);

    let source = input;
    walk_value(
        value,
        "",
        &mut |_path, key, parent_path| find_key_location(source, &lines, key, parent_path),
        &mut map,
    );

    map
}

fn find_key_location(
    source: &str,
    lines: &LineIndex,
    key: &str,
    parent_path: &str,
) -> Option<ac_source::SourceLocation> {
    let search_from = if parent_path.is_empty() {
        0
    } else {
        find_path_offset(source, lines, parent_path)?
    };

    let min_indent = if parent_path.is_empty() {
        None
    } else {
        let parent_segments: Vec<&str> = parent_path.split('.').collect();
        let last_key = parent_segments.last()?;
        let found = MATCHER.find_key_in_lines(source, lines, last_key, 0, None, false)?;
        Some(found.indent + 1)
    };

    let found = MATCHER.find_key_in_lines(
        source,
        lines,
        key,
        search_from,
        min_indent,
        min_indent.is_some(),
    )?;
    Some(lines.at_offset(found.key_offset))
}

fn find_path_offset(source: &str, lines: &LineIndex, path: &str) -> Option<usize> {
    let segments: Vec<&str> = path.split('.').collect();
    let mut search_from = 0usize;
    let mut min_indent: Option<usize> = None;

    for segment in &segments {
        let found = MATCHER.find_key_in_lines(
            source,
            lines,
            segment,
            search_from,
            min_indent,
            min_indent.is_some(),
        )?;
        search_from = found.value_offset;
        min_indent = Some(found.indent + 1);
    }

    Some(search_from)
}