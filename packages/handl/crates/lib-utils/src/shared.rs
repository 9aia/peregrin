use handl_vm::Context;
use handl_vm_values_json::Error;
use serde_json::Value as JsonValue;

use handl_vm_values_json::{FunctionValue, NonFunctionValue};

// ════════════════════════════════════════════════════════════════════
// Tier 0: Shared helpers (replaces duplicated copies across modules)
// ════════════════════════════════════════════════════════════════════

// ── type requirements ──────────────────────────────────────────────

pub(crate) fn require_array(arg: &NonFunctionValue) -> Result<&[serde_json::Value], Error> {
    arg.as_array().ok_or(Error::TypeMismatch {
        expected: "array",
        got: "non-array",
    })
}

pub(crate) fn require_function_value(arg: &NonFunctionValue) -> Result<FunctionValue, Error> {
    FunctionValue::from_json(arg.inner()).ok_or(Error::TypeMismatch {
        expected: "function value",
        got: "non-function",
    })
}

// ── argument count requirements ────────────────────────────────────

pub(crate) fn require_one_arg(args: &[NonFunctionValue]) -> Result<&NonFunctionValue, Error> {
    args.first().ok_or(Error::ArgumentCount {
        expected: 1,
        got: 0,
    })
}

pub(crate) fn require_one_array(args: &[NonFunctionValue]) -> Result<&[serde_json::Value], Error> {
    let arg = args.first().ok_or(Error::ArgumentCount {
        expected: 1,
        got: 0,
    })?;
    require_array(arg)
}

pub(crate) fn require_two_arrays(
    args: &[NonFunctionValue],
) -> Result<(&[serde_json::Value], &[serde_json::Value]), Error> {
    if args.len() < 2 {
        return Err(Error::ArgumentCount {
            expected: 2,
            got: args.len(),
        });
    }
    let a = require_array(&args[0])?;
    let b = require_array(&args[1])?;
    Ok((a, b))
}

pub(crate) fn require_function_and_array(
    args: &[NonFunctionValue],
) -> Result<(FunctionValue, &[serde_json::Value]), Error> {
    if args.len() < 2 {
        return Err(Error::ArgumentCount {
            expected: 2,
            got: args.len(),
        });
    }
    let fn_val = require_function_value(&args[0])?;
    let array = require_array(&args[1])?;
    Ok((fn_val, array))
}

pub(crate) fn require_functions(args: &[NonFunctionValue]) -> Result<Vec<FunctionValue>, Error> {
    if args.is_empty() {
        return Err(Error::ArgumentCount {
            expected: 1,
            got: 0,
        });
    }
    args.iter().map(require_function_value).collect()
}

// ── param binding ──────────────────────────────────────────────────

pub(crate) fn bind_param(
    fn_val: &FunctionValue,
    value: &serde_json::Value,
) -> Vec<(String, NonFunctionValue)> {
    fn_val
        .params
        .first()
        .map(|(name, _)| vec![(name.clone(), NonFunctionValue::new(value.clone()))])
        .unwrap_or_default()
}

pub(crate) fn bind_two_params(
    fn_val: &FunctionValue,
    acc: &serde_json::Value,
    item: &serde_json::Value,
) -> Vec<(String, NonFunctionValue)> {
    let param_names: Vec<&String> = fn_val.params.iter().map(|(name, _)| name).collect();
    let acc_name = param_names.first().map(|s| s.as_str()).unwrap_or("acc");
    let item_name = param_names.get(1).map(|s| s.as_str()).unwrap_or("x");
    vec![
        (acc_name.to_string(), NonFunctionValue::new(acc.clone())),
        (item_name.to_string(), NonFunctionValue::new(item.clone())),
    ]
}

fn pattern_binding_name(pattern: &JsonValue) -> Option<&str> {
    pattern.get("type").and_then(|v| v.as_str())?;
    pattern.get("name").and_then(|v| v.as_str())
}

fn destructure_pattern(
    pattern: &JsonValue,
    value: &JsonValue,
    out: &mut Vec<(String, JsonValue)>,
) -> Result<(), Error> {
    match pattern.get("type").and_then(|v| v.as_str()) {
        Some("identifier") => {
            if let Some(name) = pattern_binding_name(pattern) {
                out.push((name.to_string(), value.clone()));
            }
        }
        Some("array") => {
            let patterns = pattern
                .get("patterns")
                .and_then(|v| v.as_array())
                .ok_or(Error::PrimitiveError("invalid array pattern".to_string()))?;
            let arr = value.as_array().ok_or(Error::TypeMismatch {
                expected: "array",
                got: "non-array",
            })?;
            for (i, child_pattern) in patterns.iter().enumerate() {
                let child = arr.get(i).unwrap_or(&JsonValue::Null);
                destructure_pattern(child_pattern, child, out)?;
            }
        }
        Some("object") => {
            let obj = value.as_object().ok_or(Error::TypeMismatch {
                expected: "object",
                got: "non-object",
            })?;
            let entries = pattern
                .get("entries")
                .and_then(|v| v.as_array())
                .ok_or(Error::PrimitiveError("invalid object pattern".to_string()))?;
            for entry in entries {
                let arr = entry.as_array().ok_or(Error::PrimitiveError(
                    "invalid object pattern entry".to_string(),
                ))?;
                let key = arr
                    .get(0)
                    .and_then(|v| v.as_str())
                    .ok_or(Error::PrimitiveError(
                        "invalid object pattern key".to_string(),
                    ))?;
                let child_pattern = arr.get(1).ok_or(Error::PrimitiveError(
                    "invalid object pattern entry".to_string(),
                ))?;
                let child = obj.get(key).unwrap_or(&JsonValue::Null);
                destructure_pattern(child_pattern, child, out)?;
            }
        }
        _ => {}
    }
    Ok(())
}

pub(crate) fn bind_function_params(
    fn_val: &FunctionValue,
    args: &[JsonValue],
) -> Result<Vec<(String, NonFunctionValue)>, Error> {
    let mut bindings = Vec::new();

    for (i, (name, _)) in fn_val.params.iter().enumerate() {
        let arg = args.get(i).unwrap_or(&JsonValue::Null);

        if let Some(pattern) = fn_val.patterns.get(i) {
            if pattern.get("type").is_some() {
                let mut raw = Vec::new();
                destructure_pattern(pattern, arg, &mut raw)?;
                bindings.extend(raw.into_iter().map(|(n, v)| (n, NonFunctionValue::new(v))));
                continue;
            }
        }

        bindings.push((name.clone(), NonFunctionValue::new(arg.clone())));
    }

    Ok(bindings)
}

// ── truthiness ─────────────────────────────────────────────────────

pub(crate) fn is_truthy_value(value: &NonFunctionValue) -> bool {
    if let Some(b) = value.as_bool() {
        b
    } else if let Some(n) = value.as_number() {
        n != 0.0
    } else if let Some(s) = value.as_str() {
        !s.is_empty()
    } else if value.is_null() {
        false
    } else if let Some(arr) = value.as_array() {
        !arr.is_empty()
    } else if let Some(obj) = value.as_object() {
        !obj.is_empty()
    } else {
        true
    }
}

// ── constructors ───────────────────────────────────────────────────

pub(crate) fn arr(items: &[serde_json::Value]) -> NonFunctionValue {
    NonFunctionValue::new(serde_json::Value::Array(items.to_vec()))
}

pub(crate) fn b(value: bool) -> NonFunctionValue {
    NonFunctionValue::new(value)
}

pub(crate) fn num(value: f64) -> NonFunctionValue {
    NonFunctionValue::new(serde_json::json!(value))
}

// ── string conversion ──────────────────────────────────────────────

pub(crate) fn key_to_string(value: &NonFunctionValue) -> String {
    if let Some(s) = value.as_str() {
        s.to_string()
    } else if let Some(n) = value.as_number() {
        if n == n.trunc() {
            format!("{n:.0}")
        } else {
            n.to_string()
        }
    } else if let Some(b) = value.as_bool() {
        b.to_string()
    } else if value.is_null() {
        "null".to_string()
    } else {
        serde_json::to_string(&value.clone().into_inner()).unwrap_or_else(|_| "unknown".to_string())
    }
}

pub(crate) fn value_to_string(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => {
            if let Some(f) = n.as_f64() {
                if f == f.trunc() {
                    format!("{f:.0}")
                } else {
                    f.to_string()
                }
            } else {
                n.to_string()
            }
        }
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => "null".to_string(),
        _ => serde_json::to_string(value).unwrap_or_else(|_| "unknown".to_string()),
    }
}

// ════════════════════════════════════════════════════════════════════
// Tier 1: Composable primitives
// ════════════════════════════════════════════════════════════════════

/// Apply a single-param function to a value: bind_param + eval_expr.
pub(crate) fn apply_fn(
    ctx: &mut dyn Context<NonFunctionValue>,
    fn_val: &FunctionValue,
    value: &serde_json::Value,
) -> Result<NonFunctionValue, Error> {
    let vars = bind_param(fn_val, value);
    ctx.eval_expr(&fn_val.body, &vars)
}

/// Apply a two-param function to two values: bind_two_params + eval_expr.
pub(crate) fn apply_fn2(
    ctx: &mut dyn Context<NonFunctionValue>,
    fn_val: &FunctionValue,
    a: &serde_json::Value,
    b: &serde_json::Value,
) -> Result<NonFunctionValue, Error> {
    let vars = bind_two_params(fn_val, a, b);
    ctx.eval_expr(&fn_val.body, &vars)
}

/// Apply a predicate function and check if the result is truthy.
pub(crate) fn is_truthy_fn(
    ctx: &mut dyn Context<NonFunctionValue>,
    fn_val: &FunctionValue,
    value: &serde_json::Value,
) -> Result<bool, Error> {
    let result = apply_fn(ctx, fn_val, value)?;
    Ok(is_truthy_value(&result))
}

/// Map a function over an iterator, collecting results.
/// Works with any iterator yielding `&serde_json::Value`.
pub(crate) fn map_iter<'a, I>(
    ctx: &mut dyn Context<NonFunctionValue>,
    fn_val: &FunctionValue,
    iter: I,
) -> Result<Vec<serde_json::Value>, Error>
where
    I: Iterator<Item = &'a serde_json::Value>,
{
    let mut results = Vec::new();
    for item in iter {
        let mapped = apply_fn(ctx, fn_val, item)?;
        results.push(mapped.into_inner());
    }
    Ok(results)
}

/// Filter an iterator by a predicate function, collecting matching items.
pub(crate) fn filter_iter<'a, I>(
    ctx: &mut dyn Context<NonFunctionValue>,
    fn_val: &FunctionValue,
    iter: I,
) -> Result<Vec<serde_json::Value>, Error>
where
    I: Iterator<Item = &'a serde_json::Value>,
{
    let mut results = Vec::new();
    for item in iter {
        if is_truthy_fn(ctx, fn_val, item)? {
            results.push(item.clone());
        }
    }
    Ok(results)
}

/// Fold an iterator with a two-param function and an initial accumulator.
pub(crate) fn fold_iter<'a, I>(
    ctx: &mut dyn Context<NonFunctionValue>,
    fn_val: &FunctionValue,
    iter: I,
    init: serde_json::Value,
) -> Result<serde_json::Value, Error>
where
    I: Iterator<Item = &'a serde_json::Value>,
{
    let mut acc = init;
    for item in iter {
        let result = apply_fn2(ctx, fn_val, &acc, item)?;
        acc = result.into_inner();
    }
    Ok(acc)
}

/// Find the first element matching a predicate.
/// Returns `None` if no match.
pub(crate) fn find_first<'a, I>(
    ctx: &mut dyn Context<NonFunctionValue>,
    fn_val: &FunctionValue,
    iter: I,
) -> Result<Option<serde_json::Value>, Error>
where
    I: Iterator<Item = &'a serde_json::Value>,
{
    for item in iter {
        if is_truthy_fn(ctx, fn_val, item)? {
            return Ok(Some(item.clone()));
        }
    }
    Ok(None)
}

/// Find the index of the first element matching a predicate.
/// Returns `None` if no match.
pub(crate) fn find_index<'a, I>(
    ctx: &mut dyn Context<NonFunctionValue>,
    fn_val: &FunctionValue,
    iter: I,
) -> Result<Option<usize>, Error>
where
    I: Iterator<Item = &'a serde_json::Value>,
{
    for (i, item) in iter.enumerate() {
        if is_truthy_fn(ctx, fn_val, item)? {
            return Ok(Some(i));
        }
    }
    Ok(None)
}

/// Find the first matching element from a reverse iterator.
/// Returns `None` if no match.
pub(crate) fn find_first_rev<'a, I>(
    ctx: &mut dyn Context<NonFunctionValue>,
    fn_val: &FunctionValue,
    iter: I,
) -> Result<Option<serde_json::Value>, Error>
where
    I: Iterator<Item = &'a serde_json::Value>,
{
    find_first(ctx, fn_val, iter)
}

/// Find the index of the first matching element from a reverse iterator.
/// The index is relative to the reversed iteration order.
/// Returns `None` if no match.
pub(crate) fn find_index_rev<'a, I>(
    ctx: &mut dyn Context<NonFunctionValue>,
    fn_val: &FunctionValue,
    iter: I,
) -> Result<Option<usize>, Error>
where
    I: Iterator<Item = &'a serde_json::Value>,
{
    find_index(ctx, fn_val, iter)
}

#[cfg(test)]
use handl_vm::{Function, Host};
#[cfg(test)]
use handl_vm_common::Module;

#[cfg(test)]
use handl_vm_host_canonical::CanonicalHost;

#[cfg(test)]
pub(crate) struct TestHost {
    default: CanonicalHost,
    math: crate::MathModule,
    str: crate::StrModule,
    array: crate::ArrayModule,
    r#fn: crate::FnModule,
}

#[cfg(test)]
impl Default for TestHost {
    fn default() -> Self {
        TestHost {
            default: CanonicalHost::default(),
            math: crate::MathModule,
            str: crate::StrModule,
            array: crate::ArrayModule,
            r#fn: crate::FnModule,
        }
    }
}

#[cfg(test)]
impl Clone for TestHost {
    fn clone(&self) -> Self {
        TestHost {
            default: self.default.clone(),
            math: self.math,
            str: self.str,
            array: self.array,
            r#fn: self.r#fn,
        }
    }
}

#[cfg(test)]
impl Host<NonFunctionValue> for TestHost {
    fn function(&self, name: &str) -> Option<&dyn Function<NonFunctionValue>> {
        match name {
            "add" => self.math.function("add"),
            "sub" => self.math.function("sub"),
            "mul" => self.math.function("mul"),
            "div" => self.math.function("div"),
            "concat" => self.str.function("concat"),
            "len" => self.str.function("length"),
            "apply" => self.r#fn.function("apply"),
            "array" => self.array.function("array"),
            _ => self.default.function(name),
        }
    }
}
