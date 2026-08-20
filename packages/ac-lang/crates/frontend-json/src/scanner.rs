//! JSON-specific source location scanner.
//!
//! Walks a parsed `serde_json::Value` tree and scans the raw JSON text
//! to build a `SourceMap` mapping dotted paths to `SourceLocation`s.

use ac_source::{SourceLocation, SourceMap};
use serde_json::Value;

/// Builds a `SourceMap` for JSON source text.
pub fn build_source_map(input: &str, value: &Value) -> SourceMap {
    let mut map = SourceMap::new();
    let lines: Vec<usize> = compute_line_starts(input);
    walk_value(value, "", input, &lines, &mut map);
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

fn walk_value(value: &Value, path: &str, source: &str, lines: &[usize], map: &mut SourceMap) {
    match value {
        Value::Object(obj) => {
            for (key, val) in obj {
                let child_path = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{path}.{key}")
                };

                if let Some(loc) = find_key_location(source, lines, key, path) {
                    map.insert(&child_path, loc);
                }

                walk_value(val, &child_path, source, lines, map);
            }
        }
        Value::Array(arr) => {
            for (i, val) in arr.iter().enumerate() {
                let child_path = format!("{path}[{i}]");
                walk_value(val, &child_path, source, lines, map);
            }
        }
        _ => {}
    }
}

fn find_key_location(
    source: &str,
    lines: &[usize],
    key: &str,
    parent_path: &str,
) -> Option<SourceLocation> {
    let search_from = if parent_path.is_empty() {
        0
    } else {
        find_path_offset(source, lines, parent_path)?
    };

    find_key_from(source, lines, key, search_from, parent_path)
}

fn find_path_offset(source: &str, lines: &[usize], path: &str) -> Option<usize> {
    let segments: Vec<&str> = path.split('.').collect();
    let mut search_from = 0usize;
    let mut min_indent: Option<usize> = None;

    for segment in &segments {
        let found = find_key_in_lines(source, lines, segment, search_from, min_indent)?;
        search_from = found.value_offset;
        min_indent = Some(found.indent + 1);
    }

    Some(search_from)
}

struct KeyFound {
    key_offset: usize,
    value_offset: usize,
    indent: usize,
}

fn find_key_from(
    source: &str,
    lines: &[usize],
    key: &str,
    from: usize,
    parent_path: &str,
) -> Option<SourceLocation> {
    let min_indent = if parent_path.is_empty() {
        None
    } else {
        let parent_segments: Vec<&str> = parent_path.split('.').collect();
        let last_key = parent_segments.last()?;
        let found = find_key_in_lines(source, lines, last_key, 0, None)?;
        Some(found.indent + 1)
    };

    let found = find_key_in_lines(source, lines, key, from, min_indent)?;
    Some(at_byte_offset(found.key_offset, lines, source))
}

fn find_key_in_lines(
    source: &str,
    lines: &[usize],
    key: &str,
    from: usize,
    min_indent: Option<usize>,
) -> Option<KeyFound> {
    let start_line = lines
        .partition_point(|&start| start <= from)
        .saturating_sub(1);

    for (i, &line_start) in lines[start_line..].iter().enumerate() {
        let line_end = lines
            .get(start_line + i + 1)
            .copied()
            .unwrap_or(source.len());

        if min_indent.is_some() && line_start <= from && from < line_end {
            continue;
        }

        let line = &source[line_start..line_end];
        let indent = leading_whitespace(line);
        let trimmed = &line[indent..];

        if trimmed.trim().is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if let Some(min) = min_indent {
            if indent < min {
                break;
            }
        }

        if let Some(key_len) = match_key(trimmed, key) {
            let key_offset = line_start + indent;
            let value_offset = line_start + indent + key_len;
            return Some(KeyFound {
                key_offset,
                value_offset,
                indent,
            });
        }
    }

    None
}

fn leading_whitespace(s: &str) -> usize {
    s.chars().take_while(|c| *c == ' ' || *c == '\t').count()
}

fn match_key(line: &str, key: &str) -> Option<usize> {
    // Unquoted: key:
    if line.starts_with(key) {
        let after = &line[key.len()..];
        let after_trimmed = after.trim_start();
        if after_trimmed.starts_with(':') {
            let leading = after.len() - after_trimmed.len();
            return Some(key.len() + leading + 1);
        }
    }

    // Double-quoted: "key":
    let quoted_key = format!("\"{key}\"");
    if line.starts_with(&quoted_key) {
        let after = &line[quoted_key.len()..];
        let after_trimmed = after.trim_start();
        if after_trimmed.starts_with(':') {
            let leading = after.len() - after_trimmed.len();
            return Some(quoted_key.len() + leading + 1);
        }
    }

    // Single-quoted: 'key':
    let single_quoted_key = format!("'{key}'");
    if line.starts_with(&single_quoted_key) {
        let after = &line[single_quoted_key.len()..];
        let after_trimmed = after.trim_start();
        if after_trimmed.starts_with(':') {
            let leading = after.len() - after_trimmed.len();
            return Some(single_quoted_key.len() + leading + 1);
        }
    }

    None
}
