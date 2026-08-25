//! Type-checking helper host functions.

use handl_vm::{Context, Function};
use handl_vm_values_json::{Error, FunctionValue, NonFunctionValue};

use serde_json::Value as JsonValue;

/// Host function `type_kind(type_value)` → `string`.
///
/// Extracts the `kind` field from a JSON-encoded type.
#[derive(Clone, Copy)]
pub struct TypeKind;

impl Function<NonFunctionValue> for TypeKind {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        if args.is_empty() {
            return Err(Error::ArgumentCount {
                expected: 1,
                got: 0,
            });
        }
        let kind = args[0]
            .inner()
            .as_object()
            .and_then(|o| o.get("kind"))
            .and_then(|k| k.as_str())
            .unwrap_or("");
        Ok(NonFunctionValue::new(JsonValue::String(kind.to_string())))
    }
}

/// Host function `type_name(type_value)` → `string`.
///
/// Extracts the `name` field from a JSON-encoded type (for Var/Trait).
#[derive(Clone, Copy)]
pub struct TypeName;

impl Function<NonFunctionValue> for TypeName {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        if args.is_empty() {
            return Err(Error::ArgumentCount {
                expected: 1,
                got: 0,
            });
        }
        let name = args[0]
            .inner()
            .as_object()
            .and_then(|o| o.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("");
        Ok(NonFunctionValue::new(JsonValue::String(name.to_string())))
    }
}

/// Host function `pair_second(pair)` → `value`.
///
/// Extracts the second element from a 2-element array (for function params).
#[derive(Clone, Copy)]
pub struct PairSecond;

impl Function<NonFunctionValue> for PairSecond {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        if args.is_empty() {
            return Err(Error::ArgumentCount {
                expected: 1,
                got: 0,
            });
        }
        let val = args[0]
            .inner()
            .as_array()
            .and_then(|a| a.get(1))
            .cloned()
            .unwrap_or(JsonValue::Null);
        Ok(NonFunctionValue::new(val))
    }
}

/// Host function `zip_all(a, b, fn)` → `bool`.
///
/// Zips two arrays and checks that fn(a_item, b_item) is true for all pairs.
/// Returns false early if arrays have different lengths.
#[derive(Clone, Copy)]
pub struct ZipAll;

impl Function<NonFunctionValue> for ZipAll {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 3 {
            return Err(Error::ArgumentCount {
                expected: 3,
                got: args.len(),
            });
        }
        let a = args[0].as_array().ok_or(Error::TypeMismatch {
            expected: "array",
            got: "non-array",
        })?;
        let b = args[1].as_array().ok_or(Error::TypeMismatch {
            expected: "array",
            got: "non-array",
        })?;
        let fn_val = FunctionValue::from_json(args[2].inner()).ok_or(Error::TypeMismatch {
            expected: "function value",
            got: "non-function",
        })?;

        if a.len() != b.len() {
            return Ok(NonFunctionValue::new(false));
        }

        for (a_item, b_item) in a.iter().zip(b.iter()) {
            let vars: Vec<(String, NonFunctionValue)> = fn_val
                .params
                .iter()
                .zip([a_item, b_item])
                .map(|((name, _), val)| (name.clone(), NonFunctionValue::new(val.clone())))
                .collect();
            let result = ctx.eval_expr(&fn_val.body, &vars)?;
            let is_true = result.as_bool().unwrap_or(false);
            if !is_true {
                return Ok(NonFunctionValue::new(false));
            }
        }
        Ok(NonFunctionValue::new(true))
    }
}

/// Host function `obj_every(actual_props, expected_props, fn)` → `bool`.
///
/// For each key in `expected_props`, checks that `actual_props` has the key
/// and `fn(actual_type, expected_type)` returns true.
#[derive(Clone, Copy)]
pub struct ObjEvery;

impl Function<NonFunctionValue> for ObjEvery {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 3 {
            return Err(Error::ArgumentCount {
                expected: 3,
                got: args.len(),
            });
        }
        let a_props = args[0].as_object().ok_or(Error::TypeMismatch {
            expected: "object",
            got: "non-object",
        })?;
        let e_props = args[1].as_object().ok_or(Error::TypeMismatch {
            expected: "object",
            got: "non-object",
        })?;
        let fn_val = FunctionValue::from_json(args[2].inner()).ok_or(Error::TypeMismatch {
            expected: "function value",
            got: "non-function",
        })?;

        for (key, e_ty) in e_props.iter() {
            let Some(a_ty) = a_props.get(key) else {
                return Ok(NonFunctionValue::new(false));
            };
            let vars: Vec<(String, NonFunctionValue)> = fn_val
                .params
                .iter()
                .zip([a_ty, e_ty])
                .map(|((name, _), val)| (name.clone(), NonFunctionValue::new(val.clone())))
                .collect();
            let result = ctx.eval_expr(&fn_val.body, &vars)?;
            let is_true = result.as_bool().unwrap_or(false);
            if !is_true {
                return Ok(NonFunctionValue::new(false));
            }
        }
        Ok(NonFunctionValue::new(true))
    }
}

/// Host function `func_params_compatible(actual_params, expected_params, fn)` → `bool`.
///
/// Checks that both param arrays have the same length, then for each pair,
/// extracts the type (second element) and calls `fn(expected_type, actual_type)`
/// — contravariant parameter compatibility.
#[derive(Clone, Copy)]
pub struct FuncParamsCompatible;

impl Function<NonFunctionValue> for FuncParamsCompatible {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 3 {
            return Err(Error::ArgumentCount {
                expected: 3,
                got: args.len(),
            });
        }
        let a_params = args[0].as_array().ok_or(Error::TypeMismatch {
            expected: "array",
            got: "non-array",
        })?;
        let e_params = args[1].as_array().ok_or(Error::TypeMismatch {
            expected: "array",
            got: "non-array",
        })?;
        let fn_val = FunctionValue::from_json(args[2].inner()).ok_or(Error::TypeMismatch {
            expected: "function value",
            got: "non-function",
        })?;

        if a_params.len() != e_params.len() {
            return Ok(NonFunctionValue::new(false));
        }

        for (a_pair, e_pair) in a_params.iter().zip(e_params.iter()) {
            let a_ty = a_pair
                .as_array()
                .and_then(|a| a.get(1))
                .unwrap_or(&JsonValue::Null);
            let e_ty = e_pair
                .as_array()
                .and_then(|a| a.get(1))
                .unwrap_or(&JsonValue::Null);
            // Contravariant: is_compatible(expected_type, actual_type)
            let vars: Vec<(String, NonFunctionValue)> = fn_val
                .params
                .iter()
                .zip([e_ty, a_ty])
                .map(|((name, _), val)| (name.clone(), NonFunctionValue::new(val.clone())))
                .collect();
            let result = ctx.eval_expr(&fn_val.body, &vars)?;
            let is_true = result.as_bool().unwrap_or(false);
            if !is_true {
                return Ok(NonFunctionValue::new(false));
            }
        }
        Ok(NonFunctionValue::new(true))
    }
}

/// Host function `app_args_compatible(actual_args, expected_args, fn)` → `bool`.
///
/// Checks that both arg arrays have the same length, then for each pair,
/// calls `fn(actual_arg, expected_arg)` — covariant.
#[derive(Clone, Copy)]
pub struct AppArgsCompatible;

impl Function<NonFunctionValue> for AppArgsCompatible {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 3 {
            return Err(Error::ArgumentCount {
                expected: 3,
                got: args.len(),
            });
        }
        let a_args = args[0].as_array().ok_or(Error::TypeMismatch {
            expected: "array",
            got: "non-array",
        })?;
        let e_args = args[1].as_array().ok_or(Error::TypeMismatch {
            expected: "array",
            got: "non-array",
        })?;
        let fn_val = FunctionValue::from_json(args[2].inner()).ok_or(Error::TypeMismatch {
            expected: "function value",
            got: "non-function",
        })?;

        if a_args.len() != e_args.len() {
            return Ok(NonFunctionValue::new(false));
        }

        for (a_item, e_item) in a_args.iter().zip(e_args.iter()) {
            let vars: Vec<(String, NonFunctionValue)> = fn_val
                .params
                .iter()
                .zip([a_item, e_item])
                .map(|((name, _), val)| (name.clone(), NonFunctionValue::new(val.clone())))
                .collect();
            let result = ctx.eval_expr(&fn_val.body, &vars)?;
            let is_true = result.as_bool().unwrap_or(false);
            if !is_true {
                return Ok(NonFunctionValue::new(false));
            }
        }
        Ok(NonFunctionValue::new(true))
    }
}
