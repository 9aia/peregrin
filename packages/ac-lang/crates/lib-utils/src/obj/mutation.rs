use ac_runtime::{Context, Error, Function};
use ac_runtime::{FunctionValue, NonFunctionValue};

use super::common::{
    obj_value, require_function_value, require_object, require_string, require_three_args,
    set_path_recursive, update_path_recursive,
};
use crate::shared::apply_fn;

// ── set ───────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Set;
impl Function for Set {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let (obj_arg, key_arg, value_arg) = require_three_args(args)?;
        let obj = require_object(obj_arg)?;
        let key = require_string(key_arg)?;
        let mut result = obj.clone();
        result.insert(key, value_arg.inner().clone());
        Ok(obj_value(result))
    }
}

// ── update ────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Update;
impl Function for Update {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let (obj_arg, key_arg, fn_arg) = require_three_args(args)?;
        let obj = require_object(obj_arg)?;
        let key = require_string(key_arg)?;
        let fn_val = require_function_value(fn_arg)?;
        let current = obj.get(&key).cloned().unwrap_or(serde_json::Value::Null);
        let new_value = apply_fn(ctx, &fn_val, &current)?;
        let mut result = obj.clone();
        result.insert(key, new_value.into_inner());
        Ok(obj_value(result))
    }
}

// ── set_path ──────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct SetPath;
impl Function for SetPath {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let (obj_arg, path_arg, value_arg) = require_three_args(args)?;
        let path = require_string(path_arg)?;
        let segments: Vec<&str> = path.split('.').filter(|s| !s.is_empty()).collect();
        let result = set_path_recursive(
            obj_arg.inner().clone(),
            &segments,
            value_arg.inner().clone(),
        );
        Ok(NonFunctionValue::new(result))
    }
}

// ── update_path ───────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct UpdatePath;
impl Function for UpdatePath {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let (obj_arg, path_arg, fn_arg) = require_three_args(args)?;
        let path = require_string(path_arg)?;
        let fn_val: FunctionValue = require_function_value(fn_arg)?;
        let segments: Vec<&str> = path.split('.').filter(|s| !s.is_empty()).collect();
        let result = update_path_recursive(obj_arg.inner().clone(), &segments, &fn_val, ctx)?;
        Ok(NonFunctionValue::new(result))
    }
}
