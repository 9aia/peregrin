use ac_runtime::{FunctionValue, NonFunctionValue};
use ac_vm::{Context, Error, Eval};

use crate::shared;

// ── test eval infrastructure ───────────────────────────────────────

use ac_ast::Expr;
use ac_host::DefaultHost;
use ac_vm::Vm;

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
            let mut ctx = Context::new();
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

fn ctx_with_eval() -> Context {
    let mut ctx = Context::new();
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
        "body": body,
    }))
}

fn n(v: f64) -> NonFunctionValue {
    NonFunctionValue::new(serde_json::json!(v))
}

fn s(v: &str) -> NonFunctionValue {
    NonFunctionValue::new(v.to_string())
}

fn b(v: bool) -> NonFunctionValue {
    NonFunctionValue::new(v)
}

fn null() -> NonFunctionValue {
    NonFunctionValue::new(serde_json::Value::Null)
}

fn arr(items: &[serde_json::Value]) -> NonFunctionValue {
    NonFunctionValue::new(serde_json::Value::Array(items.to_vec()))
}

fn obj(pairs: &[(&str, serde_json::Value)]) -> NonFunctionValue {
    let mut map = serde_json::Map::new();
    for (k, v) in pairs {
        map.insert(k.to_string(), v.clone());
    }
    NonFunctionValue::new(serde_json::Value::Object(map))
}

// ════════════════════════════════════════════════════════════════════
// Tier 0: Shared helper tests
// ════════════════════════════════════════════════════════════════════

// ── require_array ──────────────────────────────────────────────────

#[test]
fn require_array_ok() {
    let val = arr(&[serde_json::json!(1), serde_json::json!(2)]);
    let result = shared::require_array(&val);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 2);
}

#[test]
fn require_array_err_on_non_array() {
    let val = n(42.0);
    let result = shared::require_array(&val);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn require_array_err_on_object() {
    let val = obj(&[("a", serde_json::json!(1))]);
    let result = shared::require_array(&val);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── require_object ─────────────────────────────────────────────────

#[test]
fn require_object_ok() {
    let val = obj(&[("a", serde_json::json!(1))]);
    let result = shared::require_object(&val);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 1);
}

#[test]
fn require_object_err_on_non_object() {
    let val = n(42.0);
    let result = shared::require_object(&val);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── require_string ─────────────────────────────────────────────────

#[test]
fn require_string_ok() {
    let val = s("hello");
    let result = shared::require_string(&val);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "hello");
}

#[test]
fn require_string_err_on_non_string() {
    let val = n(42.0);
    let result = shared::require_string(&val);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── require_function_value ─────────────────────────────────────────

#[test]
fn require_function_value_ok() {
    let val = fn_value("add(x, 1)", &["x"]);
    let result = shared::require_function_value(&val);
    assert!(result.is_ok());
    let fv = result.unwrap();
    assert_eq!(fv.body, "add(x, 1)");
}

#[test]
fn require_function_value_err_on_non_function() {
    let val = n(42.0);
    let result = shared::require_function_value(&val);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── require_one_arg ────────────────────────────────────────────────

#[test]
fn require_one_arg_ok() {
    let args = [n(1.0), n(2.0)];
    let result = shared::require_one_arg(&args);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().as_number(), Some(1.0));
}

#[test]
fn require_one_arg_err_on_empty() {
    let args: [NonFunctionValue; 0] = [];
    let result = shared::require_one_arg(&args);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── require_two_args ───────────────────────────────────────────────

#[test]
fn require_two_args_ok() {
    let args = [n(1.0), n(2.0), n(3.0)];
    let (a, b) = shared::require_two_args(&args).unwrap();
    assert_eq!(a.as_number(), Some(1.0));
    assert_eq!(b.as_number(), Some(2.0));
}

#[test]
fn require_two_args_err_on_insufficient() {
    let args = [n(1.0)];
    let result = shared::require_two_args(&args);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── require_three_args ─────────────────────────────────────────────

#[test]
fn require_three_args_ok() {
    let args = [n(1.0), n(2.0), n(3.0), n(4.0)];
    let (a, b, c) = shared::require_three_args(&args).unwrap();
    assert_eq!(a.as_number(), Some(1.0));
    assert_eq!(b.as_number(), Some(2.0));
    assert_eq!(c.as_number(), Some(3.0));
}

#[test]
fn require_three_args_err_on_insufficient() {
    let args = [n(1.0), n(2.0)];
    let result = shared::require_three_args(&args);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── require_one_array ──────────────────────────────────────────────

#[test]
fn require_one_array_ok() {
    let args = [arr(&[serde_json::json!(1)])];
    let result = shared::require_one_array(&args);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 1);
}

#[test]
fn require_one_array_err_on_empty() {
    let args: [NonFunctionValue; 0] = [];
    let result = shared::require_one_array(&args);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── require_two_arrays ─────────────────────────────────────────────

#[test]
fn require_two_arrays_ok() {
    let args = [arr(&[serde_json::json!(1)]), arr(&[serde_json::json!(2)])];
    let (a, b) = shared::require_two_arrays(&args).unwrap();
    assert_eq!(a.len(), 1);
    assert_eq!(b.len(), 1);
}

#[test]
fn require_two_arrays_err_on_insufficient() {
    let args = [arr(&[serde_json::json!(1)])];
    let result = shared::require_two_arrays(&args);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── require_function_and_array ─────────────────────────────────────

#[test]
fn require_function_and_array_ok() {
    let args = [fn_value("x", &["x"]), arr(&[serde_json::json!(1)])];
    let (fv, array) = shared::require_function_and_array(&args).unwrap();
    assert_eq!(fv.body, "x");
    assert_eq!(array.len(), 1);
}

#[test]
fn require_function_and_array_err_on_insufficient() {
    let args = [fn_value("x", &["x"])];
    let result = shared::require_function_and_array(&args);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── require_functions ──────────────────────────────────────────────

#[test]
fn require_functions_ok() {
    let args = [fn_value("x", &["x"]), fn_value("y", &["y"])];
    let result = shared::require_functions(&args);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 2);
}

#[test]
fn require_functions_err_on_empty() {
    let args: [NonFunctionValue; 0] = [];
    let result = shared::require_functions(&args);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn require_functions_err_on_non_function() {
    let args = [fn_value("x", &["x"]), n(42.0)];
    let result = shared::require_functions(&args);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── bind_param ─────────────────────────────────────────────────────

#[test]
fn bind_param_binds_first_param() {
    let fv = FunctionValue {
        params: vec![("x".to_string(), serde_json::Value::Null)],
        return_schema: None,
        body: "add(x, 1)".to_string(),
    };
    let vars = shared::bind_param(&fv, &serde_json::json!(42));
    assert_eq!(vars.len(), 1);
    assert_eq!(vars[0].0, "x");
    assert_eq!(vars[0].1.as_number(), Some(42.0));
}

#[test]
fn bind_param_empty_params_returns_empty() {
    let fv = FunctionValue {
        params: vec![],
        return_schema: None,
        body: "1".to_string(),
    };
    let vars = shared::bind_param(&fv, &serde_json::json!(42));
    assert!(vars.is_empty());
}

// ── bind_two_params ────────────────────────────────────────────────

#[test]
fn bind_two_params_binds_both() {
    let fv = FunctionValue {
        params: vec![
            ("acc".to_string(), serde_json::Value::Null),
            ("item".to_string(), serde_json::Value::Null),
        ],
        return_schema: None,
        body: "add(acc, item)".to_string(),
    };
    let vars = shared::bind_two_params(&fv, &serde_json::json!(10), &serde_json::json!(5));
    assert_eq!(vars.len(), 2);
    assert_eq!(vars[0].0, "acc");
    assert_eq!(vars[0].1.as_number(), Some(10.0));
    assert_eq!(vars[1].0, "item");
    assert_eq!(vars[1].1.as_number(), Some(5.0));
}

#[test]
fn bind_two_params_defaults_when_missing() {
    let fv = FunctionValue {
        params: vec![],
        return_schema: None,
        body: "1".to_string(),
    };
    let vars = shared::bind_two_params(&fv, &serde_json::json!(10), &serde_json::json!(5));
    assert_eq!(vars.len(), 2);
    assert_eq!(vars[0].0, "acc");
    assert_eq!(vars[1].0, "x");
}

// ── bind_all_params ────────────────────────────────────────────────

#[test]
fn bind_all_params_binds_all() {
    let fv = FunctionValue {
        params: vec![
            ("a".to_string(), serde_json::Value::Null),
            ("b".to_string(), serde_json::Value::Null),
        ],
        return_schema: None,
        body: "add(a, b)".to_string(),
    };
    let vars = shared::bind_all_params(&fv, &[serde_json::json!(3), serde_json::json!(4)]);
    assert_eq!(vars.len(), 2);
    assert_eq!(vars[0].0, "a");
    assert_eq!(vars[0].1.as_number(), Some(3.0));
    assert_eq!(vars[1].0, "b");
    assert_eq!(vars[1].1.as_number(), Some(4.0));
}

#[test]
fn bind_all_params_fills_missing_with_null() {
    let fv = FunctionValue {
        params: vec![
            ("a".to_string(), serde_json::Value::Null),
            ("b".to_string(), serde_json::Value::Null),
        ],
        return_schema: None,
        body: "add(a, b)".to_string(),
    };
    let vars = shared::bind_all_params(&fv, &[serde_json::json!(3)]);
    assert_eq!(vars.len(), 2);
    assert_eq!(vars[0].1.as_number(), Some(3.0));
    assert!(vars[1].1.is_null());
}

// ── is_truthy_value ────────────────────────────────────────────────

#[test]
fn is_truthy_bool_true() {
    assert!(shared::is_truthy_value(&b(true)));
}

#[test]
fn is_truthy_bool_false() {
    assert!(!shared::is_truthy_value(&b(false)));
}

#[test]
fn is_truthy_number_nonzero() {
    assert!(shared::is_truthy_value(&n(42.0)));
}

#[test]
fn is_truthy_number_zero() {
    assert!(!shared::is_truthy_value(&n(0.0)));
}

#[test]
fn is_truthy_string_nonempty() {
    assert!(shared::is_truthy_value(&s("hello")));
}

#[test]
fn is_truthy_string_empty() {
    assert!(!shared::is_truthy_value(&s("")));
}

#[test]
fn is_truthy_null_false() {
    assert!(!shared::is_truthy_value(&null()));
}

#[test]
fn is_truthy_array_nonempty() {
    assert!(shared::is_truthy_value(&arr(&[serde_json::json!(1)])));
}

#[test]
fn is_truthy_array_empty() {
    assert!(!shared::is_truthy_value(&arr(&[])));
}

#[test]
fn is_truthy_object_nonempty() {
    assert!(shared::is_truthy_value(&obj(&[("a", serde_json::json!(1))])));
}

#[test]
fn is_truthy_object_empty() {
    assert!(!shared::is_truthy_value(&obj(&[])));
}

// ── constructors: arr, b, num, obj_value ───────────────────────────

#[test]
fn arr_constructor() {
    let val = shared::arr(&[serde_json::json!(1), serde_json::json!(2)]);
    assert_eq!(val.as_array().unwrap().len(), 2);
}

#[test]
fn b_constructor() {
    let val = shared::b(true);
    assert_eq!(val.as_bool(), Some(true));
}

#[test]
fn num_constructor() {
    let val = shared::num(42.0);
    assert_eq!(val.as_number(), Some(42.0));
}

#[test]
fn obj_value_constructor() {
    let mut map = serde_json::Map::new();
    map.insert("key".to_string(), serde_json::json!(42));
    let val = shared::obj_value(map);
    assert_eq!(val.as_object().unwrap().len(), 1);
}

// ── key_to_string ──────────────────────────────────────────────────

#[test]
fn key_to_string_from_string() {
    let val = s("hello");
    assert_eq!(shared::key_to_string(&val), "hello");
}

#[test]
fn key_to_string_from_integer() {
    let val = n(42.0);
    assert_eq!(shared::key_to_string(&val), "42");
}

#[test]
fn key_to_string_from_float() {
    let val = n(3.14);
    assert_eq!(shared::key_to_string(&val), "3.14");
}

#[test]
fn key_to_string_from_bool() {
    let val = b(true);
    assert_eq!(shared::key_to_string(&val), "true");
}

#[test]
fn key_to_string_from_null() {
    let val = null();
    assert_eq!(shared::key_to_string(&val), "null");
}

// ── value_to_string ────────────────────────────────────────────────

#[test]
fn value_to_string_from_string() {
    assert_eq!(
        shared::value_to_string(&serde_json::json!("hello")),
        "hello"
    );
}

#[test]
fn value_to_string_from_integer() {
    assert_eq!(shared::value_to_string(&serde_json::json!(42)), "42");
}

#[test]
fn value_to_string_from_bool() {
    assert_eq!(
        shared::value_to_string(&serde_json::json!(true)),
        "true"
    );
}

#[test]
fn value_to_string_from_null() {
    assert_eq!(shared::value_to_string(&serde_json::Value::Null), "null");
}

// ════════════════════════════════════════════════════════════════════
// Tier 1: Composable primitive tests
// ════════════════════════════════════════════════════════════════════

// ── apply_fn ───────────────────────────────────────────────────────

#[test]
fn apply_fn_evaluates_single_param() {
    let mut ctx = ctx_with_eval();
    let fv = shared::require_function_value(&fn_value("add(x, 10)", &["x"])).unwrap();
    let result = shared::apply_fn(&mut ctx, &fv, &serde_json::json!(5)).unwrap();
    assert_eq!(result.as_number(), Some(15.0));
}

#[test]
fn apply_fn_without_eval_errors() {
    let mut ctx = Context::new();
    let fv = shared::require_function_value(&fn_value("add(x, 10)", &["x"])).unwrap();
    let result = shared::apply_fn(&mut ctx, &fv, &serde_json::json!(5));
    assert!(matches!(result, Err(Error::PrimitiveError(_))));
}

#[test]
fn apply_fn_with_string_value() {
    let mut ctx = ctx_with_eval();
    let fv = shared::require_function_value(&fn_value("x", &["x"])).unwrap();
    let result = shared::apply_fn(&mut ctx, &fv, &serde_json::json!("hello")).unwrap();
    assert_eq!(result.as_str(), Some("hello"));
}

// ── apply_fn2 ──────────────────────────────────────────────────────

#[test]
fn apply_fn2_evaluates_two_params() {
    let mut ctx = ctx_with_eval();
    let fv =
        shared::require_function_value(&fn_value("add(a, b)", &["a", "b"])).unwrap();
    let result = shared::apply_fn2(&mut ctx, &fv, &serde_json::json!(3), &serde_json::json!(4)).unwrap();
    assert_eq!(result.as_number(), Some(7.0));
}

#[test]
fn apply_fn2_defaults_param_names() {
    let mut ctx = ctx_with_eval();
    let fv = FunctionValue {
        params: vec![],
        return_schema: None,
        body: "add(acc, x)".to_string(),
    };
    let result = shared::apply_fn2(&mut ctx, &fv, &serde_json::json!(10), &serde_json::json!(5)).unwrap();
    assert_eq!(result.as_number(), Some(15.0));
}

// ── is_truthy_fn ───────────────────────────────────────────────────

#[test]
fn is_truthy_fn_returns_true_for_truthy_result() {
    let mut ctx = ctx_with_eval();
    let fv = shared::require_function_value(&fn_value("add(x, 1)", &["x"])).unwrap();
    let result = shared::is_truthy_fn(&mut ctx, &fv, &serde_json::json!(5));
    assert!(result.unwrap());
}

#[test]
fn is_truthy_fn_returns_false_for_falsy_result() {
    let mut ctx = ctx_with_eval();
    let fv = shared::require_function_value(&fn_value("mul(x, 0)", &["x"])).unwrap();
    let result = shared::is_truthy_fn(&mut ctx, &fv, &serde_json::json!(5));
    assert!(!result.unwrap());
}

// ── map_iter ───────────────────────────────────────────────────────

#[test]
fn map_iter_doubles_each_element() {
    let mut ctx = ctx_with_eval();
    let fv = shared::require_function_value(&fn_value("add(x, 2)", &["x"])).unwrap();
    let input = vec![serde_json::json!(1), serde_json::json!(2), serde_json::json!(3)];
    let result = shared::map_iter(&mut ctx, &fv, input.iter()).unwrap();
    assert_eq!(result.len(), 3);
    assert_eq!(result[0].as_f64(), Some(3.0));
    assert_eq!(result[1].as_f64(), Some(4.0));
    assert_eq!(result[2].as_f64(), Some(5.0));
}

#[test]
fn map_iter_empty_input() {
    let mut ctx = ctx_with_eval();
    let fv = shared::require_function_value(&fn_value("add(x, 2)", &["x"])).unwrap();
    let input: Vec<serde_json::Value> = vec![];
    let result = shared::map_iter(&mut ctx, &fv, input.iter()).unwrap();
    assert!(result.is_empty());
}

// ── filter_iter ────────────────────────────────────────────────────

#[test]
fn filter_iter_keeps_matching() {
    let mut ctx = ctx_with_eval();
    let fv = shared::require_function_value(&fn_value("x", &["x"])).unwrap();
    let input = vec![
        serde_json::json!(0),
        serde_json::json!(1),
        serde_json::json!(0),
        serde_json::json!(2),
    ];
    let result = shared::filter_iter(&mut ctx, &fv, input.iter()).unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].as_f64(), Some(1.0));
    assert_eq!(result[1].as_f64(), Some(2.0));
}

#[test]
fn filter_iter_empty_input() {
    let mut ctx = ctx_with_eval();
    let fv = shared::require_function_value(&fn_value("x", &["x"])).unwrap();
    let input: Vec<serde_json::Value> = vec![];
    let result = shared::filter_iter(&mut ctx, &fv, input.iter()).unwrap();
    assert!(result.is_empty());
}

// ── fold_iter ──────────────────────────────────────────────────────

#[test]
fn fold_iter_sums_all_elements() {
    let mut ctx = ctx_with_eval();
    let fv =
        shared::require_function_value(&fn_value("add(acc, x)", &["acc", "x"])).unwrap();
    let input = vec![serde_json::json!(1), serde_json::json!(2), serde_json::json!(3)];
    let result = shared::fold_iter(&mut ctx, &fv, input.iter(), serde_json::json!(0)).unwrap();
    assert_eq!(result.as_f64(), Some(6.0));
}

#[test]
fn fold_iter_empty_returns_init() {
    let mut ctx = ctx_with_eval();
    let fv =
        shared::require_function_value(&fn_value("add(acc, x)", &["acc", "x"])).unwrap();
    let input: Vec<serde_json::Value> = vec![];
    let result = shared::fold_iter(&mut ctx, &fv, input.iter(), serde_json::json!(42)).unwrap();
    assert_eq!(result.as_f64(), Some(42.0));
}

// ── find_first ─────────────────────────────────────────────────────

#[test]
fn find_first_finds_matching_element() {
    let mut ctx = ctx_with_eval();
    let fv = shared::require_function_value(&fn_value("x", &["x"])).unwrap();
    let input = vec![serde_json::json!(0), serde_json::json!(0), serde_json::json!(5)];
    let result = shared::find_first(&mut ctx, &fv, input.iter()).unwrap();
    assert_eq!(result.unwrap().as_f64(), Some(5.0));
}

#[test]
fn find_first_returns_none_when_no_match() {
    let mut ctx = ctx_with_eval();
    let fv = shared::require_function_value(&fn_value("mul(x, 0)", &["x"])).unwrap();
    let input = vec![serde_json::json!(1), serde_json::json!(2)];
    let result = shared::find_first(&mut ctx, &fv, input.iter()).unwrap();
    assert!(result.is_none());
}

// ── find_index ─────────────────────────────────────────────────────

#[test]
fn find_index_returns_index_of_match() {
    let mut ctx = ctx_with_eval();
    let fv = shared::require_function_value(&fn_value("x", &["x"])).unwrap();
    let input = vec![serde_json::json!(0), serde_json::json!(0), serde_json::json!(5)];
    let result = shared::find_index(&mut ctx, &fv, input.iter()).unwrap();
    assert_eq!(result, Some(2));
}

#[test]
fn find_index_returns_none_when_no_match() {
    let mut ctx = ctx_with_eval();
    let fv = shared::require_function_value(&fn_value("mul(x, 0)", &["x"])).unwrap();
    let input = vec![serde_json::json!(1), serde_json::json!(2)];
    let result = shared::find_index(&mut ctx, &fv, input.iter()).unwrap();
    assert_eq!(result, None);
}

// ── find_first_rev ─────────────────────────────────────────────────

#[test]
fn find_first_rev_finds_last_matching() {
    let mut ctx = ctx_with_eval();
    let fv = shared::require_function_value(&fn_value("x", &["x"])).unwrap();
    let input = vec![serde_json::json!(5), serde_json::json!(0), serde_json::json!(7)];
    let result = shared::find_first_rev(&mut ctx, &fv, input.iter().rev()).unwrap();
    assert_eq!(result.unwrap().as_f64(), Some(7.0));
}

// ── find_index_rev ─────────────────────────────────────────────────

#[test]
fn find_index_rev_returns_index_from_back() {
    let mut ctx = ctx_with_eval();
    let fv = shared::require_function_value(&fn_value("x", &["x"])).unwrap();
    let input = vec![serde_json::json!(0), serde_json::json!(5), serde_json::json!(0)];
    let result = shared::find_index_rev(&mut ctx, &fv, input.iter().rev()).unwrap();
    assert_eq!(result, Some(1));
}
