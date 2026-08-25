use handl_vm_values_json::NonFunctionValue;
use std::sync::Arc;

use handl_vm::{Context, Function};
use handl_vm_common::Module;
use handl_vm_values_json::Error;

use crate::error::OsError;

/// Abstract environment variable service.
///
/// Implementations:
/// - `handl-lib-os-std` — backed by `std::env`
/// - [`env_stub::StubEnv`] — in-memory for testing
pub trait Env: Send + Sync {
    fn get(&self, key: &str) -> Result<Option<String>, OsError>;
    fn has(&self, key: &str) -> Result<bool, OsError>;
    fn keys(&self) -> Result<Vec<String>, OsError>;
}

/// Module adapter for [`Env`] — exposes `env.*` functions.
pub struct EnvModule {
    get: GetFn,
    has: HasFn,
    keys: KeysFn,
}

impl EnvModule {
    pub fn new(env: impl Env + 'static) -> Self {
        let env = Arc::new(env);
        EnvModule {
            get: GetFn { env: env.clone() },
            has: HasFn { env: env.clone() },
            keys: KeysFn { env },
        }
    }
}

impl Module for EnvModule {
    fn name(&self) -> &str {
        "env"
    }

    fn function(&self, name: &str) -> Option<&dyn Function<NonFunctionValue>> {
        match name {
            "get" => Some(&self.get),
            "has" => Some(&self.has),
            "keys" => Some(&self.keys),
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

struct GetFn {
    env: Arc<dyn Env>,
}

impl Function<NonFunctionValue> for GetFn {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let key = require_one_string(args)?;
        let value = self.env.get(key)?;
        Ok(match value {
            Some(v) => NonFunctionValue::new(v),
            None => NonFunctionValue::default(),
        })
    }
}

struct HasFn {
    env: Arc<dyn Env>,
}

impl Function<NonFunctionValue> for HasFn {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let key = require_one_string(args)?;
        self.env
            .has(key)
            .map(NonFunctionValue::new)
            .map_err(Into::into)
    }
}

struct KeysFn {
    env: Arc<dyn Env>,
}

impl Function<NonFunctionValue> for KeysFn {
    fn call(
        &self,
        _args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let keys = self.env.keys()?;
        let arr: Vec<serde_json::Value> = keys.into_iter().map(serde_json::Value::String).collect();
        Ok(NonFunctionValue::new(serde_json::Value::Array(arr)))
    }
}
