use crate::resolved_module::ResolvedModule;

/// Cache for resolved modules, keyed by a resolver-specific string.
///
/// Each resolver decides what to cache and how to key it:
/// - **FileResolver** — key = canonical path
/// - **HttpResolver** — key = full URL
/// - **GitResolver** — key = `repo_url@ref/path` (module-level),
///   plus an internal repo cache for clone-level dedup
///
/// Implementations must be thread-safe (`Send`) since resolvers may be
/// shared across passes (`ResolvePass` + `TypecheckPass`).
///
/// See `ac-module-resolver-memory` for the default in-memory implementation
/// and `ac-module-resolver-noop` for a no-op implementation.
pub trait ModuleCache: Send + Sync {
    fn get(&self, key: &str) -> Option<ResolvedModule>;
    fn insert(&self, key: &str, module: ResolvedModule);
    fn clear(&self);
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
