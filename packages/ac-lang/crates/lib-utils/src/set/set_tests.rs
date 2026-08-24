use ac_runtime_values::NonFunctionValue;

use ac_runtime::Eval;
use ac_runtime_values::{CanonicalContext, Error};

use ac_runtime_modules::Module;

use serde_json::json;

use super::set::SetModule;

fn call(name: &str, args: &[NonFunctionValue]) -> NonFunctionValue {
    let func = SetModule.function(name).expect("function exists");
    let mut ctx = CanonicalContext::new();
    func.call(args, &mut ctx).expect("call succeeds")
}

fn call_err(name: &str, args: &[NonFunctionValue]) -> Result<NonFunctionValue, Error> {
    let func = SetModule.function(name).expect("function exists");
    let mut ctx = CanonicalContext::new();
    func.call(args, &mut ctx)
}

fn arr(items: &[serde_json::Value]) -> NonFunctionValue {
    NonFunctionValue::new(serde_json::Value::Array(items.to_vec()))
}

fn n(v: f64) -> NonFunctionValue {
    NonFunctionValue::new(serde_json::json!(v))
}

fn ni(v: i64) -> NonFunctionValue {
    NonFunctionValue::new(serde_json::json!(v))
}

fn s(v: &str) -> NonFunctionValue {
    NonFunctionValue::new(v.to_string())
}

// ── callback test infrastructure ─────────────────────────────────────

use crate::shared::TestHost;
use ac_ast_canonical::Expr;
use ac_runtime_canonical::Vm;

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
    let func = SetModule.function(name).expect("function exists");
    let mut ctx = CanonicalContext::new();
    ctx.set_eval(Box::new(TestEval));
    func.call(args, &mut ctx).expect("call succeeds")
}

fn call_err_with_eval(name: &str, args: &[NonFunctionValue]) -> Result<NonFunctionValue, Error> {
    let func = SetModule.function(name).expect("function exists");
    let mut ctx = CanonicalContext::new();
    ctx.set_eval(Box::new(TestEval));
    func.call(args, &mut ctx)
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

// ── module ───────────────────────────────────────────────────────────

#[test]
fn module_name() {
    assert_eq!(SetModule.name(), "set");
}

#[test]
fn unknown_function_returns_none() {
    assert!(SetModule.function("nope").is_none());
}

// ── unique ───────────────────────────────────────────────────────────

#[test]
fn unique_numbers() {
    let result = call(
        "unique",
        &[arr(&[json!(1), json!(2), json!(1), json!(3), json!(2)])],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array[0], json!(1));
    assert_eq!(array[1], json!(2));
    assert_eq!(array[2], json!(3));
}

#[test]
fn unique_strings() {
    let result = call("unique", &[arr(&[json!("a"), json!("b"), json!("a")])]);
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 2);
    assert_eq!(array[0], json!("a"));
    assert_eq!(array[1], json!("b"));
}

#[test]
fn unique_empty() {
    let result = call("unique", &[arr(&[])]);
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
fn unique_preserves_order() {
    let result = call(
        "unique",
        &[arr(&[json!(3), json!(1), json!(3), json!(2), json!(1)])],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array[0], json!(3));
    assert_eq!(array[1], json!(1));
    assert_eq!(array[2], json!(2));
}

#[test]
fn unique_non_array_errors() {
    let result = call_err("unique", &[n(1.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn unique_no_args_errors() {
    let result = call_err("unique", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── union ────────────────────────────────────────────────────────────

#[test]
fn union_basic() {
    let result = call(
        "union",
        &[arr(&[json!(1), json!(2)]), arr(&[json!(2), json!(3)])],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array[0], json!(1));
    assert_eq!(array[1], json!(2));
    assert_eq!(array[2], json!(3));
}

#[test]
fn union_with_duplicates_within_arrays() {
    let result = call(
        "union",
        &[
            arr(&[json!(1), json!(1), json!(2)]),
            arr(&[json!(2), json!(3), json!(3)]),
        ],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
}

#[test]
fn union_empty_first() {
    let result = call("union", &[arr(&[]), arr(&[json!(1), json!(2)])]);
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 2);
}

#[test]
fn union_empty_second() {
    let result = call("union", &[arr(&[json!(1), json!(2)]), arr(&[])]);
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 2);
}

#[test]
fn union_both_empty() {
    let result = call("union", &[arr(&[]), arr(&[])]);
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
fn union_too_few_args_errors() {
    let result = call_err("union", &[arr(&[json!(1)])]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn union_non_array_errors() {
    let result = call_err("union", &[n(1.0), arr(&[json!(1)])]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── intersection ─────────────────────────────────────────────────────

#[test]
fn intersection_basic() {
    let result = call(
        "intersection",
        &[
            arr(&[json!(1), json!(2), json!(3)]),
            arr(&[json!(2), json!(3), json!(4)]),
        ],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 2);
    assert_eq!(array[0], json!(2));
    assert_eq!(array[1], json!(3));
}

#[test]
fn intersection_deduplicates() {
    let result = call(
        "intersection",
        &[
            arr(&[json!(1), json!(1), json!(2)]),
            arr(&[json!(1), json!(2)]),
        ],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 2);
    assert_eq!(array[0], json!(1));
    assert_eq!(array[1], json!(2));
}

#[test]
fn intersection_no_overlap() {
    let result = call(
        "intersection",
        &[arr(&[json!(1), json!(2)]), arr(&[json!(3), json!(4)])],
    );
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
fn intersection_empty_first() {
    let result = call("intersection", &[arr(&[]), arr(&[json!(1), json!(2)])]);
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
fn intersection_too_few_args_errors() {
    let result = call_err("intersection", &[arr(&[json!(1)])]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn intersection_non_array_errors() {
    let result = call_err("intersection", &[n(1.0), arr(&[json!(1)])]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── difference ───────────────────────────────────────────────────────

#[test]
fn difference_basic() {
    let result = call(
        "difference",
        &[arr(&[json!(1), json!(2), json!(3)]), arr(&[json!(2)])],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 2);
    assert_eq!(array[0], json!(1));
    assert_eq!(array[1], json!(3));
}

#[test]
fn difference_deduplicates() {
    let result = call(
        "difference",
        &[arr(&[json!(1), json!(1), json!(2)]), arr(&[json!(2)])],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 1);
    assert_eq!(array[0], json!(1));
}

#[test]
fn difference_no_overlap() {
    let result = call(
        "difference",
        &[arr(&[json!(1), json!(2)]), arr(&[json!(3), json!(4)])],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 2);
}

#[test]
fn difference_empty_second() {
    let result = call("difference", &[arr(&[json!(1), json!(2)]), arr(&[])]);
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 2);
}

#[test]
fn difference_too_few_args_errors() {
    let result = call_err("difference", &[arr(&[json!(1)])]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn difference_non_array_errors() {
    let result = call_err("difference", &[n(1.0), arr(&[json!(1)])]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── symmetric_difference ─────────────────────────────────────────────

#[test]
fn symmetric_difference_basic() {
    let result = call(
        "symmetric_difference",
        &[
            arr(&[json!(1), json!(2), json!(3)]),
            arr(&[json!(2), json!(3), json!(4)]),
        ],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 2);
    assert_eq!(array[0], json!(1));
    assert_eq!(array[1], json!(4));
}

#[test]
fn symmetric_difference_no_overlap() {
    let result = call(
        "symmetric_difference",
        &[arr(&[json!(1), json!(2)]), arr(&[json!(3), json!(4)])],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 4);
}

#[test]
fn symmetric_difference_identical() {
    let result = call(
        "symmetric_difference",
        &[arr(&[json!(1), json!(2)]), arr(&[json!(1), json!(2)])],
    );
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
fn symmetric_difference_empty_first() {
    let result = call(
        "symmetric_difference",
        &[arr(&[]), arr(&[json!(1), json!(2)])],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 2);
}

#[test]
fn symmetric_difference_deduplicates() {
    let result = call(
        "symmetric_difference",
        &[
            arr(&[json!(1), json!(1), json!(2)]),
            arr(&[json!(2), json!(3), json!(3)]),
        ],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 2);
    assert_eq!(array[0], json!(1));
    assert_eq!(array[1], json!(3));
}

#[test]
fn symmetric_difference_too_few_args_errors() {
    let result = call_err("symmetric_difference", &[arr(&[])]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn symmetric_difference_non_array_errors() {
    let result = call_err("symmetric_difference", &[n(1.0), arr(&[json!(1)])]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── without ──────────────────────────────────────────────────────────

#[test]
fn without_single_value() {
    let result = call(
        "without",
        &[arr(&[json!(1), json!(2), json!(3), json!(2)]), ni(2)],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 2);
    assert_eq!(array[0], json!(1));
    assert_eq!(array[1], json!(3));
}

#[test]
fn without_multiple_values() {
    let result = call(
        "without",
        &[arr(&[json!(1), json!(2), json!(3), json!(4)]), ni(2), ni(4)],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 2);
    assert_eq!(array[0], json!(1));
    assert_eq!(array[1], json!(3));
}

#[test]
fn without_no_matches() {
    let result = call("without", &[arr(&[json!(1), json!(2)]), ni(99)]);
    assert_eq!(result.as_array().unwrap().len(), 2);
}

#[test]
fn without_empty_array() {
    let result = call("without", &[arr(&[]), ni(1)]);
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
fn without_no_excludes() {
    let result = call("without", &[arr(&[json!(1), json!(2)])]);
    assert_eq!(result.as_array().unwrap().len(), 2);
}

#[test]
fn without_strings() {
    let result = call(
        "without",
        &[arr(&[json!("a"), json!("b"), json!("c")]), s("b")],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 2);
    assert_eq!(array[0], json!("a"));
    assert_eq!(array[1], json!("c"));
}

#[test]
fn without_deduplicates() {
    let result = call("without", &[arr(&[json!(1), json!(1), json!(2)]), ni(99)]);
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 2);
    assert_eq!(array[0], json!(1));
    assert_eq!(array[1], json!(2));
}

#[test]
fn without_non_array_errors() {
    let result = call_err("without", &[n(1.0), n(2.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── unique_by ────────────────────────────────────────────────────────

#[test]
fn unique_by_identity() {
    let result = call_with_eval(
        "unique_by",
        &[
            arr(&[json!(1), json!(2), json!(1), json!(3), json!(2)]),
            fn_value("add(x, 0)", &["x"]),
        ],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array[0], json!(1));
    assert_eq!(array[1], json!(2));
    assert_eq!(array[2], json!(3));
}

#[test]
fn unique_by_empty() {
    let result = call_with_eval("unique_by", &[arr(&[]), fn_value("x", &["x"])]);
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
fn unique_by_all_same_key() {
    let result = call_with_eval(
        "unique_by",
        &[
            arr(&[json!(1), json!(2), json!(3)]),
            fn_value("mul(x, 0)", &["x"]),
        ],
    );
    assert_eq!(result.as_array().unwrap().len(), 1);
}

#[test]
fn unique_by_non_function_errors() {
    let result = call_err_with_eval("unique_by", &[arr(&[json!(1)]), n(1.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn unique_by_non_array_errors() {
    let result = call_err_with_eval("unique_by", &[n(1.0), fn_value("x", &["x"])]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn unique_by_too_few_args_errors() {
    let result = call_err_with_eval("unique_by", &[arr(&[json!(1)])]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}
