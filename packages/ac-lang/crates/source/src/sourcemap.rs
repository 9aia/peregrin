use std::fmt;

/// A location in source text — 1-indexed line and column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SourceLocation {
    /// 1-indexed line number.
    pub line: usize,
    /// 1-indexed column number (character offset within the line).
    pub column: usize,
}

impl SourceLocation {
    pub fn new(line: usize, column: usize) -> Self {
        SourceLocation { line, column }
    }
}

impl fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

/// Maps dotted paths to [`SourceLocation`]s by scanning source text.
///
/// Works across formats (YAML, JSON, TOML) by searching for key names
/// in the raw text. Uses indentation to guide nested key lookup in
/// YAML-like formats.
///
/// # Example
///
/// ```
/// use ac_source::SourceMap;
///
/// let source = "server:\n  port: 8080\n  host: localhost\n";
/// let map = SourceMap::new(source);
/// let loc = map.resolve_path("server.port");
/// assert_eq!(loc, Some(ac_source::SourceLocation::new(2, 3)));
/// ```
pub struct SourceMap {
    source: String,
    line_starts: Vec<usize>,
}

impl SourceMap {
    /// Creates a `SourceMap` from the given source text.
    pub fn new(source: impl Into<String>) -> Self {
        let source = source.into();
        let mut line_starts = vec![0];
        for (i, b) in source.bytes().enumerate() {
            if b == b'\n' {
                line_starts.push(i + 1);
            }
        }
        SourceMap { source, line_starts }
    }

    /// Converts a byte offset to a [`SourceLocation`].
    pub fn at_byte_offset(&self, offset: usize) -> SourceLocation {
        let offset = offset.min(self.source.len());
        let line_idx = self
            .line_starts
            .partition_point(|&start| start <= offset)
            .saturating_sub(1);
        let line_start = self.line_starts[line_idx];
        let column = offset - line_start + 1;
        SourceLocation {
            line: line_idx + 1,
            column,
        }
    }

    /// Resolves a dotted path (e.g. `"server.port"`) to a source location.
    ///
    /// Also accepts JSON pointer paths (e.g. `"/server/port"`).
    /// Returns `None` if the path cannot be found in the source text.
    /// Returns `Some(1:1)` for an empty path (root).
    pub fn resolve_path(&self, path: &str) -> Option<SourceLocation> {
        if path.is_empty() {
            return Some(SourceLocation::new(1, 1));
        }

        let segments = parse_path_segments(path);
        if segments.is_empty() {
            return Some(SourceLocation::new(1, 1));
        }

        let mut search_from = 0usize;
        let mut min_indent: Option<usize> = None;
        let mut last_location: Option<SourceLocation> = None;

        for segment in &segments {
            let (key, array_index) = split_array_index(segment);

            if key.is_empty() {
                if let Some(idx) = array_index {
                    let elem_offset = self.find_array_element(search_from, min_indent, idx)?;
                    last_location = Some(self.at_byte_offset(elem_offset));
                    search_from = elem_offset;
                }
                continue;
            }

            let found = self.find_key(&key, search_from, min_indent)?;
            last_location = Some(self.at_byte_offset(found.key_offset));
            search_from = found.value_offset;
            min_indent = Some(found.indent + 1);

            if let Some(idx) = array_index {
                let elem_offset = self.find_array_element(search_from, Some(found.indent + 1), idx)?;
                last_location = Some(self.at_byte_offset(elem_offset));
                search_from = elem_offset;
            }
        }

        last_location
    }

    /// Finds a key name in the source text starting from `search_from`.
    ///
    /// Scans line by line, looking for `key:` (YAML/JSON) or `"key":` (JSON)
    /// at an indentation level >= `min_indent` (if specified).
    ///
    /// When `min_indent` is set, the line containing `from` is skipped
    /// (it belongs to the parent key).
    fn find_key(&self, key: &str, from: usize, min_indent: Option<usize>) -> Option<KeyFound> {
        let start_line = self
            .line_starts
            .partition_point(|&start| start <= from)
            .saturating_sub(1);

        for (i, &line_start) in self.line_starts[start_line..].iter().enumerate() {
            let line_end = self
                .line_starts
                .get(start_line + i + 1)
                .copied()
                .unwrap_or(self.source.len());

            // Skip the line containing `from` — it's the parent key's line
            if min_indent.is_some() && line_start <= from && from < line_end {
                continue;
            }

            let line = &self.source[line_start..line_end];

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

    /// Finds the `index`-th array element starting from `search_from`.
    ///
    /// Looks for YAML-style `- ` items at an indentation >= `min_indent`.
    ///
    /// When `min_indent` is set, the line containing `from` is skipped
    /// (it belongs to the parent key).
    fn find_array_element(
        &self,
        from: usize,
        min_indent: Option<usize>,
        index: usize,
    ) -> Option<usize> {
        let start_line = self
            .line_starts
            .partition_point(|&start| start <= from)
            .saturating_sub(1);
        let mut count = 0usize;

        for (i, &line_start) in self.line_starts[start_line..].iter().enumerate() {
            let line_end = self
                .line_starts
                .get(start_line + i + 1)
                .copied()
                .unwrap_or(self.source.len());

            // Skip the line containing `from` — it's the parent key's line
            if min_indent.is_some() && line_start <= from && from < line_end {
                continue;
            }

            let line = &self.source[line_start..line_end];

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

            if trimmed.starts_with("- ") || trimmed == "-" || trimmed.starts_with("-\t") {
                if count == index {
                    let dash_end = indent + 2;
                    return Some(line_start + dash_end);
                }
                count += 1;
            }
        }

        None
    }
}

struct KeyFound {
    key_offset: usize,
    value_offset: usize,
    indent: usize,
}

fn parse_path_segments(path: &str) -> Vec<String> {
    if path.starts_with('/') {
        let mut segments = Vec::new();
        for part in path.split('/') {
            if part.is_empty() {
                continue;
            }
            let unescaped = part.replace("~1", "/").replace("~0", "~");
            segments.push(unescaped);
        }
        segments
    } else {
        path.split('.').map(|s| s.to_string()).collect()
    }
}

fn split_array_index(segment: &str) -> (String, Option<usize>) {
    if let Some(bracket_pos) = segment.find('[') {
        let key = segment[..bracket_pos].to_string();
        let rest = &segment[bracket_pos + 1..];
        let idx_str = rest.trim_end_matches(']');
        let idx = idx_str.parse::<usize>().ok();
        (key, idx)
    } else {
        (segment.to_string(), None)
    }
}

fn leading_whitespace(s: &str) -> usize {
    s.chars().take_while(|c| *c == ' ' || *c == '\t').count()
}

fn match_key(line: &str, key: &str) -> Option<usize> {
    if let Some(after) = line.strip_prefix(key) {
        let after_trimmed = after.trim_start();
        if after_trimmed.starts_with(':') {
            let leading = after.len() - after_trimmed.len();
            let colon_end = key.len() + leading + 1;
            return Some(colon_end);
        }
    }

    let quoted_key = format!("\"{key}\"");
    if let Some(after) = line.strip_prefix(&quoted_key) {
        let after_trimmed = after.trim_start();
        if after_trimmed.starts_with(':') {
            let leading = after.len() - after_trimmed.len();
            let colon_end = quoted_key.len() + leading + 1;
            return Some(colon_end);
        }
    }

    let single_quoted_key = format!("'{key}'");
    if let Some(after) = line.strip_prefix(&single_quoted_key) {
        let after_trimmed = after.trim_start();
        if after_trimmed.starts_with(':') {
            let leading = after.len() - after_trimmed.len();
            let colon_end = single_quoted_key.len() + leading + 1;
            return Some(colon_end);
        }
    }

    None
}
