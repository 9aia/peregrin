use ac_runtime::{FunctionValue, NonFunctionValue};
use ac_vm::{Context, Error, Function};

use crate::shared::{
    arr, apply_fn, apply_fn2, bind_all_params, is_truthy_fn, require_array, require_function_value,
    require_functions,
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
    })
}

// ── pipe ───────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Pipe;
impl Function for Pipe {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut Context,
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
impl Function for Flow {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let fns = require_functions(args)?;
        let composed = inline_chain(&fns)?;
        Ok(NonFunctionValue::new_function(composed))
    }
}

#[derive(Clone, Copy)]
pub struct Compose;
impl Function for Compose {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let fns = require_functions(args)?;
        let reversed: Vec<FunctionValue> = fns.into_iter().rev().collect();
        let composed = inline_chain(&reversed)?;
        Ok(NonFunctionValue::new_function(composed))
    }
}

// ── fanout ─────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Fanout;
impl Function for Fanout {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut Context,
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
impl Function for Choice {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut Context,
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
impl Function for Apply {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        let fn_val = require_function_value(&args[0])?;
        let call_args = require_array(&args[1])?;
        let vars = bind_all_params(&fn_val, call_args);
        Ok(ctx.eval_expr(&fn_val.body, &vars)?)
    }
}

// ── tap ────────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Tap;
impl Function for Tap {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut Context,
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
impl Function for Constantly {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
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
        };
        Ok(NonFunctionValue::new_function(fn_val))
    }
}

// ── flip ───────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Flip;
impl Function for Flip {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
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
        };
        Ok(NonFunctionValue::new_function(flipped))
    }
}

// ── partial ────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Partial;
impl Function for Partial {
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
        let fn_val = require_function_value(&args[0])?;
        let bound_args = require_array(&args[1])?;
        let bound_count = bound_args.len().min(fn_val.params.len());
        if bound_count == 0 {
            return Ok(NonFunctionValue::new_function(fn_val));
        }
        let remaining_params: Vec<(String, serde_json::Value)> = fn_val.params[bound_count..]
            .iter()
            .map(|(name, schema)| (name.clone(), schema.clone()))
            .collect();
        let mut body = fn_val.body.clone();
        for i in 0..bound_count {
            let param_name = &fn_val.params[i].0;
            let replacement = serde_json::to_string(&bound_args[i])
                .unwrap_or_else(|_| "null".to_string());
            body = substitute_param(&body, param_name, &replacement);
        }
        let partial_fn = FunctionValue {
            params: remaining_params,
            return_schema: None,
            body,
        };
        Ok(NonFunctionValue::new_function(partial_fn))
    }
}

// ── when / unless ──────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct When;
impl Function for When {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut Context,
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
impl Function for Unless {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut Context,
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
impl Function for Converge {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut Context,
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
