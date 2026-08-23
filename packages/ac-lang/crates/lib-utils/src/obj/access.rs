use ac_runtime::NonFunctionValue;
use ac_runtime::{Context, Error, Function};

use super::common::{b, require_object, require_string, require_two_args};

// ── get ───────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Get;
impl Function for Get {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let (obj_arg, key_arg) = require_two_args(args)?;
        let obj = require_object(obj_arg)?;
        let key = require_string(key_arg)?;
        match obj.get(&key) {
            Some(v) => Ok(NonFunctionValue::new(v.clone())),
            None => Ok(NonFunctionValue::new(serde_json::Value::Null))}
    }
}

// ── get_or ────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct GetOr;
impl Function for GetOr {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let (obj_arg, key_arg, default_arg) = super::common::require_three_args(args)?;
        let obj = require_object(obj_arg)?;
        let key = require_string(key_arg)?;
        match obj.get(&key) {
            Some(serde_json::Value::Null) | None => Ok(default_arg.clone()),
            Some(v) => Ok(NonFunctionValue::new(v.clone()))}
    }
}

// ── has_key ───────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct HasKey;
impl Function for HasKey {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let (obj_arg, key_arg) = require_two_args(args)?;
        let obj = require_object(obj_arg)?;
        let key = require_string(key_arg)?;
        Ok(b(obj.contains_key(&key)))
    }
}

// ── deep_get ──────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct DeepGet;
impl Function for DeepGet {
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
        let path = args[1].as_str().ok_or(Error::TypeMismatch {
            expected: "string",
            got: "non-string"})?;
        let mut current = args[0].inner().clone();
        for segment in path.split('.') {
            if segment.is_empty() {
                continue;
            }
            current = match &current {
                serde_json::Value::Object(obj) => {
                    obj.get(segment).cloned().unwrap_or(serde_json::Value::Null)
                }
                serde_json::Value::Array(arr) => {
                    if let Ok(idx) = segment.parse::<usize>() {
                        arr.get(idx).cloned().unwrap_or(serde_json::Value::Null)
                    } else {
                        serde_json::Value::Null
                    }
                }
                _ => serde_json::Value::Null};
            if current.is_null() {
                break;
            }
        }
        Ok(NonFunctionValue::new(current))
    }
}

// ── path_or ───────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct PathOr;
impl Function for PathOr {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let (value_arg, path_arg, default_arg) = super::common::require_three_args(args)?;
        let path = require_string(path_arg)?;
        let mut current = value_arg.inner().clone();
        for segment in path.split('.') {
            if segment.is_empty() {
                continue;
            }
            current = match &current {
                serde_json::Value::Object(obj) => {
                    obj.get(segment).cloned().unwrap_or(serde_json::Value::Null)
                }
                serde_json::Value::Array(arr) => {
                    if let Ok(idx) = segment.parse::<usize>() {
                        arr.get(idx).cloned().unwrap_or(serde_json::Value::Null)
                    } else {
                        serde_json::Value::Null
                    }
                }
                _ => serde_json::Value::Null};
            if current.is_null() {
                break;
            }
        }
        if current.is_null() {
            Ok(default_arg.clone())
        } else {
            Ok(NonFunctionValue::new(current))
        }
    }
}
