//! # ac-lib-kv-hashmap
//!
//! In-memory [`HashMap`]-backed [`KeyValueStore`] implementation.
//!
//! Provides [`HashKeyValueStore`] — a thread-safe, in-memory key-value store
//! suitable for testing, development, and ephemeral workloads.
//!
//! ## Usage
//!
//! ```ignore
//! use ac_runtime_core::ModuleHost;
//! use ac_lib_kv::{KeyValueStoreModule};
//! use ac_lib_kv_hashmap::HashKeyValueStore;
//!
//! let host = ModuleHost::new()
//!     .with(KeyValueStoreModule::new(HashKeyValueStore::default()));
//! ```

use std::collections::HashMap;
use std::sync::Mutex;

use ac_lib_kv::{KeyValueStore, KvError};

/// In-memory key-value store backed by [`HashMap`].
///
/// Thread-safe via `Mutex`. For persistent storage, consider a future
/// sled or SQLite backend.
#[derive(Default)]
pub struct HashKeyValueStore {
    data: Mutex<HashMap<String, String>>,
}

impl HashKeyValueStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with(key: impl Into<String>, value: impl Into<String>) -> Self {
        let mut data = HashMap::new();
        data.insert(key.into(), value.into());
        HashKeyValueStore {
            data: Mutex::new(data),
        }
    }
}

impl KeyValueStore for HashKeyValueStore {
    fn get(&self, key: &str) -> Result<Option<String>, KvError> {
        Ok(self.data.lock().unwrap().get(key).cloned())
    }

    fn set(&self, key: &str, value: &str) -> Result<(), KvError> {
        self.data.lock().unwrap().insert(key.into(), value.into());
        Ok(())
    }

    fn remove(&self, key: &str) -> Result<(), KvError> {
        self.data.lock().unwrap().remove(key);
        Ok(())
    }

    fn clear(&self) -> Result<(), KvError> {
        self.data.lock().unwrap().clear();
        Ok(())
    }

    fn has(&self, key: &str) -> Result<bool, KvError> {
        Ok(self.data.lock().unwrap().contains_key(key))
    }

    fn size(&self) -> Result<usize, KvError> {
        Ok(self.data.lock().unwrap().len())
    }
}

#[cfg(test)]
mod tests {
    use ac_runtime::RuntimeValue;
    use ac_runtime_core::Module;
    use ac_lib_kv::KeyValueStoreModule;
    use ac_vm::Context;

    use super::HashKeyValueStore;

    fn call(module: &dyn Module, name: &str, args: &[RuntimeValue]) -> RuntimeValue {
        let func = module.function(name).expect("function exists");
        let mut ctx = Context::new();
        func.call(args, &mut ctx).expect("call succeeds")
    }

    fn s(v: &str) -> RuntimeValue {
        RuntimeValue::new(v.to_string())
    }

    #[test]
    fn set_and_get() {
        let module = KeyValueStoreModule::new(HashKeyValueStore::default());
        call(&module, "set", &[s("key"), s("value")]);
        assert_eq!(call(&module, "get", &[s("key")]).as_str(), Some("value"));
    }

    #[test]
    fn get_missing_returns_null() {
        let module = KeyValueStoreModule::new(HashKeyValueStore::default());
        assert!(call(&module, "get", &[s("nope")]).is_null());
    }

    #[test]
    fn has() {
        let store = HashKeyValueStore::with("k", "v");
        let module = KeyValueStoreModule::new(store);
        assert_eq!(call(&module, "has", &[s("k")]).as_bool(), Some(true));
        assert_eq!(call(&module, "has", &[s("x")]).as_bool(), Some(false));
    }

    #[test]
    fn remove() {
        let store = HashKeyValueStore::with("k", "v");
        let module = KeyValueStoreModule::new(store);
        call(&module, "remove", &[s("k")]);
        assert_eq!(call(&module, "has", &[s("k")]).as_bool(), Some(false));
    }

    #[test]
    fn clear() {
        let store = HashKeyValueStore::with("a", "1");
        let module = KeyValueStoreModule::new(store);
        call(&module, "clear", &[]);
        assert_eq!(call(&module, "size", &[]).as_number(), Some(0.0));
    }

    #[test]
    fn size() {
        let store = HashKeyValueStore::with("a", "1");
        let module = KeyValueStoreModule::new(store);
        assert_eq!(call(&module, "size", &[]).as_number(), Some(1.0));
    }
}
