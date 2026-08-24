//! Source input for a compilation pipeline.

use std::path::PathBuf;

/// Source input for a compilation.
#[derive(Clone)]
pub struct Source {
    /// Raw input text.
    pub input: String,
    /// Format name (`yaml`, `json`, `toml`).
    pub format: String,
    /// File path of the input, if known. Used for relative import resolution.
    pub path: Option<PathBuf>,
}

impl Source {
    pub fn new(input: impl Into<String>, format: impl Into<String>) -> Self {
        Source {
            input: input.into(),
            format: format.into(),
            path: None,
        }
    }

    pub fn with_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Base path for resolving relative imports — the parent directory
    /// of the source file, if known.
    pub fn base_path(&self) -> Option<PathBuf> {
        self.path
            .as_ref()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
    }
}
