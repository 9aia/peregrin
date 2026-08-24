use crate::SourceLocation;
use std::collections::HashMap;

/// Maps dotted paths (e.g. `"server.port"`) to [`SourceLocation`]s.
///
/// Pure data structure — populated by each frontend's `build_source_map`
/// function using format-specific scanning. `ac-source` itself knows
/// nothing about YAML, JSON, or TOML syntax.
///
/// # Example
///
/// ```
/// use ac_source::{SourceMap, SourceLocation};
///
/// let mut map = SourceMap::new();
/// map.insert("server", SourceLocation::new(1, 1));
/// map.insert("server.port", SourceLocation::new(2, 3));
///
/// assert_eq!(map.resolve_path("server.port"), Some(SourceLocation::new(2, 3)));
/// assert_eq!(map.resolve_path("unknown"), None);
/// ```
pub struct SourceMap {
    map: HashMap<String, SourceLocation>,
}

impl SourceMap {
    /// Creates an empty `SourceMap`.
    pub fn new() -> Self {
        SourceMap {
            map: HashMap::new(),
        }
    }

    /// Inserts a path → location mapping.
    pub fn insert(&mut self, path: impl Into<String>, location: SourceLocation) {
        self.map.insert(path.into(), location);
    }

    /// Resolves a dotted path to a [`SourceLocation`].
    ///
    /// Returns `None` if the path is not in the map.
    /// Returns `Some(1:1)` for an empty path (root).
    pub fn resolve_path(&self, path: &str) -> Option<SourceLocation> {
        if path.is_empty() {
            return Some(SourceLocation::new(1, 1));
        }
        self.map.get(path).copied()
    }

    /// Returns the number of entries.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Returns true if the map is empty.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

impl Default for SourceMap {
    fn default() -> Self {
        SourceMap::new()
    }
}
