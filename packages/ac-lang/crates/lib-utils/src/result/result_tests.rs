use ac_runtime::Eval;
use ac_runtime_modules::Module;
use ac_runtime_values::{CanonicalContext, Error, NonFunctionValue};

use super::ResultModule;
use crate::shared::TestHost;

use ac_ast_canonical::Expr;
use ac_runtime_canonical::Vm;

// ── test eval infrastructure ───────────────────────────────────────

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
    if arg == "null" {
        return Ok(NonFunctionValue::new(serde_json::Value::Null));
    }
    if arg.starts_with('"') && arg.ends_with('"') {
        return Ok(NonFunctionValue::new(arg[1..arg.len() - 1].to_string()));
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

fn ctx_with_eval() -> CanonicalContext {
    let mut ctx = CanonicalContext::new();
    ctx.set_eval(Box::new(TestEval));
    ctx
}

fn fn_value(body: &str, params: &[&str]) -> NonFunctionValue {
    let params_json: Vec<serde_json::Value> = params
        .iter()
        .map(|p| serde_json::json!({"name": p, "schema": {}}))
        .collect();
    NonFunctionValue::new(serde_json::json!({
        "params": params_json,
        "body": body}))
}

fn n(v: f64) -> NonFunctionValue {
    NonFunctionValue::new(serde_json::json!(v))
}

fn s(v: &str) -> NonFunctionValue {
    NonFunctionValue::new(v.to_string())
}

fn call(name: &str, args: &[NonFunctionValue]) -> NonFunctionValue {
    let func = ResultModule.function(name).expect("function exists");
    let mut ctx = CanonicalContext::new();
    func.call(args, &mut ctx).expect("call succeeds")
}

fn call_err(name: &str, args: &[NonFunctionValue]) -> Result<NonFunctionValue, Error> {
    let func = ResultModule.function(name).expect("function exists");
    let mut ctx = CanonicalContext::new();
    func.call(args, &mut ctx)
}

fn call_with_eval(name: &str, args: &[NonFunctionValue]) -> NonFunctionValue {
    let func = ResultModule.function(name).expect("function exists");
    let mut ctx = ctx_with_eval();
    func.call(args, &mut ctx).expect("call succeeds")
}

fn call_err_with_eval(
    name: &str,
    args: &[NonFunctionValue],
) -> Result<NonFunctionValue, Error> {
    let func = ResultModule.function(name).expect("function exists");
    let mut ctx = ctx_with_eval();
    func.call(args, &mut ctx)
}

// ── constructors ───────────────────────────────────────────────────

#[test]
fn ok_wraps_value() {
    let value = n(42.0);
    let result = call("ok", &[value]);
    assert_eq!(
        result.inner(),
        &serde_json::json!({"_tag": "Ok", "value": 42.0})
    );
}

#[test]
fn err_wraps_error() {
    let error = s("nope");
    let result = call("err", &[error]);
    assert_eq!(
        result.inner(),
        &serde_json::json!({"_tag": "Err", "error": "nope"})
    );
}

#[test]
fn constructors_require_one_arg() {
    assert!(matches!(call_err("ok", &[]), Err(Error::ArgumentCount { .. })));
    assert!(matches!(call_err("err", &[]), Err(Error::ArgumentCount { .. })));
}

// ── predicates ─────────────────────────────────────────────────────

#[test]
fn is_ok_true_for_ok() {
    let ok = call("ok", &[n(1.0)]);
    let result = call("is_ok", &[ok]);
    assert_eq!(result.as_bool(), Some(true));
}

#[test]
fn is_ok_false_for_err() {
    let err = call("err", &[s("bad")]);
    let result = call("is_ok", &[err]);
    assert_eq!(result.as_bool(), Some(false));
}

#[test]
fn is_ok_false_for_non_result() {
    let result = call("is_ok", &[n(42.0)]);
    assert_eq!(result.as_bool(), Some(false));
}

#[test]
fn is_err_true_for_err() {
    let err = call("err", &[s("bad")]);
    let result = call("is_err", &[err]);
    assert_eq!(result.as_bool(), Some(true));
}

#[test]
fn is_err_false_for_ok() {
    let ok = call("ok", &[n(1.0)]);
    let result = call("is_err", &[ok]);
    assert_eq!(result.as_bool(), Some(false));
}

#[test]
fn is_err_false_for_non_result() {
    let result = call("is_err", &[n(42.0)]);
    assert_eq!(result.as_bool(), Some(false));
}

#[test]
fn predicates_require_one_arg() {
    assert!(matches!(call_err("is_ok", &[]), Err(Error::ArgumentCount { .. })));
    assert!(matches!(call_err("is_err", &[]), Err(Error::ArgumentCount { .. })));
}

// ── match ──────────────────────────────────────────────────────────

#[test]
fn match_calls_on_ok_branch() {
    let ok = call("ok", &[n(5.0)]);
    let on_ok = fn_value("add(x, 10)", &["x"]);
    let on_err = fn_value("add(x, 1000)", &["x"]);
    let result = call_with_eval("match", &[ok, on_ok, on_err]);
    assert_eq!(result.as_number(), Some(15.0));
}

#[test]
fn match_calls_on_err_branch() {
    let err = call("err", &[n(3.0)]);
    let on_ok = fn_value("add(x, 10)", &["x"]);
    let on_err = fn_value("add(x, 100)", &["x"]);
    let result = call_with_eval("match", &[err, on_ok, on_err]);
    assert_eq!(result.as_number(), Some(103.0));
}

#[test]
fn match_errors_on_non_result() {
    let on_ok = fn_value("x", &["x"]);
    let on_err = fn_value("x", &["x"]);
    let result = call_err_with_eval("match", &[n(42.0), on_ok, on_err]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn match_errors_for_missing_callbacks() {
    let ok = call("ok", &[n(1.0)]);
    assert!(matches!(
        call_err_with_eval("match", &[ok.clone()]),
        Err(Error::ArgumentCount { .. })
    ));
    assert!(matches!(
        call_err_with_eval("match", &[ok.clone(), fn_value("x", &["x"])]),
        Err(Error::ArgumentCount { .. })
    ));
}

#[test]
fn match_errors_for_missing_payload_field() {
    let ok_missing_value = NonFunctionValue::new(serde_json::json!({"_tag": "Ok"}));
    let err_missing_error = NonFunctionValue::new(serde_json::json!({"_tag": "Err"}));
    let on_ok = fn_value("x", &["x"]);
    let on_err = fn_value("x", &["x"]);

    assert!(matches!(
        call_err_with_eval("match", &[ok_missing_value, on_ok.clone(), on_err.clone()]),
        Err(Error::TypeMismatch { .. })
    ));
    assert!(matches!(
        call_err_with_eval("match", &[err_missing_error, on_ok, on_err]),
        Err(Error::TypeMismatch { .. })
    ));
}
