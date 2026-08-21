use ac_module_resolver_core::{ModuleCache, ResolvedModule};

/// No-op cache — every `get` returns `None`.
///
/// Use when caching is not desired (e.g., tests that need fresh I/O
/// on every resolve, or memory-constrained environments).
pub struct NoCache;

impl ModuleCache for NoCache {
    fn get(&self, _key: &str) -> Option<ResolvedModule> {
        None
    }

    fn insert(&self, _key: &str, _module: ResolvedModule) {}

    fn clear(&self) {}

    fn len(&self) -> usize {
        0
    }
}
