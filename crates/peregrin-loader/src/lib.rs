//! peregrin-loader — the campaign-loader port. Impls:
//! `peregrin-loader-yaml`. A source produces a fully-expanded
//! [`peregrin_core::Campaign`] — includes/snippets already resolved.

use std::path::Path;

/// Loader failure — the Rust analogue of the TS
/// `Data.TaggedError("LoaderError")` (`reason` + optional `cause`,
/// carried as `#[source]`).
#[derive(Debug, thiserror::Error)]
#[error("{reason}")]
pub struct LoadError {
    reason: String,
    #[source]
    cause: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl LoadError {
    /// A reason with no underlying cause.
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
            cause: None,
        }
    }

    /// A reason wrapping the I/O or parse error that caused it.
    pub fn caused(
        reason: impl Into<String>,
        cause: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self {
            reason: reason.into(),
            cause: Some(Box::new(cause)),
        }
    }

    /// The TS `reason` field — message text the TS tests assert on.
    pub fn reason(&self) -> &str {
        &self.reason
    }
}

/// The loader port — mirrors the `ac-loader-*` module surface:
/// `load(path)` and `load_str(text, base_dir)` (TS `loadWorkflow` /
/// `loadWorkflowFromText`). Impls may also expose a `load_raw`
/// intermediate returning the expanded-but-untyped mapping.
pub trait Loader: Send + Sync {
    /// Load a campaign from a file path. Includes/snippets resolve
    /// relative to the file's directory.
    fn load(&self, source: &Path) -> Result<peregrin_core::Campaign, LoadError>;

    /// Load a campaign from text; `base_dir` anchors relative includes
    /// and file discovery (TS `baseDir`, defaulting there to `cwd`).
    fn load_str(&self, text: &str, base_dir: &Path) -> Result<peregrin_core::Campaign, LoadError>;
}
