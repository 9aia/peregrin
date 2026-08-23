use ac_runtime::NonFunctionValue;
use ac_runtime::{Context, Error, Function};

use super::common::{arr, num, require_object, require_one_arg};

// ── keys ──────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Keys;
impl Function for Keys {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let arg = require_one_arg(args)?;
        let obj = require_object(arg)?;
        let keys: Vec<serde_json::Value> = obj
            .keys()
            .map(|k| serde_json::Value::String(k.clone()))
            .collect();
        Ok(arr(&keys))
    }
}

// ── values ────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Values;
impl Function for Values {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let arg = require_one_arg(args)?;
        let obj = require_object(arg)?;
        let values: Vec<serde_json::Value> = obj.values().cloned().collect();
        Ok(arr(&values))
    }
}

// ── entries ───────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Entries;
impl Function for Entries {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let arg = require_one_arg(args)?;
        let obj = require_object(arg)?;
        let entries: Vec<serde_json::Value> = obj
            .iter()
            .map(|(k, v)| serde_json::json!([serde_json::Value::String(k.clone()), v.clone()]))
            .collect();
        Ok(arr(&entries))
    }
}

// ── size ──────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Size;
impl Function for Size {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let arg = require_one_arg(args)?;
        let size = if arg.is_null() {
            0.0
        } else if let Some(arr) = arg.as_array() {
            arr.len() as f64
        } else if let Some(obj) = arg.as_object() {
            obj.len() as f64
        } else if let Some(s) = arg.as_str() {
            s.chars().count() as f64
        } else {
            return Err(Error::TypeMismatch {
                expected: "array, object, string, or null",
                got: "non-sized type",
            });
        };
        Ok(num(size))
    }
}
