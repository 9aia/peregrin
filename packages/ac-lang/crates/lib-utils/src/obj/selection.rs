use ac_runtime::NonFunctionValue;
use ac_vm::{Context, Error, Function};

use super::common::{require_array, require_object};

// ── pick ──────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Pick;
impl Function for Pick {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        let obj = require_object(&args[0])?;
        let keys = require_array(&args[1])?;
        let mut result = serde_json::Map::new();
        for key_val in keys {
            if let Some(key) = key_val.as_str()
                && let Some(v) = obj.get(key)
            {
                result.insert(key.to_string(), v.clone());
            }
        }
        Ok(NonFunctionValue::new(serde_json::Value::Object(result)))
    }
}

// ── omit ──────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Omit;
impl Function for Omit {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        let obj = require_object(&args[0])?;
        let keys = require_array(&args[1])?;
        let omit_set: std::collections::HashSet<&str> =
            keys.iter().filter_map(|v| v.as_str()).collect();
        let mut result = serde_json::Map::new();
        for (k, v) in obj {
            if !omit_set.contains(k.as_str()) {
                result.insert(k.clone(), v.clone());
            }
        }
        Ok(NonFunctionValue::new(serde_json::Value::Object(result)))
    }
}
