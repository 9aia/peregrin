use std::collections::HashMap;
use std::sync::Mutex;

use crate::resolver::ResolvedModule;

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
pub trait ModuleCache: Send + Sync {
    fn get(&self, key: &str) -> Option<ResolvedModule>;
    fn insert(&self, key: &str, module: ResolvedModule);
    fn clear(&self);
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Default in-memory cache backed by a `Mutex<HashMap>`.
///
/// Thread-safe — suitable for both single-threaded and multi-threaded
/// pipeline usage (e.g. parallel type checking with rayon).
pub struct MemoryModuleCache {
    map: Mutex<HashMap<String, ResolvedModule>>,
}

impl MemoryModuleCache {
    pub fn new() -> Self {
        MemoryModuleCache {
            map: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for MemoryModuleCache {
    fn default() -> Self {
        Self::new()
    }
}

impl ModuleCache for MemoryModuleCache {
    fn get(&self, key: &str) -> Option<ResolvedModule> {
        self.map.lock().unwrap().get(key).cloned()
    }

    fn insert(&self, key: &str, module: ResolvedModule) {
        self.map.lock().unwrap().insert(key.to_string(), module);
    }

    fn clear(&self) {
        self.map.lock().unwrap().clear();
    }

    fn len(&self) -> usize {
        self.map.lock().unwrap().len()
    }
}

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
