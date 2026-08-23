use ac_runtime::NonFunctionValue;
use ac_runtime::{CanonicalContext, Context, Error, Eval};
use ac_runtime_modules::Module;

use super::module::FnModule;

fn call(name: &str, args: &[NonFunctionValue]) -> NonFunctionValue {
    let func = FnModule.function(name).expect("function exists");
    let mut ctx = CanonicalContext::new();
    func.call(args, &mut ctx).expect("call succeeds")
}

fn call_err(name: &str, args: &[NonFunctionValue]) -> Result<NonFunctionValue, Error> {
    let func = FnModule.function(name).expect("function exists");
    let mut ctx = CanonicalContext::new();
    func.call(args, &mut ctx)
}

fn n(v: f64) -> NonFunctionValue {
    NonFunctionValue::new(serde_json::json!(v))
}

fn s(v: &str) -> NonFunctionValue {
    NonFunctionValue::new(v.to_string())
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

// ── TestEval infrastructure ────────────────────────────────────────

use ac_ast_canonical::Expr;
use ac_runtime_canonical::Vm;
use ac_runtime_example_1::DefaultHost;

struct TestEval;
impl Eval for TestEval {
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
            let vm = Vm::new(DefaultHost::default());
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

fn call_with_eval(name: &str, args: &[NonFunctionValue]) -> NonFunctionValue {
    let func = FnModule.function(name).expect("function exists");
    let mut ctx = CanonicalContext::new();
    ctx.set_eval(Box::new(TestEval));
    func.call(args, &mut ctx).expect("call succeeds")
}

fn call_err_with_eval(name: &str, args: &[NonFunctionValue]) -> Result<NonFunctionValue, Error> {
    let func = FnModule.function(name).expect("function exists");
    let mut ctx = CanonicalContext::new();
    ctx.set_eval(Box::new(TestEval));
    func.call(args, &mut ctx)
}

// ── module ─────────────────────────────────────────────────────────

#[test]
fn module_name() {
    assert_eq!(FnModule.name(), "fn");
}

// ── pipe ───────────────────────────────────────────────────────────

#[test]
fn pipe_chains_functions_left_to_right() {
    let result = call_with_eval(
        "pipe",
        &[
            n(5.0),
            fn_value("add(x, 10)", &["x"]),
            fn_value("mul(x, 2)", &["x"]),
        ],
    );
    assert_eq!(result.as_number(), Some(30.0));
}

#[test]
fn pipe_single_function() {
    let result = call_with_eval("pipe", &[n(3.0), fn_value("add(x, 7)", &["x"])]);
    assert_eq!(result.as_number(), Some(10.0));
}

#[test]
fn pipe_without_eval_errors() {
    let result = call_err("pipe", &[n(3.0), fn_value("add(x, 1)", &["x"])]);
    assert!(matches!(result, Err(Error::PrimitiveError(_))));
}

#[test]
fn pipe_non_function_errors() {
    let result = call_err("pipe", &[n(3.0), n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn pipe_no_args_errors() {
    let result = call_err("pipe", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn pipe_value_only_errors() {
    let result = call_err("pipe", &[n(5.0)]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── flow ───────────────────────────────────────────────────────────

#[test]
fn flow_returns_function_value() {
    let result = call(
        "flow",
        &[
            fn_value("add(x, 10)", &["x"]),
            fn_value("mul(x, 2)", &["x"]),
        ],
    );
    assert!(result.is_function());
    let fv = result.as_function().unwrap();
    assert_eq!(fv.params.len(), 1);
    assert_eq!(fv.params[0].0, "x");
    assert_eq!(fv.body, "mul((add(x, 10)), 2)");
}

#[test]
fn flow_three_functions() {
    let result = call(
        "flow",
        &[
            fn_value("add(x, 1)", &["x"]),
            fn_value("mul(x, 3)", &["x"]),
            fn_value("add(x, 100)", &["x"]),
        ],
    );
    let fv = result.as_function().unwrap();
    assert_eq!(fv.body, "add((mul((add(x, 1)), 3)), 100)");
}

#[test]
fn flow_single_function() {
    let result = call("flow", &[fn_value("add(x, 42)", &["x"])]);
    let fv = result.as_function().unwrap();
    assert_eq!(fv.body, "add(x, 42)");
}

#[test]
fn flow_with_different_param_names() {
    let result = call(
        "flow",
        &[
            fn_value("add(x, 10)", &["x"]),
            fn_value("mul(y, 2)", &["y"]),
        ],
    );
    let fv = result.as_function().unwrap();
    assert_eq!(fv.body, "mul((add(x, 10)), 2)");
}

#[test]
fn flow_no_args_errors() {
    let result = call_err("flow", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn flow_non_function_errors() {
    let result = call_err("flow", &[n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn flow_preserves_param_inside_string_literals() {
    let result = call(
        "flow",
        &[
            fn_value("concat(\"x\", x)", &["x"]),
            fn_value("concat(\"y\", x)", &["x"]),
        ],
    );
    let fv = result.as_function().unwrap();
    assert_eq!(fv.body, "concat(\"y\", (concat(\"x\", x)))");
}

#[test]
fn flow_handles_escaped_quotes_in_string_literals() {
    let result = call(
        "flow",
        &[
            fn_value("add(x, 1)", &["x"]),
            fn_value("concat(\"hello \\\"x\\\"\", x)", &["x"]),
        ],
    );
    let fv = result.as_function().unwrap();
    assert_eq!(fv.body, "concat(\"hello \\\"x\\\"\", (add(x, 1)))");
}

#[test]
fn flow_does_not_substring_match_param_names() {
    let result = call(
        "flow",
        &[
            fn_value("add(x, 1)", &["x"]),
            fn_value("add(x2, x)", &["x"]),
        ],
    );
    let fv = result.as_function().unwrap();
    assert_eq!(fv.body, "add(x2, (add(x, 1)))");
}

#[test]
fn flow_longer_param_name_does_not_match_shorter() {
    let result = call(
        "flow",
        &[
            fn_value("add(x, 1)", &["x"]),
            fn_value("add(x, x2)", &["x2"]),
        ],
    );
    let fv = result.as_function().unwrap();
    assert_eq!(fv.body, "add(x, (add(x, 1)))");
}

#[test]
fn flow_handles_unicode_in_string_literals() {
    let result = call(
        "flow",
        &[
            fn_value("add(x, 1)", &["x"]),
            fn_value("concat(\"café\", x)", &["x"]),
        ],
    );
    let fv = result.as_function().unwrap();
    assert_eq!(fv.body, "concat(\"café\", (add(x, 1)))");
}

// ── compose ────────────────────────────────────────────────────────

#[test]
fn compose_returns_function_value() {
    let result = call(
        "compose",
        &[
            fn_value("add(x, 100)", &["x"]),
            fn_value("mul(x, 2)", &["x"]),
            fn_value("add(x, 10)", &["x"]),
        ],
    );
    let fv = result.as_function().unwrap();
    assert_eq!(fv.body, "add((mul((add(x, 10)), 2)), 100)");
}

#[test]
fn compose_single_function() {
    let result = call("compose", &[fn_value("add(x, 42)", &["x"])]);
    let fv = result.as_function().unwrap();
    assert_eq!(fv.body, "add(x, 42)");
}

#[test]
fn flow_and_compose_are_inverse_order() {
    let f = fn_value("add(x, 1)", &["x"]);
    let g = fn_value("mul(x, 3)", &["x"]);

    let flow_result = call("flow", &[f.clone(), g.clone()]);
    let compose_result = call("compose", &[f, g]);

    let flow_fv = flow_result.as_function().unwrap();
    let compose_fv = compose_result.as_function().unwrap();

    assert_eq!(flow_fv.body, "mul((add(x, 1)), 3)");
    assert_eq!(compose_fv.body, "add((mul(x, 3)), 1)");
}

#[test]
fn compose_no_args_errors() {
    let result = call_err("compose", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn compose_non_function_errors() {
    let result = call_err("compose", &[n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── flow result is callable via eval ───────────────────────────────

#[test]
fn flow_result_is_callable_via_eval() {
    let composed = call(
        "flow",
        &[
            fn_value("add(x, 10)", &["x"]),
            fn_value("mul(x, 2)", &["x"]),
        ],
    );
    let result = call_with_eval("pipe", &[n(5.0), composed]);
    assert_eq!(result.as_number(), Some(30.0));
}

// ── fanout ─────────────────────────────────────────────────────────

#[test]
fn fanout_applies_all_functions_to_value() {
    let result = call_with_eval(
        "fanout",
        &[
            n(5.0),
            fn_value("add(x, 10)", &["x"]),
            fn_value("mul(x, 2)", &["x"]),
            fn_value("add(x, -1)", &["x"]),
        ],
    );
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 3);
    assert_eq!(arr[0].as_f64(), Some(15.0));
    assert_eq!(arr[1].as_f64(), Some(10.0));
    assert_eq!(arr[2].as_f64(), Some(4.0));
}

#[test]
fn fanout_single_function() {
    let result = call_with_eval("fanout", &[n(3.0), fn_value("add(x, 7)", &["x"])]);
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0].as_f64(), Some(10.0));
}

#[test]
fn fanout_without_eval_errors() {
    let result = call_err("fanout", &[n(3.0), fn_value("add(x, 1)", &["x"])]);
    assert!(matches!(result, Err(Error::PrimitiveError(_))));
}

#[test]
fn fanout_non_function_errors() {
    let result = call_err("fanout", &[n(3.0), n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn fanout_too_few_args_errors() {
    let result = call_err("fanout", &[n(3.0)]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn fanout_no_function_args_errors() {
    let result = call_err("fanout", &[n(3.0), n(42.0), n(99.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── choice ─────────────────────────────────────────────────────────

#[test]
fn choice_selects_true_branch() {
    let result = call_with_eval(
        "choice",
        &[
            n(5.0),
            fn_value("add(x, 0)", &["x"]),
            fn_value("mul(x, 100)", &["x"]),
            fn_value("mul(x, -1)", &["x"]),
        ],
    );
    assert_eq!(result.as_number(), Some(500.0));
}

#[test]
fn choice_selects_false_branch() {
    let result = call_with_eval(
        "choice",
        &[
            n(0.0),
            fn_value("add(x, 0)", &["x"]),
            fn_value("mul(x, 100)", &["x"]),
            fn_value("mul(x, -1)", &["x"]),
        ],
    );
    assert_eq!(result.as_number(), Some(0.0));
}

#[test]
fn choice_with_string_predicate() {
    let result = call_with_eval(
        "choice",
        &[
            s("hello"),
            fn_value("x", &["x"]),
            fn_value("add(x, 0)", &["x"]),
            fn_value("add(x, 100)", &["x"]),
        ],
    );
    assert_eq!(result.as_number(), Some(0.0));
}

#[test]
fn choice_without_eval_errors() {
    let result = call_err(
        "choice",
        &[
            n(1.0),
            fn_value("x", &["x"]),
            fn_value("add(x, 1)", &["x"]),
            fn_value("add(x, 2)", &["x"]),
        ],
    );
    assert!(matches!(result, Err(Error::PrimitiveError(_))));
}

#[test]
fn choice_non_function_predicate_errors() {
    let result = call_err(
        "choice",
        &[
            n(1.0),
            n(42.0),
            fn_value("add(x, 1)", &["x"]),
            fn_value("add(x, 2)", &["x"]),
        ],
    );
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn choice_too_few_args_errors() {
    let result = call_err(
        "choice",
        &[n(1.0), fn_value("x", &["x"]), fn_value("add(x, 1)", &["x"])],
    );
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn choice_no_args_errors() {
    let result = call_err("choice", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── apply ──────────────────────────────────────────────────────────

#[test]
fn apply_calls_function_with_args() {
    let result = call_with_eval(
        "apply",
        &[
            fn_value("add(a, b)", &["a", "b"]),
            NonFunctionValue::new(serde_json::json!([3, 4])),
        ],
    );
    assert_eq!(result.as_number(), Some(7.0));
}

#[test]
fn apply_binds_fewer_args_than_params() {
    let result = call_with_eval(
        "apply",
        &[
            fn_value("add(a, b)", &["a", "b"]),
            NonFunctionValue::new(serde_json::json!([3])),
        ],
    );
    assert_eq!(result.as_number(), Some(3.0));
}

#[test]
fn apply_single_arg() {
    let result = call_with_eval(
        "apply",
        &[
            fn_value("mul(x, 2)", &["x"]),
            NonFunctionValue::new(serde_json::json!([5])),
        ],
    );
    assert_eq!(result.as_number(), Some(10.0));
}

#[test]
fn apply_without_eval_errors() {
    let result = call_err(
        "apply",
        &[
            fn_value("add(a, b)", &["a", "b"]),
            NonFunctionValue::new(serde_json::json!([1, 2])),
        ],
    );
    assert!(matches!(result, Err(Error::PrimitiveError(_))));
}

#[test]
fn apply_non_function_errors() {
    let result = call_err(
        "apply",
        &[n(42.0), NonFunctionValue::new(serde_json::json!([1]))],
    );
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn apply_too_few_args_errors() {
    let result = call_err("apply", &[fn_value("add(a, b)", &["a", "b"])]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn apply_non_array_args_errors() {
    let result = call_err("apply", &[fn_value("add(a, b)", &["a", "b"]), n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── tap ────────────────────────────────────────────────────────────

#[test]
fn tap_returns_value_unchanged() {
    let result = call_with_eval("tap", &[fn_value("add(x, 100)", &["x"]), n(42.0)]);
    assert_eq!(result.as_number(), Some(42.0));
}

#[test]
fn tap_with_string() {
    let result = call_with_eval("tap", &[fn_value("x", &["x"]), s("hello")]);
    assert_eq!(result.as_str(), Some("hello"));
}

#[test]
fn tap_without_eval_errors() {
    let result = call_err("tap", &[fn_value("add(x, 1)", &["x"]), n(5.0)]);
    assert!(matches!(result, Err(Error::PrimitiveError(_))));
}

#[test]
fn tap_non_function_errors() {
    let result = call_err("tap", &[n(42.0), n(5.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn tap_too_few_args_errors() {
    let result = call_err("tap", &[fn_value("add(x, 1)", &["x"])]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── unknown function ───────────────────────────────────────────────

#[test]
fn unknown_function_returns_none() {
    assert!(FnModule.function("nope").is_none());
}

// ── constantly ─────────────────────────────────────────────────────

#[test]
fn constantly_returns_function_that_ignores_input() {
    let result = call_with_eval("pipe", &[n(999.0), call("constantly", &[n(42.0)])]);
    assert_eq!(result.as_number(), Some(42.0));
}

#[test]
fn constantly_with_string() {
    let result = call_with_eval("pipe", &[n(1.0), call("constantly", &[s("hello")])]);
    assert_eq!(result.as_str(), Some("hello"));
}

#[test]
fn constantly_no_args_errors() {
    let result = call_err("constantly", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── flip ───────────────────────────────────────────────────────────

#[test]
fn flip_swaps_two_args() {
    let subtract = fn_value("add(a, mul(b, -1))", &["a", "b"]);
    let flipped = call("flip", &[subtract]);
    assert!(flipped.is_function());
    let result = call_with_eval(
        "apply",
        &[flipped, NonFunctionValue::new(serde_json::json!([10, 3]))],
    );
    assert_eq!(result.as_number(), Some(-7.0));
}

#[test]
fn flip_non_function_errors() {
    let result = call_err("flip", &[n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn flip_single_param_errors() {
    let result = call_err("flip", &[fn_value("add(x, 1)", &["x"])]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn flip_no_args_errors() {
    let result = call_err("flip", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── partial ────────────────────────────────────────────────────────

#[test]
fn partial_binds_first_arg() {
    let add = fn_value("add(a, b)", &["a", "b"]);
    let bound = call(
        "partial",
        &[add, NonFunctionValue::new(serde_json::json!([10]))],
    );
    assert!(bound.is_function());
    let result = call_with_eval(
        "apply",
        &[bound, NonFunctionValue::new(serde_json::json!([5]))],
    );
    assert_eq!(result.as_number(), Some(15.0));
}

#[test]
fn partial_binds_all_args() {
    let add = fn_value("add(a, b)", &["a", "b"]);
    let bound = call(
        "partial",
        &[add, NonFunctionValue::new(serde_json::json!([3, 4]))],
    );
    let result = call_with_eval(
        "apply",
        &[bound, NonFunctionValue::new(serde_json::json!([]))],
    );
    assert_eq!(result.as_number(), Some(7.0));
}

#[test]
fn partial_no_bound_args_returns_original() {
    let add = fn_value("add(a, b)", &["a", "b"]);
    let bound = call(
        "partial",
        &[add, NonFunctionValue::new(serde_json::json!([]))],
    );
    let result = call_with_eval(
        "apply",
        &[bound, NonFunctionValue::new(serde_json::json!([3, 4]))],
    );
    assert_eq!(result.as_number(), Some(7.0));
}

#[test]
fn partial_non_function_errors() {
    let result = call_err(
        "partial",
        &[n(42.0), NonFunctionValue::new(serde_json::json!([1]))],
    );
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn partial_non_array_bound_args_errors() {
    let result = call_err("partial", &[fn_value("add(a, b)", &["a", "b"]), n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn partial_too_few_args_errors() {
    let result = call_err("partial", &[fn_value("add(a, b)", &["a", "b"])]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── when ───────────────────────────────────────────────────────────

#[test]
fn when_applies_when_predicate_true() {
    let result = call_with_eval(
        "when",
        &[
            n(5.0),
            fn_value("add(x, 0)", &["x"]),
            fn_value("mul(x, 100)", &["x"]),
        ],
    );
    assert_eq!(result.as_number(), Some(500.0));
}

#[test]
fn when_returns_value_when_predicate_false() {
    let result = call_with_eval(
        "when",
        &[
            n(0.0),
            fn_value("add(x, 0)", &["x"]),
            fn_value("mul(x, 100)", &["x"]),
        ],
    );
    assert_eq!(result.as_number(), Some(0.0));
}

#[test]
fn when_non_function_predicate_errors() {
    let result = call_err("when", &[n(1.0), n(42.0), fn_value("add(x, 1)", &["x"])]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn when_non_function_transform_errors() {
    let result = call_err("when", &[n(1.0), fn_value("x", &["x"]), n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn when_too_few_args_errors() {
    let result = call_err("when", &[n(1.0), fn_value("x", &["x"])]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── unless ─────────────────────────────────────────────────────────

#[test]
fn unless_applies_when_predicate_false() {
    let result = call_with_eval(
        "unless",
        &[
            n(0.0),
            fn_value("add(x, 0)", &["x"]),
            fn_value("add(x, 100)", &["x"]),
        ],
    );
    assert_eq!(result.as_number(), Some(100.0));
}

#[test]
fn unless_returns_value_when_predicate_true() {
    let result = call_with_eval(
        "unless",
        &[
            n(5.0),
            fn_value("add(x, 0)", &["x"]),
            fn_value("add(x, 100)", &["x"]),
        ],
    );
    assert_eq!(result.as_number(), Some(5.0));
}

#[test]
fn unless_too_few_args_errors() {
    let result = call_err("unless", &[n(1.0), fn_value("x", &["x"])]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── converge ───────────────────────────────────────────────────────

#[test]
fn converge_two_branches() {
    let result = call_with_eval(
        "converge",
        &[
            n(10.0),
            fn_value("add(a, b)", &["a", "b"]),
            fn_value("mul(x, 2)", &["x"]),
            fn_value("mul(x, 3)", &["x"]),
        ],
    );
    assert_eq!(result.as_number(), Some(50.0));
}

#[test]
fn converge_single_branch() {
    let result = call_with_eval(
        "converge",
        &[
            n(10.0),
            fn_value("add(a, b)", &["a", "b"]),
            fn_value("mul(x, 2)", &["x"]),
        ],
    );
    assert_eq!(result.as_number(), Some(20.0));
}

#[test]
fn converge_non_function_combiner_errors() {
    let result = call_err(
        "converge",
        &[n(10.0), n(42.0), fn_value("mul(x, 2)", &["x"])],
    );
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn converge_non_function_branch_errors() {
    let result = call_err(
        "converge",
        &[n(10.0), fn_value("add(a, b)", &["a", "b"]), n(42.0)],
    );
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn converge_too_few_args_errors() {
    let result = call_err("converge", &[n(10.0), fn_value("add(a, b)", &["a", "b"])]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}
