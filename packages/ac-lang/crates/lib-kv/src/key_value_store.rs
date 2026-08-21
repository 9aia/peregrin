use std::sync::Arc;

use ac_runtime::RuntimeValue;
use ac_runtime_core::Module;
use ac_vm::{Context, Error, Function};

use crate::error::KvError;

/// Abstract key-value store service.
///
/// Implementations:
/// - [`crate::StubKeyValueStore`] — in-memory `HashMap` for testing
/// - Future backends: sled, SQLite, Redis, etc.
pub trait KeyValueStore: Send + Sync {
    fn get(&self, key: &str) -> Result<Option<String>, KvError>;
    fn set(&self, key: &str, value: &str) -> Result<(), KvError>;
    fn remove(&self, key: &str) -> Result<(), KvError>;
    fn clear(&self) -> Result<(), KvError>;
    fn has(&self, key: &str) -> Result<bool, KvError>;
    fn size(&self) -> Result<usize, KvError>;
}

/// Module adapter for [`KeyValueStore`] — exposes `kv.*` functions.
pub struct KeyValueStoreModule {
    get: GetFn,
    set: SetFn,
    remove: RemoveFn,
    clear: ClearFn,
    has: HasFn,
    size: SizeFn,
}

impl KeyValueStoreModule {
    pub fn new(store: impl KeyValueStore + 'static) -> Self {
        let store = Arc::new(store);
        KeyValueStoreModule {
            get: GetFn { store: store.clone() },
            set: SetFn { store: store.clone() },
            remove: RemoveFn { store: store.clone() },
            clear: ClearFn { store: store.clone() },
            has: HasFn { store: store.clone() },
            size: SizeFn { store },
        }
    }
}

impl Module for KeyValueStoreModule {
    fn name(&self) -> &str {
        "kv"
    }

    fn function(&self, name: &str) -> Option<&dyn Function> {
        match name {
            "get" => Some(&self.get),
            "set" => Some(&self.set),
            "remove" => Some(&self.remove),
            "clear" => Some(&self.clear),
            "has" => Some(&self.has),
            "size" => Some(&self.size),
            _ => None,
        }
    }
}

fn require_one_string(args: &[RuntimeValue]) -> Result<&str, Error> {
    let arg = args.first().ok_or(Error::ArgumentCount {
        expected: 1,
        got: 0,
    })?;
    arg.as_str().ok_or(Error::TypeMismatch {
        expected: "string",
        got: "non-string",
    })
}

fn require_two_strings(args: &[RuntimeValue]) -> Result<(&str, &str), Error> {
    if args.len() < 2 {
        return Err(Error::ArgumentCount {
            expected: 2,
            got: args.len(),
        });
    }
    let key = args[0].as_str().ok_or(Error::TypeMismatch {
        expected: "string",
        got: "non-string",
    })?;
    let value = args[1].as_str().ok_or(Error::TypeMismatch {
        expected: "string",
        got: "non-string",
    })?;
    Ok((key, value))
}

struct GetFn {
    store: Arc<dyn KeyValueStore>,
}

impl Function for GetFn {
    fn call(&self, args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
        let key = require_one_string(args)?;
        let value = self.store.get(key)?;
        Ok(match value {
            Some(v) => RuntimeValue::new(v),
            None => RuntimeValue::default(),
        })
    }
}

struct SetFn {
    store: Arc<dyn KeyValueStore>,
}

impl Function for SetFn {
    fn call(&self, args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
        let (key, value) = require_two_strings(args)?;
        self.store.set(key, value)?;
        Ok(RuntimeValue::default())
    }
}

struct RemoveFn {
    store: Arc<dyn KeyValueStore>,
}

impl Function for RemoveFn {
    fn call(&self, args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
        let key = require_one_string(args)?;
        self.store.remove(key)?;
        Ok(RuntimeValue::default())
    }
}

struct ClearFn {
    store: Arc<dyn KeyValueStore>,
}

impl Function for ClearFn {
    fn call(&self, _args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
        self.store.clear()?;
        Ok(RuntimeValue::default())
    }
}

struct HasFn {
    store: Arc<dyn KeyValueStore>,
}

impl Function for HasFn {
    fn call(&self, args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
        let key = require_one_string(args)?;
        self.store.has(key).map(RuntimeValue::new).map_err(Into::into)
    }
}

struct SizeFn {
    store: Arc<dyn KeyValueStore>,
}

impl Function for SizeFn {
    fn call(&self, _args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
        self.store
            .size()
            .map(|n| RuntimeValue::new(serde_json::json!(n)))
            .map_err(Into::into)
    }
}
