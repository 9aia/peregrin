use std::collections::HashMap;
use std::sync::Mutex;

use crate::module_cache::ModuleCache;
use crate::resolved_module::ResolvedModule;

/// A simple in-memory cache for testing.
pub struct TestCache {
    map: Mutex<HashMap<String, ResolvedModule>>}

impl TestCache {
    pub fn new() -> Self {
        TestCache {
            map: Mutex::new(HashMap::new())}
    }
}

impl ModuleCache for TestCache {
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
