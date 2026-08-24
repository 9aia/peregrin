use std::collections::HashMap;
use std::sync::Mutex;

use ac_module_resolver_core::{ModuleCache, ResolvedModule};

/// In-memory cache backed by a `Mutex<HashMap>`.
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