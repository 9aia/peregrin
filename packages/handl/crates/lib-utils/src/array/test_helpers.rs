use handl_vm_values_json::NonFunctionValue;

use handl_vm::Eval;
use handl_vm_values_json::{CanonicalContext, Error};

use handl_vm_common::Module;

use super::module::ArrayModule;

pub(super) fn call(name: &str, args: &[NonFunctionValue]) -> NonFunctionValue {
    let func = ArrayModule.function(name).expect("function exists");
    let mut ctx = CanonicalContext::new();
    func.call(args, &mut ctx).expect("call succeeds")
}

pub(super) fn call_err(name: &str, args: &[NonFunctionValue]) -> Result<NonFunctionValue, Error> {
    let func = ArrayModule.function(name).expect("function exists");
    let mut ctx = CanonicalContext::new();
    func.call(args, &mut ctx)
}

pub(super) fn arr(items: &[serde_json::Value]) -> NonFunctionValue {
    NonFunctionValue::new(serde_json::Value::Array(items.to_vec()))
}

pub(super) fn n(v: f64) -> NonFunctionValue {
    NonFunctionValue::new(serde_json::json!(v))
}

pub(super) fn ni(v: i64) -> NonFunctionValue {
    NonFunctionValue::new(serde_json::json!(v))
}

pub(super) fn s(v: &str) -> NonFunctionValue {
    NonFunctionValue::new(v.to_string())
}

pub(super) fn fn_value(body: &str, params: &[&str]) -> NonFunctionValue {
    let params_json: Vec<serde_json::Value> = params
        .iter()
        .map(|p| serde_json::json!({"name": p, "schema": {}}))
        .collect();
    NonFunctionValue::new(serde_json::json!({
        "params": params_json,
        "body": body}))
}

// ── TestEval infrastructure ────────────────────────────────────────

use crate::shared::TestHost;
use handl_ast_canonical::Expr;
use handl_vm_backend_rust_interpreter::Vm;

struct TestEval;
impl Eval<NonFunctionValue> for TestEval {
    fn eval_expr(
        &self,
        expr: &str,
        vars: &[(String, NonFunctionValue)],
    ) -> Result<NonFunctionValue, Error> {
        let expr = expr.trim();
        if expr.starts_with('(') && expr.ends_with(')') {
            let inner = &expr[1..expr.len() - 1];
            if inner.matches('(').count() == inner.matches(')').count() {
                return self.eval_expr(inner, vars);
            }
        }
        if let Some(paren) = expr.find('(')
            && expr.ends_with(')')
            && paren < expr.len() - 1
        {
            let name = &expr[..paren];
            let args_str = &expr[paren + 1..expr.len() - 1];
            let mut args = Vec::new();
            for arg_str in split_args(args_str) {
                let arg_val = eval_arg(&arg_str, vars)?;
                args.push(Expr::Value(arg_val));
            }
            let vm = Vm::new(TestHost::default());
            let mut ctx = CanonicalContext::new();
            let call_expr = Expr::call(name.trim(), args);
            return vm.eval(&call_expr, &mut ctx);
        }
        for (name, val) in vars {
            if name == expr.trim() {
                return Ok(val.clone());
            }
        }
        if let Ok(n) = expr.parse::<f64>() {
            return Ok(NonFunctionValue::new(n));
        }
        if expr == "true" {
            return Ok(NonFunctionValue::new(true));
        }
        if expr == "false" {
            return Ok(NonFunctionValue::new(false));
        }
        if expr.starts_with('"') && expr.ends_with('"') {
            return Ok(NonFunctionValue::new(expr[1..expr.len() - 1].to_string()));
        }
        if expr == "null" {
            return Ok(NonFunctionValue::new(serde_json::Value::Null));
        }
        Err(Error::UnknownVariable(expr.to_string()))
    }
}

fn split_args(s: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut depth = 0;
    let mut current = String::new();
    for ch in s.chars() {
        match ch {
            '(' => {
                depth += 1;
                current.push(ch);
            }
            ')' => {
                depth -= 1;
                current.push(ch);
            }
            ',' if depth == 0 => {
                result.push(current.trim().to_string());
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    if !current.trim().is_empty() {
        result.push(current.trim().to_string());
    }
    result
}

fn eval_arg(arg: &str, vars: &[(String, NonFunctionValue)]) -> Result<NonFunctionValue, Error> {
    let arg = arg.trim();
    if arg.starts_with('(') && arg.ends_with(')') {
        let inner = &arg[1..arg.len() - 1];
        if inner.matches('(').count() == inner.matches(')').count() {
            return TestEval.eval_expr(inner, vars);
        }
    }
    if let Some(n) = arg.parse::<f64>().ok() {
        return Ok(NonFunctionValue::new(n));
    }
    if arg == "true" {
        return Ok(NonFunctionValue::new(true));
    }
    if arg == "false" {
        return Ok(NonFunctionValue::new(false));
    }
    if arg.starts_with('"') && arg.ends_with('"') {
        return Ok(NonFunctionValue::new(arg[1..arg.len() - 1].to_string()));
    }
    if arg == "null" {
        return Ok(NonFunctionValue::new(serde_json::Value::Null));
    }
    if arg.find('(').is_some() {
        return TestEval.eval_expr(arg, vars);
    }
    for (name, val) in vars {
        if name == arg {
            return Ok(val.clone());
        }
    }
    Err(Error::UnknownVariable(arg.to_string()))
}

pub(super) fn call_with_eval(name: &str, args: &[NonFunctionValue]) -> NonFunctionValue {
    let func = ArrayModule.function(name).expect("function exists");
    let mut ctx = CanonicalContext::new();
    ctx.set_eval(Box::new(TestEval));
    func.call(args, &mut ctx).expect("call succeeds")
}

pub(super) fn call_err_with_eval(
    name: &str,
    args: &[NonFunctionValue],
) -> Result<NonFunctionValue, Error> {
    let func = ArrayModule.function(name).expect("function exists");
    let mut ctx = CanonicalContext::new();
    ctx.set_eval(Box::new(TestEval));
    func.call(args, &mut ctx)
}
