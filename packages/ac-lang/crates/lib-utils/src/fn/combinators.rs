use ac_runtime::{FunctionValue, NonFunctionValue};
use ac_vm::{Context, Error, Function};

fn arr(items: &[serde_json::Value]) -> NonFunctionValue {
    NonFunctionValue::new(serde_json::Value::Array(items.to_vec()))
}

fn require_function_value(arg: &NonFunctionValue) -> Result<FunctionValue, Error> {
    arg.as_function().ok_or(Error::TypeMismatch {
        expected: "function value",
        got: "non-function",
    })
}

fn bind_param(
    fn_val: &FunctionValue,
    value: &serde_json::Value,
) -> Vec<(String, NonFunctionValue)> {
    fn_val
        .params
        .first()
        .map(|(name, _)| vec![(name.clone(), NonFunctionValue::new(value.clone()))])
        .unwrap_or_default()
}

fn is_truthy_value(value: &NonFunctionValue) -> bool {
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

fn require_functions(args: &[NonFunctionValue]) -> Result<Vec<FunctionValue>, Error> {
    if args.is_empty() {
        return Err(Error::ArgumentCount {
            expected: 1,
            got: 0,
        });
    }
    args.iter().map(require_function_value).collect()
}

fn substitute_param(body: &str, param: &str, replacement: &str) -> String {
    let wrapped = format!("({replacement})");
    let mut result = String::new();
    let bytes = body.as_bytes();
    let mut i = 0;
    let mut in_string = false;
    while i < body.len() {
        let ch = bytes[i];
        if in_string {
            result.push(ch as char);
            if ch == b'"' {
                in_string = false;
            }
            i += 1;
            continue;
        }
        if ch == b'"' {
            in_string = true;
            result.push(ch as char);
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
        result.push(ch as char);
        i += 1;
    }
    result
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

fn require_array(arg: &NonFunctionValue) -> Result<&[serde_json::Value], Error> {
    arg.as_array().ok_or(Error::TypeMismatch {
        expected: "array",
        got: "non-array",
    })
}

fn bind_all_params(
    fn_val: &FunctionValue,
    args: &[serde_json::Value],
) -> Vec<(String, NonFunctionValue)> {
    fn_val
        .params
        .iter()
        .enumerate()
        .map(|(i, (name, _))| {
            let val = args.get(i).cloned().unwrap_or(serde_json::Value::Null);
            (name.clone(), NonFunctionValue::new(val))
        })
        .collect()
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
            let vars = bind_param(&fn_val, &value.into_inner());
            value = ctx.eval_expr(&fn_val.body, &vars)?;
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
            let vars = bind_param(&fn_val, &inner);
            let result = ctx.eval_expr(&fn_val.body, &vars)?;
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
        let vars = bind_param(&predicate, &inner);
        let cond = ctx.eval_expr(&predicate.body, &vars)?;
        let branch = if is_truthy_value(&cond) {
            &on_true
        } else {
            &on_false
        };
        let vars = bind_param(branch, &inner);
        Ok(ctx.eval_expr(&branch.body, &vars)?)
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
        let vars = bind_param(&fn_val, &inner);
        let _ = ctx.eval_expr(&fn_val.body, &vars)?;
        Ok(value.clone())
    }
}
