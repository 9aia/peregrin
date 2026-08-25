use handl_ast_canonical::Expr;
use handl_vm::{Context, Function, LazyFunction};
use handl_vm_values_json::Error;

use handl_vm_values_json::{FunctionValue, NonFunctionValue};
use serde_json::Value as JsonValue;

use crate::shared::{
    apply_fn, apply_fn2, arr, bind_function_params, is_truthy_fn, require_array,
    require_function_value, require_functions,
};

fn substitute_param(body: &str, param: &str, replacement: &str) -> String {
    let wrapped = format!("({replacement})");
    let mut result = String::new();
    let bytes = body.as_bytes();
    let mut i = 0;
    let mut in_string = false;
    while i < body.len() {
        let ch = bytes[i];
        if in_string {
            if ch == b'\\' && i + 1 < body.len() {
                result.push_str(&body[i..i + 2]);
                i += 2;
                continue;
            }
            if ch == b'"' {
                in_string = false;
                result.push('"');
                i += 1;
                continue;
            }
            let end = next_char_boundary(body, i);
            result.push_str(&body[i..end]);
            i = end;
            continue;
        }
        if ch == b'"' {
            in_string = true;
            result.push('"');
            i += 1;
            continue;
        }
        if body[i..].starts_with(param) {
            let before_ok = i == 0 || !is_ident_char(bytes[i - 1]);
            let after = i + param.len();
            let after_ok = after >= body.len() || !is_ident_char(bytes[after]);
            if before_ok && after_ok {
                result.push_str(&wrapped);
                i = after;
                continue;
            }
        }
        let end = next_char_boundary(body, i);
        result.push_str(&body[i..end]);
        i = end;
    }
    result
}

fn next_char_boundary(s: &str, i: usize) -> usize {
    let mut j = i + 1;
    while j < s.len() && !s.is_char_boundary(j) {
        j += 1;
    }
    j
}

fn is_ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn inline_chain(fns: &[FunctionValue]) -> Result<FunctionValue, Error> {
    if fns.is_empty() {
        return Err(Error::ArgumentCount {
            expected: 1,
            got: 0,
        });
    }
    let first_fn = &fns[0];
    let input_param = first_fn
        .params
        .first()
        .map(|(name, _)| name.as_str())
        .unwrap_or("x");

    let mut body = first_fn.body.clone();
    for fn_val in &fns[1..] {
        let param = fn_val
            .params
            .first()
            .map(|(name, _)| name.as_str())
            .unwrap_or("x");
        body = substitute_param(&fn_val.body, param, &body);
    }

    Ok(FunctionValue {
        params: vec![(input_param.to_string(), serde_json::Value::Null)],
        return_schema: None,
        body,
        patterns: Vec::new(),
    })
}

// ── pipe ───────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Pipe;
impl Function<NonFunctionValue> for Pipe {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        let mut value = args[0].clone();
        for fn_arg in &args[1..] {
            let fn_val = require_function_value(fn_arg)?;
            value = apply_fn(ctx, &fn_val, &value.into_inner())?;
        }
        Ok(value)
    }
}

// ── flow / compose ─────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Flow;
impl Function<NonFunctionValue> for Flow {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let fns = require_functions(args)?;
        let composed = inline_chain(&fns)?;
        Ok(NonFunctionValue::new(composed))
    }
}

#[derive(Clone, Copy)]
pub struct Compose;
impl Function<NonFunctionValue> for Compose {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let fns = require_functions(args)?;
        let reversed: Vec<FunctionValue> = fns.into_iter().rev().collect();
        let composed = inline_chain(&reversed)?;
        Ok(NonFunctionValue::new(composed))
    }
}

// ── fanout ─────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Fanout;
impl Function<NonFunctionValue> for Fanout {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        let value = args[0].clone();
        let inner = value.into_inner();
        let mut results = Vec::with_capacity(args.len() - 1);
        for fn_arg in &args[1..] {
            let fn_val = require_function_value(fn_arg)?;
            let result = apply_fn(ctx, &fn_val, &inner)?;
            results.push(result.into_inner());
        }
        Ok(arr(&results))
    }
}

// ── choice ─────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Choice;
impl Function<NonFunctionValue> for Choice {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 4 {
            return Err(Error::ArgumentCount {
                expected: 4,
                got: args.len(),
            });
        }
        let value = &args[0];
        let predicate = require_function_value(&args[1])?;
        let on_true = require_function_value(&args[2])?;
        let on_false = require_function_value(&args[3])?;

        let inner = value.clone().into_inner();
        let is_true = is_truthy_fn(ctx, &predicate, &inner)?;
        let branch = if is_true { &on_true } else { &on_false };
        Ok(apply_fn(ctx, branch, &inner)?)
    }
}

// ── apply ──────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Apply;
impl Function<NonFunctionValue> for Apply {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        let fn_val = require_function_value(&args[0])?;
        let call_args = require_array(&args[1])?;
        let vars = bind_function_params(&fn_val, call_args)?;
        Ok(ctx.eval_expr(&fn_val.body, &vars)?)
    }
}

// ── tap ────────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Tap;
impl Function<NonFunctionValue> for Tap {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        let fn_val = require_function_value(&args[0])?;
        let value = &args[1];
        let inner = value.clone().into_inner();
        let _ = apply_fn(ctx, &fn_val, &inner)?;
        Ok(value.clone())
    }
}

// ── constantly ─────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Constantly;
impl Function<NonFunctionValue> for Constantly {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let value = args.first().ok_or(Error::ArgumentCount {
            expected: 1,
            got: 0,
        })?;
        let captured = value.inner().clone();
        let fn_val = FunctionValue {
            params: vec![("_".to_string(), serde_json::Value::Null)],
            return_schema: None,
            body: serde_json::to_string(&captured).unwrap_or_else(|_| "null".to_string()),
            patterns: Vec::new(),
        };
        Ok(NonFunctionValue::new(fn_val))
    }
}

// ── flip ───────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Flip;
impl Function<NonFunctionValue> for Flip {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let fn_val = require_function_value(args.first().ok_or(Error::ArgumentCount {
            expected: 1,
            got: 0,
        })?)?;
        if fn_val.params.len() < 2 {
            return Err(Error::TypeMismatch {
                expected: "function with at least 2 parameters",
                got: "function with fewer than 2 parameters",
            });
        }
        let mut swapped_params = fn_val.params.clone();
        swapped_params.swap(0, 1);
        let flipped = FunctionValue {
            params: swapped_params,
            return_schema: None,
            body: fn_val.body.clone(),
            patterns: Vec::new(),
        };
        Ok(NonFunctionValue::new(flipped))
    }
}

// ── partial ────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Partial;
impl Function<NonFunctionValue> for Partial {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        let fn_val = require_function_value(&args[0])?;
        let bound_args = require_array(&args[1])?;
        let bound_count = bound_args.len().min(fn_val.params.len());
        if bound_count == 0 {
            return Ok(NonFunctionValue::new(fn_val));
        }
        let remaining_params: Vec<(String, serde_json::Value)> = fn_val.params[bound_count..]
            .iter()
            .map(|(name, schema)| (name.clone(), schema.clone()))
            .collect();
        let mut body = fn_val.body.clone();
        for i in 0..bound_count {
            let param_name = &fn_val.params[i].0;
            let replacement =
                serde_json::to_string(&bound_args[i]).unwrap_or_else(|_| "null".to_string());
            body = substitute_param(&body, param_name, &replacement);
        }
        let partial_fn = FunctionValue {
            params: remaining_params,
            return_schema: None,
            body,
            patterns: Vec::new(),
        };
        Ok(NonFunctionValue::new(partial_fn))
    }
}

// ── when / unless ──────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct When;
impl Function<NonFunctionValue> for When {
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
        let value = &args[0];
        let predicate = require_function_value(&args[1])?;
        let transform = require_function_value(&args[2])?;
        let inner = value.clone().into_inner();
        if is_truthy_fn(ctx, &predicate, &inner)? {
            Ok(apply_fn(ctx, &transform, &inner)?)
        } else {
            Ok(value.clone())
        }
    }
}

#[derive(Clone, Copy)]
pub struct Unless;
impl Function<NonFunctionValue> for Unless {
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
        let value = &args[0];
        let predicate = require_function_value(&args[1])?;
        let transform = require_function_value(&args[2])?;
        let inner = value.clone().into_inner();
        if is_truthy_fn(ctx, &predicate, &inner)? {
            Ok(value.clone())
        } else {
            Ok(apply_fn(ctx, &transform, &inner)?)
        }
    }
}

// ── converge ───────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Converge;
impl Function<NonFunctionValue> for Converge {
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
        let value = &args[0];
        let combiner = require_function_value(&args[1])?;
        let branch_count = args.len() - 2;
        let mut results = Vec::with_capacity(branch_count);
        let inner = value.clone().into_inner();
        for branch_arg in &args[2..] {
            let branch_fn = require_function_value(branch_arg)?;
            let result = apply_fn(ctx, &branch_fn, &inner)?;
            results.push(result.into_inner());
        }
        let combined = if results.len() == 1 {
            results.into_iter().next().unwrap()
        } else {
            let mut acc = results[0].clone();
            for item in &results[1..] {
                acc = apply_fn2(ctx, &combiner, &acc, item)?.into_inner();
            }
            acc
        };
        Ok(NonFunctionValue::new(combined))
    }
}

// ════════════════════════════════════════════════════════════════════
// Pattern matching
// ════════════════════════════════════════════════════════════════════

fn match_pattern(
    pattern: &JsonValue,
    value: &JsonValue,
    out: &mut Vec<(String, JsonValue)>,
) -> Result<bool, Error> {
    if let Some(obj) = pattern.as_object() {
        if let Some(type_name) = obj.get("type").and_then(|v| v.as_str()) {
            match type_name {
                "wildcard" => return Ok(true),
                "literal" => {
                    let expected = pattern.get("value").ok_or_else(|| {
                        Error::PrimitiveError("literal pattern missing `value`".into())
                    })?;
                    return Ok(expected == value);
                }
                "identifier" => {
                    let name = pattern
                        .get("name")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            Error::PrimitiveError("identifier pattern missing `name`".into())
                        })?;
                    out.push((name.to_string(), value.clone()));
                    return Ok(true);
                }
                "array" => {
                    let patterns = pattern
                        .get("patterns")
                        .and_then(|v| v.as_array())
                        .ok_or_else(|| {
                            Error::PrimitiveError("array pattern missing `patterns`".into())
                        })?;
                    let arr = match value.as_array() {
                        Some(a) => a,
                        None => return Ok(false),
                    };
                    for (i, child_pattern) in patterns.iter().enumerate() {
                        let child = arr.get(i).unwrap_or(&JsonValue::Null);
                        if !match_pattern(child_pattern, child, out)? {
                            return Ok(false);
                        }
                    }
                    return Ok(true);
                }
                "object" => {
                    let entries = pattern
                        .get("entries")
                        .and_then(|v| v.as_array())
                        .ok_or_else(|| {
                            Error::PrimitiveError("object pattern missing `entries`".into())
                        })?;
                    let obj = match value.as_object() {
                        Some(o) => o,
                        None => return Ok(false),
                    };
                    for entry in entries {
                        let arr = entry.as_array().ok_or_else(|| {
                            Error::PrimitiveError("invalid object pattern entry".into())
                        })?;
                        let key = arr.get(0).and_then(|v| v.as_str()).ok_or_else(|| {
                            Error::PrimitiveError("invalid object pattern key".into())
                        })?;
                        let child_pattern = arr.get(1).ok_or_else(|| {
                            Error::PrimitiveError("invalid object pattern entry".into())
                        })?;
                        let child = obj.get(key).unwrap_or(&JsonValue::Null);
                        if !match_pattern(child_pattern, child, out)? {
                            return Ok(false);
                        }
                    }
                    return Ok(true);
                }
                _ => {
                    return Err(Error::PrimitiveError(format!(
                        "unknown pattern type: {type_name}"
                    )));
                }
            }
        }
    }
    Ok(pattern == value)
}

// ── match ───────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Match;
impl Function<NonFunctionValue> for Match {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        let value = args[0].inner();
        for branch_arg in &args[1..] {
            let branch = branch_arg.as_array().ok_or(Error::TypeMismatch {
                expected: "array [pattern, handler]",
                got: "non-array branch",
            })?;
            if branch.len() < 2 {
                return Err(Error::ArgumentCount {
                    expected: 2,
                    got: branch.len(),
                });
            }
            let pattern = &branch[0];
            let handler = &branch[1];
            let mut bindings = Vec::new();
            if match_pattern(pattern, value, &mut bindings)? {
                let fn_val = FunctionValue::from_json(handler).ok_or(Error::TypeMismatch {
                    expected: "function value",
                    got: "non-function branch handler",
                })?;
                let vars: Vec<(String, NonFunctionValue)> = bindings
                    .into_iter()
                    .map(|(name, val)| (name, NonFunctionValue::new(val)))
                    .collect();
                return ctx.eval_expr(&fn_val.body, &vars);
            }
        }
        Err(Error::PrimitiveError("no matching branch".into()))
    }
}

// ── lazy control flow ───────────────────────────────────────────────

/// Lazy `if(cond, then, else)` — only evaluates the chosen branch.
#[derive(Clone, Copy)]
pub struct IfLazy;

impl LazyFunction<NonFunctionValue> for IfLazy {
    fn call(
        &self,
        args: &[Expr<NonFunctionValue>],
        ctx: &mut dyn Context<NonFunctionValue>,
        evaluate: &mut dyn FnMut(
            &Expr<NonFunctionValue>,
            &mut dyn Context<NonFunctionValue>,
        ) -> Result<NonFunctionValue, Error>,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 3 {
            return Err(Error::ArgumentCount {
                expected: 3,
                got: args.len(),
            });
        }
        let cond = evaluate(&args[0], ctx)?;
        let cond = cond.as_bool().ok_or(Error::TypeMismatch {
            expected: "boolean",
            got: "non-boolean",
        })?;
        if cond {
            evaluate(&args[1], ctx)
        } else {
            evaluate(&args[2], ctx)
        }
    }
}
