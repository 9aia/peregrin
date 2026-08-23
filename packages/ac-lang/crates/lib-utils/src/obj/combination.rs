use ac_runtime::NonFunctionValue;
use ac_runtime::{Context, Error, Function};

use super::common::{deep_merge_values, obj_value, require_array, require_object, require_one_arg};

// ── merge ─────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Merge;
impl Function for Merge {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len()});
        }
        let a = require_object(&args[0])?;
        let b = require_object(&args[1])?;
        let mut result = a.clone();
        for (k, v) in b {
            result.insert(k.clone(), v.clone());
        }
        Ok(NonFunctionValue::new(serde_json::Value::Object(result)))
    }
}

// ── deep_merge ────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct DeepMerge;
impl Function for DeepMerge {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len()});
        }
        let a = require_object(&args[0])?;
        let b = require_object(&args[1])?;
        let merged = deep_merge_values(
            &serde_json::Value::Object(a.clone()),
            &serde_json::Value::Object(b.clone()),
        );
        Ok(NonFunctionValue::new(merged))
    }
}

// ── from_entries ──────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct FromEntries;
impl Function for FromEntries {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let arg = require_one_arg(args)?;
        let entries = require_array(arg)?;
        let mut result = serde_json::Map::new();
        for entry in entries {
            let pair = entry.as_array().ok_or(Error::TypeMismatch {
                expected: "array pair [key, value]",
                got: "non-array"})?;
            if pair.len() < 2 {
                return Err(Error::TypeMismatch {
                    expected: "pair with at least 2 elements",
                    got: "short pair"});
            }
            let key = pair[0].as_str().ok_or(Error::TypeMismatch {
                expected: "string key",
                got: "non-string"})?;
            result.insert(key.to_string(), pair[1].clone());
        }
        Ok(obj_value(result))
    }
}
