use std::path::PathBuf;

use ac_fields::FunctionSignature;
use serde_json::Value as JsonValue;

/// A module resolver — abstracts how import paths are loaded.
///
/// Implementations:
/// - `ac-resolver-file` — filesystem-based (`std::fs`)
/// - `ac-resolver-virtual` — in-memory (`HashMap`)
/// - `ac-resolver-http` — HTTP(S) via `ureq`
/// - `ac-resolver-git` — Git repositories via `git2`
pub trait ModuleResolver: Send + Sync {
    /// Resolve a module path to its extracted fields + data.
    ///
    /// `base` is the base path of the importing document (for relative
    /// resolution). Returns `None` if the module cannot be found.
    fn resolve(&self, path: &str, base: Option<&std::path::Path>) -> Option<ResolvedModule>;
}

/// A resolved module — the result of a `ModuleResolver::resolve` call.
///
/// Contains everything needed to merge imported schemas into a parent
/// context and recurse into nested imports.
#[derive(Clone)]
pub struct ResolvedModule {
    /// Field name → raw schema fragment (AC keys still present).
    pub schemas: Vec<(String, JsonValue)>,
    /// Field names marked with `merge: true`.
    pub merge: Vec<String>,
    /// Function signatures declared in the module.
    pub functions: Vec<(String, FunctionSignature)>,
    /// The document's data values (for data-driven import resolution).
    pub data: JsonValue,
    /// Base path for resolving nested imports from this module.
    pub base_path: Option<PathBuf>,
}
