use ac_runtime::NonFunctionValue;
use ac_vm::{Context, Error, Function};

use super::common::{
    bind_param, is_truthy_value, obj_value, require_function_value, require_object,
    require_one_arg, require_two_args, value_to_string,
};

// ── map_values ────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct MapValues;
impl Function for MapValues {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let (fn_arg, obj_arg) = require_two_args(args)?;
        let fn_val = require_function_value(fn_arg)?;
        let obj = require_object(obj_arg)?;
        let mut result = serde_json::Map::new();
        for (k, v) in obj {
            let vars = bind_param(&fn_val, v);
            let new_value = ctx.eval_expr(&fn_val.body, &vars)?;
            result.insert(k.clone(), new_value.into_inner());
        }
        Ok(obj_value(result))
    }
}

// ── map_keys ──────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct MapKeys;
impl Function for MapKeys {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let (fn_arg, obj_arg) = require_two_args(args)?;
        let fn_val = require_function_value(fn_arg)?;
        let obj = require_object(obj_arg)?;
        let mut result = serde_json::Map::new();
        for (k, v) in obj {
            let key_value = serde_json::Value::String(k.clone());
            let vars = bind_param(&fn_val, &key_value);
            let new_key = ctx.eval_expr(&fn_val.body, &vars)?;
            let new_key_str = new_key.as_str().ok_or(Error::TypeMismatch {
                expected: "string key from function",
                got: "non-string",
            })?;
            result.insert(new_key_str.to_string(), v.clone());
        }
        Ok(obj_value(result))
    }
}

// ── filter_keys ───────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct FilterKeys;
impl Function for FilterKeys {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let (fn_arg, obj_arg) = require_two_args(args)?;
        let fn_val = require_function_value(fn_arg)?;
        let obj = require_object(obj_arg)?;
        let mut result = serde_json::Map::new();
        for (k, v) in obj {
            let key_value = serde_json::Value::String(k.clone());
            let vars = bind_param(&fn_val, &key_value);
            let keep = ctx.eval_expr(&fn_val.body, &vars)?;
            if is_truthy_value(&keep) {
                result.insert(k.clone(), v.clone());
            }
        }
        Ok(obj_value(result))
    }
}

// ── filter_values ─────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct FilterValues;
impl Function for FilterValues {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let (fn_arg, obj_arg) = require_two_args(args)?;
        let fn_val = require_function_value(fn_arg)?;
        let obj = require_object(obj_arg)?;
        let mut result = serde_json::Map::new();
        for (k, v) in obj {
            let vars = bind_param(&fn_val, v);
            let keep = ctx.eval_expr(&fn_val.body, &vars)?;
            if is_truthy_value(&keep) {
                result.insert(k.clone(), v.clone());
            }
        }
        Ok(obj_value(result))
    }
}

// ── invert ────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Invert;
impl Function for Invert {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let arg = require_one_arg(args)?;
        let obj = require_object(arg)?;
        let mut result = serde_json::Map::new();
        for (k, v) in obj {
            let new_key = value_to_string(v);
            result.insert(new_key, serde_json::Value::String(k.clone()));
        }
        Ok(obj_value(result))
    }
}

// ── clone_deep ────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct CloneDeep;
impl Function for CloneDeep {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let arg = require_one_arg(args)?;
        Ok(NonFunctionValue::new(arg.inner().clone()))
    }
}
