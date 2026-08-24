use crate::resolved_module::ResolvedModule;

/// A module resolver — abstracts how import paths are loaded.
///
/// Implementations:
/// - `ac-module-resolver-file` — filesystem-based (`std::fs`)
/// - `ac-module-resolver-virtual` — in-memory (`HashMap`)
/// - `ac-module-resolver-http` — HTTP(S) via `ureq`
/// - `ac-module-resolver-git` — Git repositories via `git2`
pub trait ModuleResolver: Send + Sync {
    /// Resolve a module path to its extracted fields + data.
    ///
    /// `base` is the base path of the importing document (for relative
    /// resolution). Returns `None` if the module cannot be found.
    fn resolve(&self, path: &str, base: Option<&std::path::Path>) -> Option<ResolvedModule>;
}