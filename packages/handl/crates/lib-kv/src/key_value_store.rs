use handl_vm_values_json::NonFunctionValue;
use std::sync::Arc;

use handl_vm_common::Module;
use handl_vm_values_json::Error;
use handl_vm_values_json::{Context, Function};

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
            get: GetFn {
                store: store.clone(),
            },
            set: SetFn {
                store: store.clone(),
            },
            remove: RemoveFn {
                store: store.clone(),
            },
            clear: ClearFn {
                store: store.clone(),
            },
            has: HasFn {
                store: store.clone(),
            },
            size: SizeFn { store },
        }
    }
}

impl Module for KeyValueStoreModule {
    fn name(&self) -> &str {
        "kv"
    }

    fn function(&self, name: &str) -> Option<&dyn Function<NonFunctionValue>> {
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

fn require_one_string(args: &[NonFunctionValue]) -> Result<&str, Error> {
    let arg = args.first().ok_or(Error::ArgumentCount {
        expected: 1,
        got: 0,
    })?;
    arg.as_str().ok_or(Error::TypeMismatch {
        expected: "string",
        got: "non-string",
    })
}

fn require_two_strings(args: &[NonFunctionValue]) -> Result<(&str, &str), Error> {
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

impl Function<NonFunctionValue> for GetFn {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let key = require_one_string(args)?;
        let value = self.store.get(key)?;
        Ok(match value {
            Some(v) => NonFunctionValue::new(v),
            None => NonFunctionValue::default(),
        })
    }
}

struct SetFn {
    store: Arc<dyn KeyValueStore>,
}

impl Function<NonFunctionValue> for SetFn {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let (key, value) = require_two_strings(args)?;
        self.store.set(key, value)?;
        Ok(NonFunctionValue::default())
    }
}

struct RemoveFn {
    store: Arc<dyn KeyValueStore>,
}

impl Function<NonFunctionValue> for RemoveFn {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let key = require_one_string(args)?;
        self.store.remove(key)?;
        Ok(NonFunctionValue::default())
    }
}

struct ClearFn {
    store: Arc<dyn KeyValueStore>,
}

impl Function<NonFunctionValue> for ClearFn {
    fn call(
        &self,
        _args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        self.store.clear()?;
        Ok(NonFunctionValue::default())
    }
}

struct HasFn {
    store: Arc<dyn KeyValueStore>,
}

impl Function<NonFunctionValue> for HasFn {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let key = require_one_string(args)?;
        self.store
            .has(key)
            .map(NonFunctionValue::new)
            .map_err(Into::into)
    }
}

struct SizeFn {
    store: Arc<dyn KeyValueStore>,
}

impl Function<NonFunctionValue> for SizeFn {
    fn call(
        &self,
        _args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        self.store
            .size()
            .map(|n| NonFunctionValue::new(serde_json::json!(n)))
            .map_err(Into::into)
    }
}
