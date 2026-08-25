use handl_vm_values_json::NonFunctionValue;

use handl_vm::Eval;
use handl_vm_values_json::{CanonicalContext, Error};

use handl_vm_common::Module;

use super::module::ArrayModule;

fn call(name: &str, args: &[NonFunctionValue]) -> NonFunctionValue {
    let func = ArrayModule.function(name).expect("function exists");
    let mut ctx = CanonicalContext::new();
    func.call(args, &mut ctx).expect("call succeeds")
}

fn call_err(name: &str, args: &[NonFunctionValue]) -> Result<NonFunctionValue, Error> {
    let func = ArrayModule.function(name).expect("function exists");
    let mut ctx = CanonicalContext::new();
    func.call(args, &mut ctx)
}

fn n(v: f64) -> NonFunctionValue {
    NonFunctionValue::new(serde_json::json!(v))
}

fn s(v: &str) -> NonFunctionValue {
    NonFunctionValue::new(v.to_string())
}

fn arr(items: &[serde_json::Value]) -> NonFunctionValue {
    NonFunctionValue::new(serde_json::Value::Array(items.to_vec()))
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
    let func = ArrayModule.function(name).expect("function exists");
    let mut ctx = CanonicalContext::new();
    ctx.set_eval(Box::new(TestEval));
    func.call(args, &mut ctx).expect("call succeeds")
}

fn call_err_with_eval(name: &str, args: &[NonFunctionValue]) -> Result<NonFunctionValue, Error> {
    let func = ArrayModule.function(name).expect("function exists");
    let mut ctx = CanonicalContext::new();
    ctx.set_eval(Box::new(TestEval));
    func.call(args, &mut ctx)
}

// ── range ──────────────────────────────────────────────────────────

#[test]
fn range_default_step() {
    let result = call("range", &[n(0.0), n(5.0)]);
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 5);
    assert_eq!(arr[0].as_f64(), Some(0.0));
    assert_eq!(arr[4].as_f64(), Some(4.0));
}

#[test]
fn range_custom_step() {
    let result = call("range", &[n(0.0), n(10.0), n(2.0)]);
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 5);
    assert_eq!(arr[0].as_f64(), Some(0.0));
    assert_eq!(arr[4].as_f64(), Some(8.0));
}

#[test]
fn range_negative_step() {
    let result = call("range", &[n(5.0), n(0.0), n(-1.0)]);
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 5);
    assert_eq!(arr[0].as_f64(), Some(5.0));
    assert_eq!(arr[4].as_f64(), Some(1.0));
}

#[test]
fn range_empty() {
    let result = call("range", &[n(5.0), n(5.0)]);
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
fn range_zero_step_errors() {
    let result = call_err("range", &[n(0.0), n(5.0), n(0.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn range_no_args_errors() {
    let result = call_err("range", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn range_one_arg_errors() {
    let result = call_err("range", &[n(5.0)]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn range_non_number_start_errors() {
    let result = call_err("range", &[s("x"), n(5.0), n(1.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn range_non_number_end_errors() {
    let result = call_err("range", &[n(0.0), s("x"), n(1.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn range_non_number_step_errors() {
    let result = call_err("range", &[n(0.0), n(5.0), s("x")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn range_descending() {
    let result = call("range", &[n(5.0), n(0.0), n(-1.0)]);
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 5);
    assert_eq!(arr[0].as_f64(), Some(5.0));
    assert_eq!(arr[4].as_f64(), Some(1.0));
}

#[test]
fn range_start_equals_end() {
    let result = call("range", &[n(5.0), n(5.0), n(1.0)]);
    assert!(result.as_array().unwrap().is_empty());
}

// ── zip ────────────────────────────────────────────────────────────

#[test]
fn zip_equal_length() {
    let result = call(
        "zip",
        &[
            arr(&[serde_json::json!(1), serde_json::json!(2)]),
            arr(&[serde_json::json!("a"), serde_json::json!("b")]),
        ],
    );
    let pairs = result.as_array().unwrap();
    assert_eq!(pairs.len(), 2);
    assert_eq!(pairs[0].as_array().unwrap()[0].as_f64(), Some(1.0));
    assert_eq!(pairs[0].as_array().unwrap()[1].as_str(), Some("a"));
}

#[test]
fn zip_unequal_length_truncates() {
    let result = call(
        "zip",
        &[
            arr(&[
                serde_json::json!(1),
                serde_json::json!(2),
                serde_json::json!(3),
            ]),
            arr(&[serde_json::json!("a")]),
        ],
    );
    assert_eq!(result.as_array().unwrap().len(), 1);
}

#[test]
fn zip_no_args_errors() {
    let result = call_err("zip", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn zip_one_arg_errors() {
    let result = call_err("zip", &[arr(&[serde_json::json!(1)])]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn zip_non_array_first_errors() {
    let result = call_err("zip", &[n(1.0), arr(&[serde_json::json!(1)])]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn zip_non_array_second_errors() {
    let result = call_err("zip", &[arr(&[serde_json::json!(1)]), n(1.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn zip_both_empty() {
    let result = call("zip", &[arr(&[]), arr(&[])]);
    assert!(result.as_array().unwrap().is_empty());
}

// ── chunk ──────────────────────────────────────────────────────────

#[test]
fn chunk_even() {
    let result = call(
        "chunk",
        &[
            arr(&[
                serde_json::json!(1),
                serde_json::json!(2),
                serde_json::json!(3),
                serde_json::json!(4),
            ]),
            n(2.0),
        ],
    );
    let chunks = result.as_array().unwrap();
    assert_eq!(chunks.len(), 2);
    assert_eq!(chunks[0].as_array().unwrap().len(), 2);
    assert_eq!(chunks[1].as_array().unwrap().len(), 2);
}

#[test]
fn chunk_uneven_remainder() {
    let result = call(
        "chunk",
        &[
            arr(&[
                serde_json::json!(1),
                serde_json::json!(2),
                serde_json::json!(3),
            ]),
            n(2.0),
        ],
    );
    let chunks = result.as_array().unwrap();
    assert_eq!(chunks.len(), 2);
    assert_eq!(chunks[0].as_array().unwrap().len(), 2);
    assert_eq!(chunks[1].as_array().unwrap().len(), 1);
}

#[test]
fn chunk_zero_size_errors() {
    let result = call_err("chunk", &[arr(&[serde_json::json!(1)]), n(0.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn chunk_no_args_errors() {
    let result = call_err("chunk", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn chunk_one_arg_errors() {
    let result = call_err("chunk", &[arr(&[serde_json::json!(1)])]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn chunk_non_array_errors() {
    let result = call_err("chunk", &[n(1.0), n(2.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn chunk_non_number_size_errors() {
    let result = call_err("chunk", &[arr(&[serde_json::json!(1)]), s("x")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn chunk_exact_multiple() {
    let result = call(
        "chunk",
        &[
            arr(&[
                serde_json::json!(1),
                serde_json::json!(2),
                serde_json::json!(3),
                serde_json::json!(4),
            ]),
            n(2.0),
        ],
    );
    let chunks = result.as_array().unwrap();
    assert_eq!(chunks.len(), 2);
    assert_eq!(chunks[0].as_array().unwrap().len(), 2);
    assert_eq!(chunks[1].as_array().unwrap().len(), 2);
}

// ── map ────────────────────────────────────────────────────────────

#[test]
fn map_doubles_each_element() {
    let result = call_with_eval(
        "map",
        &[
            fn_value("add(x, 2)", &["x"]),
            arr(&[
                serde_json::json!(1),
                serde_json::json!(2),
                serde_json::json!(3),
            ]),
        ],
    );
    let mapped = result.as_array().unwrap();
    assert_eq!(mapped.len(), 3);
    assert_eq!(mapped[0].as_f64(), Some(3.0));
    assert_eq!(mapped[1].as_f64(), Some(4.0));
    assert_eq!(mapped[2].as_f64(), Some(5.0));
}

#[test]
fn map_empty_array() {
    let result = call_with_eval("map", &[fn_value("add(x, 1)", &["x"]), arr(&[])]);
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
fn map_without_eval_errors() {
    let result = call_err(
        "map",
        &[fn_value("add(x, 1)", &["x"]), arr(&[serde_json::json!(1)])],
    );
    assert!(matches!(result, Err(Error::PrimitiveError(_))));
}

#[test]
fn map_non_function_errors() {
    let result = call_err("map", &[n(42.0), arr(&[serde_json::json!(1)])]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn map_too_few_args_errors() {
    let result = call_err("map", &[fn_value("add(x, 1)", &["x"])]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn map_non_array_errors() {
    let result = call_err("map", &[fn_value("add(x, 1)", &["x"]), n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── filter ─────────────────────────────────────────────────────────

#[test]
fn filter_keeps_matching_elements() {
    let result = call_with_eval(
        "filter",
        &[
            fn_value("add(x, 0)", &["x"]),
            arr(&[
                serde_json::json!(0),
                serde_json::json!(3),
                serde_json::json!(0),
                serde_json::json!(5),
            ]),
        ],
    );
    let filtered = result.as_array().unwrap();
    assert_eq!(filtered.len(), 2);
    assert_eq!(filtered[0].as_f64(), Some(3.0));
    assert_eq!(filtered[1].as_f64(), Some(5.0));
}

#[test]
fn filter_empty_array() {
    let result = call_with_eval("filter", &[fn_value("add(x, 0)", &["x"]), arr(&[])]);
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
fn filter_too_few_args_errors() {
    let result = call_err("filter", &[fn_value("add(x, 0)", &["x"])]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn filter_non_function_errors() {
    let result = call_err("filter", &[n(42.0), arr(&[serde_json::json!(1)])]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn filter_non_array_errors() {
    let result = call_err("filter", &[fn_value("add(x, 0)", &["x"]), n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn filter_without_eval_errors() {
    let result = call_err(
        "filter",
        &[fn_value("add(x, 0)", &["x"]), arr(&[serde_json::json!(1)])],
    );
    assert!(matches!(result, Err(Error::PrimitiveError(_))));
}

// ── reduce ─────────────────────────────────────────────────────────

#[test]
fn reduce_sums_all_elements() {
    let result = call_with_eval(
        "reduce",
        &[
            fn_value("add(acc, x)", &["acc", "x"]),
            arr(&[
                serde_json::json!(1),
                serde_json::json!(2),
                serde_json::json!(3),
                serde_json::json!(4),
            ]),
            n(0.0),
        ],
    );
    assert_eq!(result.as_number(), Some(10.0));
}

#[test]
fn reduce_empty_returns_initial() {
    let result = call_with_eval(
        "reduce",
        &[fn_value("add(acc, x)", &["acc", "x"]), arr(&[]), n(42.0)],
    );
    assert_eq!(result.as_number(), Some(42.0));
}

#[test]
fn reduce_too_few_args_errors() {
    let result = call_err("reduce", &[fn_value("add(acc, x)", &["acc", "x"])]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn reduce_non_function_errors() {
    let result = call_err("reduce", &[n(42.0), arr(&[serde_json::json!(1)]), n(0.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn reduce_non_array_errors() {
    let result = call_err(
        "reduce",
        &[fn_value("add(acc, x)", &["acc", "x"]), n(42.0), n(0.0)],
    );
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn reduce_without_eval_errors() {
    let result = call_err(
        "reduce",
        &[
            fn_value("add(acc, x)", &["acc", "x"]),
            arr(&[serde_json::json!(1)]),
            n(0.0),
        ],
    );
    assert!(matches!(result, Err(Error::PrimitiveError(_))));
}

// ── flat_map ───────────────────────────────────────────────────────

#[test]
fn flat_map_flattens_array_results() {
    let result = call_with_eval(
        "flat_map",
        &[
            fn_value("add(x, 1)", &["x"]),
            arr(&[serde_json::json!(1), serde_json::json!(2)]),
        ],
    );
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0].as_f64(), Some(2.0));
    assert_eq!(arr[1].as_f64(), Some(3.0));
}

#[test]
fn flat_map_empty_array() {
    let result = call_with_eval("flat_map", &[fn_value("add(x, 1)", &["x"]), arr(&[])]);
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
fn flat_map_non_function_errors() {
    let result = call_err("flat_map", &[n(42.0), arr(&[serde_json::json!(1)])]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn flat_map_too_few_args_errors() {
    let result = call_err("flat_map", &[fn_value("add(x, 1)", &["x"])]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn flat_map_non_array_errors() {
    let result = call_err("flat_map", &[fn_value("add(x, 1)", &["x"]), n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn flat_map_without_eval_errors() {
    let result = call_err(
        "flat_map",
        &[fn_value("add(x, 1)", &["x"]), arr(&[serde_json::json!(1)])],
    );
    assert!(matches!(result, Err(Error::PrimitiveError(_))));
}

// ── find ───────────────────────────────────────────────────────────

#[test]
fn find_returns_first_matching() {
    let result = call_with_eval(
        "find",
        &[
            fn_value("add(x, 0)", &["x"]),
            arr(&[
                serde_json::json!(0),
                serde_json::json!(3),
                serde_json::json!(5),
            ]),
        ],
    );
    assert_eq!(result.as_number(), Some(3.0));
}

#[test]
fn find_returns_null_when_no_match() {
    let result = call_with_eval(
        "find",
        &[
            fn_value("add(x, 0)", &["x"]),
            arr(&[serde_json::json!(0), serde_json::json!(0)]),
        ],
    );
    assert!(result.is_null());
}

#[test]
fn find_empty_array_returns_null() {
    let result = call_with_eval("find", &[fn_value("add(x, 0)", &["x"]), arr(&[])]);
    assert!(result.is_null());
}

#[test]
fn find_non_function_errors() {
    let result = call_err("find", &[n(42.0), arr(&[serde_json::json!(1)])]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn find_non_array_errors() {
    let result = call_err("find", &[fn_value("add(x, 0)", &["x"]), n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn find_without_eval_errors() {
    let result = call_err(
        "find",
        &[fn_value("add(x, 0)", &["x"]), arr(&[serde_json::json!(1)])],
    );
    assert!(matches!(result, Err(Error::PrimitiveError(_))));
}

#[test]
fn find_too_few_args_errors() {
    let result = call_err("find", &[fn_value("add(x, 0)", &["x"])]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── some / every ───────────────────────────────────────────────────

#[test]
fn some_returns_true_when_any_matches() {
    let result = call_with_eval(
        "some",
        &[
            fn_value("add(x, 0)", &["x"]),
            arr(&[serde_json::json!(0), serde_json::json!(3)]),
        ],
    );
    assert_eq!(result.as_bool(), Some(true));
}

#[test]
fn some_returns_false_when_none_match() {
    let result = call_with_eval(
        "some",
        &[
            fn_value("add(x, 0)", &["x"]),
            arr(&[serde_json::json!(0), serde_json::json!(0)]),
        ],
    );
    assert_eq!(result.as_bool(), Some(false));
}

#[test]
fn some_empty_array_returns_false() {
    let result = call_with_eval("some", &[fn_value("add(x, 0)", &["x"]), arr(&[])]);
    assert_eq!(result.as_bool(), Some(false));
}

#[test]
fn every_returns_true_when_all_match() {
    let result = call_with_eval(
        "every",
        &[
            fn_value("add(x, 0)", &["x"]),
            arr(&[serde_json::json!(3), serde_json::json!(5)]),
        ],
    );
    assert_eq!(result.as_bool(), Some(true));
}

#[test]
fn every_returns_false_when_one_fails() {
    let result = call_with_eval(
        "every",
        &[
            fn_value("add(x, 0)", &["x"]),
            arr(&[serde_json::json!(3), serde_json::json!(0)]),
        ],
    );
    assert_eq!(result.as_bool(), Some(false));
}

#[test]
fn every_empty_array_returns_true() {
    let result = call_with_eval("every", &[fn_value("add(x, 0)", &["x"]), arr(&[])]);
    assert_eq!(result.as_bool(), Some(true));
}

#[test]
fn some_non_function_errors() {
    let result = call_err("some", &[n(42.0), arr(&[serde_json::json!(1)])]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn some_non_array_errors() {
    let result = call_err("some", &[fn_value("add(x, 0)", &["x"]), n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn some_without_eval_errors() {
    let result = call_err(
        "some",
        &[fn_value("add(x, 0)", &["x"]), arr(&[serde_json::json!(1)])],
    );
    assert!(matches!(result, Err(Error::PrimitiveError(_))));
}

#[test]
fn some_too_few_args_errors() {
    let result = call_err("some", &[fn_value("add(x, 0)", &["x"])]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn every_non_function_errors() {
    let result = call_err("every", &[n(42.0), arr(&[serde_json::json!(1)])]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn every_non_array_errors() {
    let result = call_err("every", &[fn_value("add(x, 0)", &["x"]), n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn every_without_eval_errors() {
    let result = call_err(
        "every",
        &[fn_value("add(x, 0)", &["x"]), arr(&[serde_json::json!(1)])],
    );
    assert!(matches!(result, Err(Error::PrimitiveError(_))));
}

#[test]
fn every_too_few_args_errors() {
    let result = call_err("every", &[fn_value("add(x, 0)", &["x"])]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── group_by ───────────────────────────────────────────────────────

#[test]
fn group_by_groups_by_key() {
    let result = call_with_eval(
        "group_by",
        &[
            fn_value("x", &["x"]),
            arr(&[
                serde_json::json!(1),
                serde_json::json!(2),
                serde_json::json!(1),
                serde_json::json!(3),
            ]),
        ],
    );
    let obj = result.as_object().unwrap();
    assert_eq!(obj.len(), 3);
    assert_eq!(obj.get("1").unwrap().as_array().unwrap().len(), 2);
    assert_eq!(obj.get("2").unwrap().as_array().unwrap().len(), 1);
    assert_eq!(obj.get("3").unwrap().as_array().unwrap().len(), 1);
}

#[test]
fn group_by_string_keys() {
    let result = call_with_eval(
        "group_by",
        &[
            fn_value("x", &["x"]),
            arr(&[
                serde_json::json!("a"),
                serde_json::json!("b"),
                serde_json::json!("a"),
            ]),
        ],
    );
    let obj = result.as_object().unwrap();
    assert_eq!(obj.len(), 2);
    assert_eq!(obj.get("a").unwrap().as_array().unwrap().len(), 2);
    assert_eq!(obj.get("b").unwrap().as_array().unwrap().len(), 1);
}

#[test]
fn group_by_empty_array() {
    let result = call_with_eval("group_by", &[fn_value("x", &["x"]), arr(&[])]);
    assert!(result.as_object().unwrap().is_empty());
}

#[test]
fn group_by_non_function_errors() {
    let result = call_err("group_by", &[n(42.0), arr(&[serde_json::json!(1)])]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn group_by_non_array_errors() {
    let result = call_err("group_by", &[fn_value("x", &["x"]), n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn group_by_without_eval_errors() {
    let result = call_err(
        "group_by",
        &[fn_value("x", &["x"]), arr(&[serde_json::json!(1)])],
    );
    assert!(matches!(result, Err(Error::PrimitiveError(_))));
}

#[test]
fn group_by_too_few_args_errors() {
    let result = call_err("group_by", &[fn_value("x", &["x"])]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── partition ──────────────────────────────────────────────────────

#[test]
fn partition_splits_matching_and_rest() {
    let result = call_with_eval(
        "partition",
        &[
            fn_value("add(x, 0)", &["x"]),
            arr(&[
                serde_json::json!(0),
                serde_json::json!(3),
                serde_json::json!(0),
                serde_json::json!(5),
            ]),
        ],
    );
    let outer = result.as_array().unwrap();
    assert_eq!(outer.len(), 2);
    let matching = outer[0].as_array().unwrap();
    let rest = outer[1].as_array().unwrap();
    assert_eq!(matching.len(), 2);
    assert_eq!(matching[0].as_f64(), Some(3.0));
    assert_eq!(matching[1].as_f64(), Some(5.0));
    assert_eq!(rest.len(), 2);
    assert_eq!(rest[0].as_f64(), Some(0.0));
    assert_eq!(rest[1].as_f64(), Some(0.0));
}

#[test]
fn partition_all_match() {
    let result = call_with_eval(
        "partition",
        &[
            fn_value("add(x, 0)", &["x"]),
            arr(&[serde_json::json!(1), serde_json::json!(2)]),
        ],
    );
    let outer = result.as_array().unwrap();
    assert_eq!(outer[0].as_array().unwrap().len(), 2);
    assert_eq!(outer[1].as_array().unwrap().len(), 0);
}

#[test]
fn partition_empty_array() {
    let result = call_with_eval("partition", &[fn_value("add(x, 0)", &["x"]), arr(&[])]);
    let outer = result.as_array().unwrap();
    assert_eq!(outer[0].as_array().unwrap().len(), 0);
    assert_eq!(outer[1].as_array().unwrap().len(), 0);
}

#[test]
fn partition_none_match() {
    let result = call_with_eval(
        "partition",
        &[
            fn_value("add(x, 0)", &["x"]),
            arr(&[serde_json::json!(0), serde_json::json!(0)]),
        ],
    );
    let outer = result.as_array().unwrap();
    assert_eq!(outer[0].as_array().unwrap().len(), 0);
    assert_eq!(outer[1].as_array().unwrap().len(), 2);
}

#[test]
fn partition_non_function_errors() {
    let result = call_err("partition", &[n(42.0), arr(&[serde_json::json!(1)])]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn partition_non_array_errors() {
    let result = call_err("partition", &[fn_value("add(x, 0)", &["x"]), n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn partition_without_eval_errors() {
    let result = call_err(
        "partition",
        &[fn_value("add(x, 0)", &["x"]), arr(&[serde_json::json!(1)])],
    );
    assert!(matches!(result, Err(Error::PrimitiveError(_))));
}

#[test]
fn partition_too_few_args_errors() {
    let result = call_err("partition", &[fn_value("add(x, 0)", &["x"])]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── sort_by ────────────────────────────────────────────────────────

#[test]
fn sort_by_sorts_ascending() {
    let result = call_with_eval(
        "sort_by",
        &[
            fn_value("x", &["x"]),
            arr(&[
                serde_json::json!(3),
                serde_json::json!(1),
                serde_json::json!(2),
            ]),
        ],
    );
    let arr = result.as_array().unwrap();
    assert_eq!(arr[0].as_f64(), Some(1.0));
    assert_eq!(arr[1].as_f64(), Some(2.0));
    assert_eq!(arr[2].as_f64(), Some(3.0));
}

#[test]
fn sort_by_with_key_function() {
    let result = call_with_eval(
        "sort_by",
        &[
            fn_value("mul(x, -1)", &["x"]),
            arr(&[
                serde_json::json!(1),
                serde_json::json!(3),
                serde_json::json!(2),
            ]),
        ],
    );
    let arr = result.as_array().unwrap();
    assert_eq!(arr[0].as_f64(), Some(3.0));
    assert_eq!(arr[1].as_f64(), Some(2.0));
    assert_eq!(arr[2].as_f64(), Some(1.0));
}

#[test]
fn sort_by_empty_array() {
    let result = call_with_eval("sort_by", &[fn_value("x", &["x"]), arr(&[])]);
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
fn sort_by_non_number_key_errors() {
    let result = call_err_with_eval(
        "sort_by",
        &[
            fn_value("x", &["x"]),
            arr(&[serde_json::json!("a"), serde_json::json!("b")]),
        ],
    );
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn sort_by_non_function_errors() {
    let result = call_err("sort_by", &[n(42.0), arr(&[serde_json::json!(1)])]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn sort_by_non_array_errors() {
    let result = call_err("sort_by", &[fn_value("x", &["x"]), n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn sort_by_without_eval_errors() {
    let result = call_err(
        "sort_by",
        &[fn_value("x", &["x"]), arr(&[serde_json::json!(1)])],
    );
    assert!(matches!(result, Err(Error::PrimitiveError(_))));
}

#[test]
fn sort_by_too_few_args_errors() {
    let result = call_err("sort_by", &[fn_value("x", &["x"])]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn sort_by_single_element() {
    let result = call_with_eval(
        "sort_by",
        &[fn_value("x", &["x"]), arr(&[serde_json::json!(42)])],
    );
    assert_eq!(result.as_array().unwrap().len(), 1);
}

// ── sort_by_desc ───────────────────────────────────────────────────

#[test]
fn sort_by_desc_sorts_descending() {
    let result = call_with_eval(
        "sort_by_desc",
        &[
            fn_value("x", &["x"]),
            arr(&[
                serde_json::json!(1),
                serde_json::json!(3),
                serde_json::json!(2),
            ]),
        ],
    );
    let arr = result.as_array().unwrap();
    assert_eq!(arr[0].as_f64(), Some(3.0));
    assert_eq!(arr[1].as_f64(), Some(2.0));
    assert_eq!(arr[2].as_f64(), Some(1.0));
}

#[test]
fn sort_by_desc_with_key_function() {
    let result = call_with_eval(
        "sort_by_desc",
        &[
            fn_value("mul(x, -1)", &["x"]),
            arr(&[
                serde_json::json!(1),
                serde_json::json!(3),
                serde_json::json!(2),
            ]),
        ],
    );
    let arr = result.as_array().unwrap();
    assert_eq!(arr[0].as_f64(), Some(1.0));
    assert_eq!(arr[1].as_f64(), Some(2.0));
    assert_eq!(arr[2].as_f64(), Some(3.0));
}

#[test]
fn sort_by_desc_empty_array() {
    let result = call_with_eval("sort_by_desc", &[fn_value("x", &["x"]), arr(&[])]);
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
fn sort_by_desc_non_function_errors() {
    let result = call_err("sort_by_desc", &[n(42.0), arr(&[serde_json::json!(1)])]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn sort_by_desc_non_array_errors() {
    let result = call_err("sort_by_desc", &[fn_value("x", &["x"]), n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn sort_by_desc_without_eval_errors() {
    let result = call_err(
        "sort_by_desc",
        &[fn_value("x", &["x"]), arr(&[serde_json::json!(1)])],
    );
    assert!(matches!(result, Err(Error::PrimitiveError(_))));
}

#[test]
fn sort_by_desc_too_few_args_errors() {
    let result = call_err("sort_by_desc", &[fn_value("x", &["x"])]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn sort_by_desc_non_number_key_errors() {
    let result = call_err_with_eval(
        "sort_by_desc",
        &[fn_value("x", &["x"]), arr(&[serde_json::json!("a")])],
    );
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn sort_by_desc_single_element() {
    let result = call_with_eval(
        "sort_by_desc",
        &[fn_value("x", &["x"]), arr(&[serde_json::json!(42)])],
    );
    assert_eq!(result.as_array().unwrap().len(), 1);
}

// ── compact ────────────────────────────────────────────────────────

#[test]
fn compact_removes_nulls() {
    let result = call(
        "compact",
        &[arr(&[
            serde_json::json!(1),
            serde_json::Value::Null,
            serde_json::json!(3),
            serde_json::Value::Null,
        ])],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 2);
    assert_eq!(array[0].as_f64(), Some(1.0));
    assert_eq!(array[1].as_f64(), Some(3.0));
}

#[test]
fn compact_empty_array() {
    let result = call("compact", &[arr(&[])]);
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
fn compact_no_nulls() {
    let result = call(
        "compact",
        &[arr(&[serde_json::json!(1), serde_json::json!(2)])],
    );
    assert_eq!(result.as_array().unwrap().len(), 2);
}

#[test]
fn compact_on_non_array_errors() {
    let result = call_err("compact", &[n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn compact_no_args_errors() {
    let result = call_err("compact", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn compact_all_nulls() {
    let result = call(
        "compact",
        &[arr(&[serde_json::Value::Null, serde_json::Value::Null])],
    );
    assert!(result.as_array().unwrap().is_empty());
}

// ── pluck ──────────────────────────────────────────────────────────

#[test]
fn pluck_extracts_property() {
    let result = call(
        "pluck",
        &[
            arr(&[
                serde_json::json!({"name": "a", "age": 1}),
                serde_json::json!({"name": "b", "age": 2}),
            ]),
            s("name"),
        ],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 2);
    assert_eq!(array[0].as_str(), Some("a"));
    assert_eq!(array[1].as_str(), Some("b"));
}

#[test]
fn pluck_missing_key_returns_null() {
    let result = call("pluck", &[arr(&[serde_json::json!({"a": 1})]), s("b")]);
    assert!(result.as_array().unwrap()[0].is_null());
}

#[test]
fn pluck_on_non_array_errors() {
    let result = call_err("pluck", &[n(42.0), s("a")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn pluck_too_few_args_errors() {
    let result = call_err("pluck", &[arr(&[serde_json::json!(1)])]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn pluck_non_string_key_errors() {
    let result = call_err("pluck", &[arr(&[serde_json::json!(1)]), n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn pluck_empty_array() {
    let result = call("pluck", &[arr(&[]), s("name")]);
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
fn pluck_on_non_object_element_returns_null() {
    let result = call("pluck", &[arr(&[serde_json::json!(42)]), s("name")]);
    assert!(result.as_array().unwrap()[0].is_null());
}

// ── min_by / max_by ────────────────────────────────────────────────

#[test]
fn min_by_returns_element_with_min_key() {
    let result = call_with_eval(
        "min_by",
        &[
            fn_value("x", &["x"]),
            arr(&[
                serde_json::json!(3),
                serde_json::json!(1),
                serde_json::json!(2),
            ]),
        ],
    );
    assert_eq!(result.as_number(), Some(1.0));
}

#[test]
fn min_by_empty_returns_null() {
    let result = call_with_eval("min_by", &[fn_value("x", &["x"]), arr(&[])]);
    assert!(result.is_null());
}

#[test]
fn max_by_returns_element_with_max_key() {
    let result = call_with_eval(
        "max_by",
        &[
            fn_value("x", &["x"]),
            arr(&[
                serde_json::json!(3),
                serde_json::json!(1),
                serde_json::json!(2),
            ]),
        ],
    );
    assert_eq!(result.as_number(), Some(3.0));
}

#[test]
fn max_by_empty_returns_null() {
    let result = call_with_eval("max_by", &[fn_value("x", &["x"]), arr(&[])]);
    assert!(result.is_null());
}

#[test]
fn min_by_non_function_errors() {
    let result = call_err("min_by", &[n(42.0), arr(&[serde_json::json!(1)])]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn min_by_non_array_errors() {
    let result = call_err("min_by", &[fn_value("x", &["x"]), n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn min_by_without_eval_errors() {
    let result = call_err(
        "min_by",
        &[fn_value("x", &["x"]), arr(&[serde_json::json!(1)])],
    );
    assert!(matches!(result, Err(Error::PrimitiveError(_))));
}

#[test]
fn min_by_too_few_args_errors() {
    let result = call_err("min_by", &[fn_value("x", &["x"])]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn min_by_non_number_key_errors() {
    let result = call_err_with_eval(
        "min_by",
        &[fn_value("x", &["x"]), arr(&[serde_json::json!("a")])],
    );
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn min_by_single_element() {
    let result = call_with_eval(
        "min_by",
        &[fn_value("x", &["x"]), arr(&[serde_json::json!(42)])],
    );
    assert_eq!(result.as_number(), Some(42.0));
}

#[test]
fn max_by_non_function_errors() {
    let result = call_err("max_by", &[n(42.0), arr(&[serde_json::json!(1)])]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn max_by_non_array_errors() {
    let result = call_err("max_by", &[fn_value("x", &["x"]), n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn max_by_without_eval_errors() {
    let result = call_err(
        "max_by",
        &[fn_value("x", &["x"]), arr(&[serde_json::json!(1)])],
    );
    assert!(matches!(result, Err(Error::PrimitiveError(_))));
}

#[test]
fn max_by_too_few_args_errors() {
    let result = call_err("max_by", &[fn_value("x", &["x"])]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn max_by_non_number_key_errors() {
    let result = call_err_with_eval(
        "max_by",
        &[fn_value("x", &["x"]), arr(&[serde_json::json!("a")])],
    );
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn max_by_single_element() {
    let result = call_with_eval(
        "max_by",
        &[fn_value("x", &["x"]), arr(&[serde_json::json!(42)])],
    );
    assert_eq!(result.as_number(), Some(42.0));
}

// ── count_by ───────────────────────────────────────────────────────

#[test]
fn count_by_groups_and_counts() {
    let result = call_with_eval(
        "count_by",
        &[
            fn_value("x", &["x"]),
            arr(&[
                serde_json::json!(1),
                serde_json::json!(2),
                serde_json::json!(1),
                serde_json::json!(3),
                serde_json::json!(1),
            ]),
        ],
    );
    let obj = result.as_object().unwrap();
    assert_eq!(obj.len(), 3);
    assert_eq!(obj.get("1").unwrap().as_f64(), Some(3.0));
    assert_eq!(obj.get("2").unwrap().as_f64(), Some(1.0));
    assert_eq!(obj.get("3").unwrap().as_f64(), Some(1.0));
}

#[test]
fn count_by_empty_array() {
    let result = call_with_eval("count_by", &[fn_value("x", &["x"]), arr(&[])]);
    assert!(result.as_object().unwrap().is_empty());
}

#[test]
fn count_by_non_function_errors() {
    let result = call_err("count_by", &[n(42.0), arr(&[serde_json::json!(1)])]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn count_by_non_array_errors() {
    let result = call_err("count_by", &[fn_value("x", &["x"]), n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn count_by_without_eval_errors() {
    let result = call_err(
        "count_by",
        &[fn_value("x", &["x"]), arr(&[serde_json::json!(1)])],
    );
    assert!(matches!(result, Err(Error::PrimitiveError(_))));
}

#[test]
fn count_by_too_few_args_errors() {
    let result = call_err("count_by", &[fn_value("x", &["x"])]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn count_by_string_keys() {
    let result = call_with_eval(
        "count_by",
        &[
            fn_value("x", &["x"]),
            arr(&[
                serde_json::json!("a"),
                serde_json::json!("b"),
                serde_json::json!("a"),
            ]),
        ],
    );
    let obj = result.as_object().unwrap();
    assert_eq!(obj.len(), 2);
    assert_eq!(obj.get("a").unwrap().as_f64(), Some(2.0));
    assert_eq!(obj.get("b").unwrap().as_f64(), Some(1.0));
}
