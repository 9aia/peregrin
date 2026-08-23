use ac_runtime::NonFunctionValue;
use ac_runtime_core::Module;
use ac_vm::{Context, Error, Eval};

use serde_json::json;

use super::module::ArrayModule;

fn call(module: &dyn Module, name: &str, args: &[NonFunctionValue]) -> NonFunctionValue {
    let func = module.function(name).expect("function exists");
    let mut ctx = Context::new();
    func.call(args, &mut ctx).expect("call succeeds")
}

fn call_err(
    module: &dyn Module,
    name: &str,
    args: &[NonFunctionValue],
) -> Result<NonFunctionValue, Error> {
    let func = module.function(name).expect("function exists");
    let mut ctx = Context::new();
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

#[test]
fn module_name() {
    assert_eq!(ArrayModule.name(), "array");
}

#[test]
fn length() {
    assert_eq!(
        call(
            &ArrayModule,
            "length",
            &[arr(&[json!(1), json!(2), json!(3)])]
        )
        .as_number(),
        Some(3.0)
    );
}

#[test]
fn length_empty() {
    assert_eq!(
        call(&ArrayModule, "length", &[arr(&[])]).as_number(),
        Some(0.0)
    );
}

#[test]
fn reverse() {
    let result = call(
        &ArrayModule,
        "reverse",
        &[arr(&[json!(1), json!(2), json!(3)])],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array[0], json!(3));
    assert_eq!(array[1], json!(2));
    assert_eq!(array[2], json!(1));
}

#[test]
fn concat() {
    let result = call(
        &ArrayModule,
        "concat",
        &[arr(&[json!(1), json!(2)]), arr(&[json!(3), json!(4)])],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 4);
    assert_eq!(array[0], json!(1));
    assert_eq!(array[3], json!(4));
}

#[test]
fn contains_true() {
    assert_eq!(
        call(
            &ArrayModule,
            "contains",
            &[arr(&[json!(1), json!(2), json!(3)]), ni(2)]
        )
        .as_bool(),
        Some(true)
    );
}

#[test]
fn contains_false() {
    assert_eq!(
        call(
            &ArrayModule,
            "contains",
            &[arr(&[json!(1), json!(2), json!(3)]), ni(5)]
        )
        .as_bool(),
        Some(false)
    );
}

#[test]
fn contains_string() {
    assert_eq!(
        call(
            &ArrayModule,
            "contains",
            &[arr(&[json!("a"), json!("b")]), s("b")]
        )
        .as_bool(),
        Some(true)
    );
}

#[test]
fn index_of_found() {
    assert_eq!(
        call(
            &ArrayModule,
            "index_of",
            &[arr(&[json!(10), json!(20), json!(30)]), ni(20)]
        )
        .as_number(),
        Some(1.0)
    );
}

#[test]
fn index_of_not_found() {
    assert_eq!(
        call(
            &ArrayModule,
            "index_of",
            &[arr(&[json!(10), json!(20)]), ni(99)]
        )
        .as_number(),
        Some(-1.0)
    );
}

#[test]
fn slice() {
    let result = call(
        &ArrayModule,
        "slice",
        &[
            arr(&[json!(1), json!(2), json!(3), json!(4), json!(5)]),
            n(1.0),
            n(4.0),
        ],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array[0], json!(2));
    assert_eq!(array[2], json!(4));
}

#[test]
fn slice_out_of_bounds_clamps() {
    let result = call(
        &ArrayModule,
        "slice",
        &[arr(&[json!(1), json!(2)]), n(0.0), n(10.0)],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 2);
}

#[test]
fn first() {
    let result = call(
        &ArrayModule,
        "first",
        &[arr(&[json!(1), json!(2), json!(3)])],
    );
    assert_eq!(result.as_number(), Some(1.0));
}

#[test]
fn first_empty() {
    let result = call(&ArrayModule, "first", &[arr(&[])]);
    assert!(result.as_number().is_none());
    assert!(result.as_str().is_none());
}

#[test]
fn last() {
    let result = call(
        &ArrayModule,
        "last",
        &[arr(&[json!(1), json!(2), json!(3)])],
    );
    assert_eq!(result.as_number(), Some(3.0));
}

#[test]
fn last_empty() {
    let result = call(&ArrayModule, "last", &[arr(&[])]);
    assert!(result.as_number().is_none());
    assert!(result.as_str().is_none());
}

#[test]
fn sort_numbers() {
    let result = call(
        &ArrayModule,
        "sort",
        &[arr(&[json!(3), json!(1), json!(2)])],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array[0], json!(1));
    assert_eq!(array[1], json!(2));
    assert_eq!(array[2], json!(3));
}

#[test]
fn sort_strings() {
    let result = call(
        &ArrayModule,
        "sort",
        &[arr(&[json!("c"), json!("a"), json!("b")])],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array[0], json!("a"));
    assert_eq!(array[1], json!("b"));
    assert_eq!(array[2], json!("c"));
}

#[test]
fn sort_mixed_numbers_before_strings() {
    let result = call(
        &ArrayModule,
        "sort",
        &[arr(&[json!("b"), json!(2), json!("a"), json!(1)])],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array[0], json!(1));
    assert_eq!(array[1], json!(2));
    assert_eq!(array[2], json!("a"));
    assert_eq!(array[3], json!("b"));
}

#[test]
fn flatten() {
    let result = call(
        &ArrayModule,
        "flatten",
        &[arr(&[json!(1), json!([2, 3]), json!(4)])],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 4);
    assert_eq!(array[0], json!(1));
    assert_eq!(array[1], json!(2));
    assert_eq!(array[2], json!(3));
    assert_eq!(array[3], json!(4));
}

#[test]
fn flatten_no_nested() {
    let result = call(
        &ArrayModule,
        "flatten",
        &[arr(&[json!(1), json!(2), json!(3)])],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
}

#[test]
fn sum() {
    assert_eq!(
        call(
            &ArrayModule,
            "sum",
            &[arr(&[json!(1), json!(2), json!(3), json!(4)])]
        )
        .as_number(),
        Some(10.0)
    );
}

#[test]
fn sum_empty() {
    assert_eq!(
        call(&ArrayModule, "sum", &[arr(&[])]).as_number(),
        Some(0.0)
    );
}

#[test]
fn sum_mixed_types() {
    assert_eq!(
        call(
            &ArrayModule,
            "sum",
            &[arr(&[json!(1), json!("ignore"), json!(2)])]
        )
        .as_number(),
        Some(3.0)
    );
}

#[test]
fn avg() {
    assert_eq!(
        call(&ArrayModule, "avg", &[arr(&[json!(1), json!(2), json!(3)])]).as_number(),
        Some(2.0)
    );
}

#[test]
fn avg_empty() {
    assert_eq!(
        call(&ArrayModule, "avg", &[arr(&[])]).as_number(),
        Some(0.0)
    );
}

#[test]
fn min() {
    assert_eq!(
        call(&ArrayModule, "min", &[arr(&[json!(3), json!(1), json!(2)])]).as_number(),
        Some(1.0)
    );
}

#[test]
fn min_empty() {
    let result = call(&ArrayModule, "min", &[arr(&[])]);
    assert!(result.as_number().is_none());
}

#[test]
fn max() {
    assert_eq!(
        call(&ArrayModule, "max", &[arr(&[json!(3), json!(1), json!(2)])]).as_number(),
        Some(3.0)
    );
}

#[test]
fn max_empty() {
    let result = call(&ArrayModule, "max", &[arr(&[])]);
    assert!(result.as_number().is_none());
}

#[test]
fn at_valid_index() {
    let result = call(
        &ArrayModule,
        "at",
        &[arr(&[json!(10), json!(20), json!(30)]), n(1.0)],
    );
    assert_eq!(result.as_number(), Some(20.0));
}

#[test]
fn at_out_of_bounds_returns_null() {
    let result = call(&ArrayModule, "at", &[arr(&[json!(1)]), n(5.0)]);
    assert!(result.is_null());
}

#[test]
fn at_negative_errors() {
    let func = ArrayModule.function("at").unwrap();
    let mut ctx = Context::new();
    let result = func.call(&[arr(&[json!(1)]), n(-1.0)], &mut ctx);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn push_single() {
    let result = call(&ArrayModule, "push", &[arr(&[json!(1), json!(2)]), n(3.0)]);
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array[2].as_f64(), Some(3.0));
}

#[test]
fn push_multiple() {
    let result = call(
        &ArrayModule,
        "push",
        &[arr(&[json!(1)]), n(2.0), n(3.0), s("a")],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 4);
    assert_eq!(array[3], json!("a"));
}

#[test]
fn fill_three() {
    let result = call(&ArrayModule, "fill", &[n(3.0), n(0.0)]);
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array[0].as_f64(), Some(0.0));
    assert_eq!(array[2].as_f64(), Some(0.0));
}

#[test]
fn fill_zero() {
    let result = call(&ArrayModule, "fill", &[n(0.0), s("x")]);
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
fn fill_negative_errors() {
    let func = ArrayModule.function("fill").unwrap();
    let mut ctx = Context::new();
    let result = func.call(&[n(-1.0), n(0.0)], &mut ctx);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn deep_flatten_nested() {
    let result = call(
        &ArrayModule,
        "deep_flatten",
        &[arr(&[json!(1), json!([2, json!([3, 4])]), json!(5)])],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 5);
    assert_eq!(array[0], json!(1));
    assert_eq!(array[4], json!(5));
}

#[test]
fn deep_flatten_no_nested() {
    let result = call(
        &ArrayModule,
        "deep_flatten",
        &[arr(&[json!(1), json!(2), json!(3)])],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
}

#[test]
fn slice_negative_errors() {
    let func = ArrayModule.function("slice").unwrap();
    let mut ctx = Context::new();
    let result = func.call(&[arr(&[json!(1)]), n(-1.0), n(1.0)], &mut ctx);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn pop_removes_last() {
    let result = call(&ArrayModule, "pop", &[arr(&[json!(1), json!(2), json!(3)])]);
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 2);
    assert_eq!(array[0].as_f64(), Some(1.0));
    assert_eq!(array[1].as_f64(), Some(2.0));
}

#[test]
fn pop_empty_returns_empty() {
    let result = call(&ArrayModule, "pop", &[arr(&[])]);
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
fn pop_single_element() {
    let result = call(&ArrayModule, "pop", &[arr(&[json!(42)])]);
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
fn shift_removes_first() {
    let result = call(
        &ArrayModule,
        "shift",
        &[arr(&[json!(1), json!(2), json!(3)])],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 2);
    assert_eq!(array[0].as_f64(), Some(2.0));
    assert_eq!(array[1].as_f64(), Some(3.0));
}

#[test]
fn shift_empty_returns_empty() {
    let result = call(&ArrayModule, "shift", &[arr(&[])]);
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
fn shift_single_element() {
    let result = call(&ArrayModule, "shift", &[arr(&[json!(42)])]);
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
fn unknown_function_returns_none() {
    assert!(ArrayModule.function("nope").is_none());
}

#[test]
fn type_mismatch_on_non_array() {
    let func = ArrayModule.function("length").unwrap();
    let mut ctx = Context::new();
    let result = func.call(&[s("not an array")], &mut ctx);
    assert_eq!(
        result,
        Err(Error::TypeMismatch {
            expected: "array",
            got: "non-array",
        })
    );
}

#[test]
fn argument_count_on_missing_args() {
    let func = ArrayModule.function("length").unwrap();
    let mut ctx = Context::new();
    let result = func.call(&[], &mut ctx);
    assert_eq!(
        result,
        Err(Error::ArgumentCount {
            expected: 1,
            got: 0,
        })
    );
}

// ── callback test infrastructure ───────────────────────────────────

use ac_ast_canonical::Expr;
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

fn call_with_eval(module: &dyn Module, name: &str, args: &[NonFunctionValue]) -> NonFunctionValue {
    let func = module.function(name).expect("function exists");
    let mut ctx = Context::new();
    ctx.set_eval(Box::new(TestEval));
    func.call(args, &mut ctx).expect("call succeeds")
}

fn call_err_with_eval(
    module: &dyn Module,
    name: &str,
    args: &[NonFunctionValue],
) -> Result<NonFunctionValue, Error> {
    let func = module.function(name).expect("function exists");
    let mut ctx = Context::new();
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
        "body": body,
    }))
}

// ── take ───────────────────────────────────────────────────────────

#[test]
fn take_basic() {
    let result = call(
        &ArrayModule,
        "take",
        &[
            arr(&[json!(1), json!(2), json!(3), json!(4), json!(5)]),
            n(3.0),
        ],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array[0], json!(1));
    assert_eq!(array[2], json!(3));
}

#[test]
fn take_more_than_length() {
    let result = call(&ArrayModule, "take", &[arr(&[json!(1), json!(2)]), n(10.0)]);
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 2);
}

#[test]
fn take_zero() {
    let result = call(&ArrayModule, "take", &[arr(&[json!(1), json!(2)]), n(0.0)]);
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
fn take_empty_array() {
    let result = call(&ArrayModule, "take", &[arr(&[]), n(3.0)]);
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
fn take_negative_errors() {
    let result = call_err(&ArrayModule, "take", &[arr(&[json!(1)]), n(-1.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn take_non_number_errors() {
    let result = call_err(&ArrayModule, "take", &[arr(&[json!(1)]), s("x")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn take_too_few_args_errors() {
    let result = call_err(&ArrayModule, "take", &[arr(&[json!(1)])]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn take_non_array_errors() {
    let result = call_err(&ArrayModule, "take", &[n(1.0), n(2.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── drop ───────────────────────────────────────────────────────────

#[test]
fn drop_basic() {
    let result = call(
        &ArrayModule,
        "drop",
        &[
            arr(&[json!(1), json!(2), json!(3), json!(4), json!(5)]),
            n(2.0),
        ],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array[0], json!(3));
    assert_eq!(array[2], json!(5));
}

#[test]
fn drop_more_than_length() {
    let result = call(&ArrayModule, "drop", &[arr(&[json!(1), json!(2)]), n(10.0)]);
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
fn drop_zero() {
    let result = call(&ArrayModule, "drop", &[arr(&[json!(1), json!(2)]), n(0.0)]);
    assert_eq!(result.as_array().unwrap().len(), 2);
}

#[test]
fn drop_empty_array() {
    let result = call(&ArrayModule, "drop", &[arr(&[]), n(3.0)]);
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
fn drop_negative_errors() {
    let result = call_err(&ArrayModule, "drop", &[arr(&[json!(1)]), n(-1.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn drop_non_array_errors() {
    let result = call_err(&ArrayModule, "drop", &[n(1.0), n(2.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── enumerate ──────────────────────────────────────────────────────

#[test]
fn enumerate_basic() {
    let result = call(
        &ArrayModule,
        "enumerate",
        &[arr(&[json!("a"), json!("b"), json!("c")])],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array[0], json!([0, "a"]));
    assert_eq!(array[1], json!([1, "b"]));
    assert_eq!(array[2], json!([2, "c"]));
}

#[test]
fn enumerate_empty() {
    let result = call(&ArrayModule, "enumerate", &[arr(&[])]);
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
fn enumerate_single() {
    let result = call(&ArrayModule, "enumerate", &[arr(&[json!(42)])]);
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 1);
    assert_eq!(array[0], json!([0, 42]));
}

#[test]
fn enumerate_non_array_errors() {
    let result = call_err(&ArrayModule, "enumerate", &[n(1.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── unzip ──────────────────────────────────────────────────────────

#[test]
fn unzip_basic() {
    let result = call(
        &ArrayModule,
        "unzip",
        &[arr(&[json!([1, "a"]), json!([2, "b"]), json!([3, "c"])])],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 2);
    assert_eq!(array[0], json!([1, 2, 3]));
    assert_eq!(array[1], json!(["a", "b", "c"]));
}

#[test]
fn unzip_empty() {
    let result = call(&ArrayModule, "unzip", &[arr(&[])]);
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 2);
    assert!(array[0].as_array().unwrap().is_empty());
    assert!(array[1].as_array().unwrap().is_empty());
}

#[test]
fn unzip_skips_non_pairs() {
    let result = call(
        &ArrayModule,
        "unzip",
        &[arr(&[json!([1, 2]), json!(42), json!([3, 4])])],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array[0], json!([1, 3]));
    assert_eq!(array[1], json!([2, 4]));
}

#[test]
fn unzip_non_array_errors() {
    let result = call_err(&ArrayModule, "unzip", &[n(1.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── intersperse ────────────────────────────────────────────────────

#[test]
fn intersperse_basic() {
    let result = call(
        &ArrayModule,
        "intersperse",
        &[arr(&[json!(1), json!(2), json!(3)]), s(",")],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 5);
    assert_eq!(array[0], json!(1));
    assert_eq!(array[1], json!(","));
    assert_eq!(array[2], json!(2));
    assert_eq!(array[3], json!(","));
    assert_eq!(array[4], json!(3));
}

#[test]
fn intersperse_single_element() {
    let result = call(&ArrayModule, "intersperse", &[arr(&[json!(1)]), s(",")]);
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 1);
    assert_eq!(array[0], json!(1));
}

#[test]
fn intersperse_empty() {
    let result = call(&ArrayModule, "intersperse", &[arr(&[]), s(",")]);
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
fn intersperse_with_number_separator() {
    let result = call(
        &ArrayModule,
        "intersperse",
        &[arr(&[json!(1), json!(2)]), n(0.0)],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array[1].as_f64(), Some(0.0));
}

#[test]
fn intersperse_too_few_args_errors() {
    let result = call_err(&ArrayModule, "intersperse", &[arr(&[json!(1)])]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn intersperse_non_array_errors() {
    let result = call_err(&ArrayModule, "intersperse", &[n(1.0), s(",")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── intercalate ────────────────────────────────────────────────────

#[test]
fn intercalate_basic() {
    let result = call(
        &ArrayModule,
        "intercalate",
        &[
            arr(&[json!([1, 2]), json!([3, 4]), json!([5])]),
            arr(&[json!(0), json!(0)]),
        ],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 9);
    assert_eq!(array[0], json!(1));
    assert_eq!(array[1], json!(2));
    assert_eq!(array[2], json!(0));
    assert_eq!(array[3], json!(0));
    assert_eq!(array[4], json!(3));
    assert_eq!(array[5], json!(4));
    assert_eq!(array[6], json!(0));
    assert_eq!(array[7], json!(0));
    assert_eq!(array[8], json!(5));
}

#[test]
fn intercalate_single_group() {
    let result = call(
        &ArrayModule,
        "intercalate",
        &[arr(&[json!([1, 2])]), arr(&[json!(0)])],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 2);
    assert_eq!(array[0], json!(1));
    assert_eq!(array[1], json!(2));
}

#[test]
fn intercalate_empty_groups() {
    let result = call(&ArrayModule, "intercalate", &[arr(&[]), arr(&[json!(0)])]);
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
fn intercalate_non_array_second_errors() {
    let result = call_err(&ArrayModule, "intercalate", &[arr(&[json!([1])]), n(0.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── dedupe ─────────────────────────────────────────────────────────

#[test]
fn dedupe_consecutive() {
    let result = call(
        &ArrayModule,
        "dedupe",
        &[arr(&[
            json!(1),
            json!(1),
            json!(2),
            json!(2),
            json!(3),
            json!(1),
        ])],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 4);
    assert_eq!(array[0], json!(1));
    assert_eq!(array[1], json!(2));
    assert_eq!(array[2], json!(3));
    assert_eq!(array[3], json!(1));
}

#[test]
fn dedupe_all_same() {
    let result = call(
        &ArrayModule,
        "dedupe",
        &[arr(&[json!(1), json!(1), json!(1)])],
    );
    assert_eq!(result.as_array().unwrap().len(), 1);
}

#[test]
fn dedupe_no_duplicates() {
    let result = call(
        &ArrayModule,
        "dedupe",
        &[arr(&[json!(1), json!(2), json!(3)])],
    );
    assert_eq!(result.as_array().unwrap().len(), 3);
}

#[test]
fn dedupe_empty() {
    let result = call(&ArrayModule, "dedupe", &[arr(&[])]);
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
fn dedupe_strings() {
    let result = call(
        &ArrayModule,
        "dedupe",
        &[arr(&[
            json!("a"),
            json!("a"),
            json!("b"),
            json!("b"),
            json!("a"),
        ])],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array[0], json!("a"));
    assert_eq!(array[1], json!("b"));
    assert_eq!(array[2], json!("a"));
}

#[test]
fn dedupe_non_array_errors() {
    let result = call_err(&ArrayModule, "dedupe", &[n(1.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── prepend ────────────────────────────────────────────────────────

#[test]
fn prepend_basic() {
    let result = call(
        &ArrayModule,
        "prepend",
        &[arr(&[json!(2), json!(3)]), n(1.0)],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array[0].as_f64(), Some(1.0));
    assert_eq!(array[1], json!(2));
    assert_eq!(array[2], json!(3));
}

#[test]
fn prepend_to_empty() {
    let result = call(&ArrayModule, "prepend", &[arr(&[]), n(1.0)]);
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 1);
    assert_eq!(array[0].as_f64(), Some(1.0));
}

#[test]
fn prepend_string() {
    let result = call(&ArrayModule, "prepend", &[arr(&[json!("b")]), s("a")]);
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 2);
    assert_eq!(array[0], json!("a"));
    assert_eq!(array[1], json!("b"));
}

#[test]
fn prepend_too_few_args_errors() {
    let result = call_err(&ArrayModule, "prepend", &[arr(&[json!(1)])]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn prepend_non_array_errors() {
    let result = call_err(&ArrayModule, "prepend", &[n(1.0), n(2.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── take_while ─────────────────────────────────────────────────────

#[test]
fn take_while_basic() {
    let result = call_with_eval(
        &ArrayModule,
        "take_while",
        &[
            arr(&[json!(1), json!(2), json!(3), json!(1), json!(0)]),
            fn_value("add(x, -3)", &["x"]),
        ],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 2);
    assert_eq!(array[0], json!(1));
    assert_eq!(array[1], json!(2));
}

#[test]
fn take_while_all_match() {
    let result = call_with_eval(
        &ArrayModule,
        "take_while",
        &[arr(&[json!(1), json!(2)]), fn_value("add(x, -10)", &["x"])],
    );
    assert_eq!(result.as_array().unwrap().len(), 2);
}

#[test]
fn take_while_none_match() {
    let result = call_with_eval(
        &ArrayModule,
        "take_while",
        &[arr(&[json!(3), json!(1)]), fn_value("add(x, -3)", &["x"])],
    );
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
fn take_while_empty() {
    let result = call_with_eval(
        &ArrayModule,
        "take_while",
        &[arr(&[]), fn_value("add(x, -3)", &["x"])],
    );
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
fn take_while_non_function_errors() {
    let result = call_err_with_eval(&ArrayModule, "take_while", &[arr(&[json!(1)]), n(1.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn take_while_non_array_errors() {
    let result = call_err_with_eval(&ArrayModule, "take_while", &[n(1.0), fn_value("x", &["x"])]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── drop_while ─────────────────────────────────────────────────────

#[test]
fn drop_while_basic() {
    let result = call_with_eval(
        &ArrayModule,
        "drop_while",
        &[
            arr(&[json!(1), json!(2), json!(3), json!(1), json!(0)]),
            fn_value("add(x, -3)", &["x"]),
        ],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array[0], json!(3));
    assert_eq!(array[1], json!(1));
    assert_eq!(array[2], json!(0));
}

#[test]
fn drop_while_all_match() {
    let result = call_with_eval(
        &ArrayModule,
        "drop_while",
        &[arr(&[json!(1), json!(2)]), fn_value("add(x, -10)", &["x"])],
    );
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
fn drop_while_none_match() {
    let result = call_with_eval(
        &ArrayModule,
        "drop_while",
        &[arr(&[json!(3), json!(1)]), fn_value("add(x, -3)", &["x"])],
    );
    assert_eq!(result.as_array().unwrap().len(), 2);
}

#[test]
fn drop_while_empty() {
    let result = call_with_eval(
        &ArrayModule,
        "drop_while",
        &[arr(&[]), fn_value("add(x, -3)", &["x"])],
    );
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
fn drop_while_non_function_errors() {
    let result = call_err_with_eval(&ArrayModule, "drop_while", &[arr(&[json!(1)]), n(1.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── scan ───────────────────────────────────────────────────────────

#[test]
fn scan_basic() {
    let result = call_with_eval(
        &ArrayModule,
        "scan",
        &[
            arr(&[json!(1), json!(2), json!(3)]),
            fn_value("add(acc, x)", &["acc", "x"]),
            n(0.0),
        ],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array[0].as_f64(), Some(1.0));
    assert_eq!(array[1].as_f64(), Some(3.0));
    assert_eq!(array[2].as_f64(), Some(6.0));
}

#[test]
fn scan_empty() {
    let result = call_with_eval(
        &ArrayModule,
        "scan",
        &[arr(&[]), fn_value("add(acc, x)", &["acc", "x"]), n(0.0)],
    );
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
fn scan_single_element() {
    let result = call_with_eval(
        &ArrayModule,
        "scan",
        &[
            arr(&[json!(5)]),
            fn_value("add(acc, x)", &["acc", "x"]),
            n(0.0),
        ],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 1);
    assert_eq!(array[0].as_f64(), Some(5.0));
}

#[test]
fn scan_too_few_args_errors() {
    let result = call_err_with_eval(
        &ArrayModule,
        "scan",
        &[arr(&[json!(1)]), fn_value("add(acc, x)", &["acc", "x"])],
    );
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn scan_non_array_errors() {
    let result = call_err_with_eval(
        &ArrayModule,
        "scan",
        &[n(1.0), fn_value("add(acc, x)", &["acc", "x"]), n(0.0)],
    );
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── foldr ──────────────────────────────────────────────────────────

#[test]
fn foldr_basic() {
    let result = call_with_eval(
        &ArrayModule,
        "foldr",
        &[
            arr(&[json!(1), json!(2), json!(3)]),
            fn_value("add(acc, x)", &["acc", "x"]),
            n(0.0),
        ],
    );
    // Right fold: processes right-to-left with fn(acc, item):
    //   acc=0, item=3 -> add(0, 3) = 3
    //   acc=3, item=2 -> add(3, 2) = 5
    //   acc=5, item=1 -> add(5, 1) = 6
    assert_eq!(result.as_number(), Some(6.0));
}

#[test]
fn foldr_empty() {
    let result = call_with_eval(
        &ArrayModule,
        "foldr",
        &[arr(&[]), fn_value("add(acc, x)", &["acc", "x"]), n(42.0)],
    );
    assert_eq!(result.as_number(), Some(42.0));
}

#[test]
fn foldr_single_element() {
    let result = call_with_eval(
        &ArrayModule,
        "foldr",
        &[
            arr(&[json!(5)]),
            fn_value("add(acc, x)", &["acc", "x"]),
            n(0.0),
        ],
    );
    assert_eq!(result.as_number(), Some(5.0));
}

#[test]
fn foldr_too_few_args_errors() {
    let result = call_err_with_eval(
        &ArrayModule,
        "foldr",
        &[arr(&[json!(1)]), fn_value("add(acc, x)", &["acc", "x"])],
    );
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn foldr_non_array_errors() {
    let result = call_err_with_eval(
        &ArrayModule,
        "foldr",
        &[n(1.0), fn_value("add(acc, x)", &["acc", "x"]), n(0.0)],
    );
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── find_index ─────────────────────────────────────────────────────

#[test]
fn find_index_basic() {
    let result = call_with_eval(
        &ArrayModule,
        "find_index",
        &[
            arr(&[json!(0), json!(0), json!(3), json!(0)]),
            fn_value("add(x, 0)", &["x"]),
        ],
    );
    assert_eq!(result.as_number(), Some(2.0));
}

#[test]
fn find_index_not_found() {
    let result = call_with_eval(
        &ArrayModule,
        "find_index",
        &[arr(&[json!(0), json!(0)]), fn_value("add(x, 0)", &["x"])],
    );
    assert_eq!(result.as_number(), Some(-1.0));
}

#[test]
fn find_index_empty() {
    let result = call_with_eval(
        &ArrayModule,
        "find_index",
        &[arr(&[]), fn_value("add(x, 0)", &["x"])],
    );
    assert_eq!(result.as_number(), Some(-1.0));
}

#[test]
fn find_index_non_function_errors() {
    let result = call_err_with_eval(&ArrayModule, "find_index", &[arr(&[json!(1)]), n(1.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── find_last ──────────────────────────────────────────────────────

#[test]
fn find_last_basic() {
    let result = call_with_eval(
        &ArrayModule,
        "find_last",
        &[
            arr(&[json!(0), json!(0), json!(3), json!(5)]),
            fn_value("add(x, 0)", &["x"]),
        ],
    );
    assert_eq!(result.as_number(), Some(5.0));
}

#[test]
fn find_last_not_found() {
    let result = call_with_eval(
        &ArrayModule,
        "find_last",
        &[arr(&[json!(0), json!(0)]), fn_value("add(x, 0)", &["x"])],
    );
    assert!(result.is_null());
}

#[test]
fn find_last_empty() {
    let result = call_with_eval(
        &ArrayModule,
        "find_last",
        &[arr(&[]), fn_value("add(x, 0)", &["x"])],
    );
    assert!(result.is_null());
}

#[test]
fn find_last_first_element() {
    let result = call_with_eval(
        &ArrayModule,
        "find_last",
        &[
            arr(&[json!(3), json!(0), json!(0)]),
            fn_value("add(x, 0)", &["x"]),
        ],
    );
    assert_eq!(result.as_number(), Some(3.0));
}

#[test]
fn find_last_non_function_errors() {
    let result = call_err_with_eval(&ArrayModule, "find_last", &[arr(&[json!(1)]), n(1.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── find_last_index ────────────────────────────────────────────────

#[test]
fn find_last_index_basic() {
    let result = call_with_eval(
        &ArrayModule,
        "find_last_index",
        &[
            arr(&[json!(0), json!(0), json!(3), json!(5)]),
            fn_value("add(x, 0)", &["x"]),
        ],
    );
    assert_eq!(result.as_number(), Some(3.0));
}

#[test]
fn find_last_index_not_found() {
    let result = call_with_eval(
        &ArrayModule,
        "find_last_index",
        &[arr(&[json!(0), json!(0)]), fn_value("add(x, 0)", &["x"])],
    );
    assert_eq!(result.as_number(), Some(-1.0));
}

#[test]
fn find_last_index_empty() {
    let result = call_with_eval(
        &ArrayModule,
        "find_last_index",
        &[arr(&[]), fn_value("add(x, 0)", &["x"])],
    );
    assert_eq!(result.as_number(), Some(-1.0));
}

#[test]
fn find_last_index_single_match() {
    let result = call_with_eval(
        &ArrayModule,
        "find_last_index",
        &[
            arr(&[json!(3), json!(0), json!(0)]),
            fn_value("add(x, 0)", &["x"]),
        ],
    );
    assert_eq!(result.as_number(), Some(0.0));
}

#[test]
fn find_last_index_non_function_errors() {
    let result = call_err_with_eval(&ArrayModule, "find_last_index", &[arr(&[json!(1)]), n(1.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── product ────────────────────────────────────────────────────────

#[test]
fn product_basic() {
    assert_eq!(
        call(
            &ArrayModule,
            "product",
            &[arr(&[json!(2), json!(3), json!(4)])]
        )
        .as_number(),
        Some(24.0)
    );
}

#[test]
fn product_single() {
    assert_eq!(
        call(&ArrayModule, "product", &[arr(&[json!(7)])]).as_number(),
        Some(7.0)
    );
}

#[test]
fn product_empty() {
    assert_eq!(
        call(&ArrayModule, "product", &[arr(&[])]).as_number(),
        Some(1.0)
    );
}

#[test]
fn product_with_zero() {
    assert_eq!(
        call(
            &ArrayModule,
            "product",
            &[arr(&[json!(5), json!(0), json!(3)])]
        )
        .as_number(),
        Some(0.0)
    );
}

#[test]
fn product_negative() {
    assert_eq!(
        call(&ArrayModule, "product", &[arr(&[json!(-2), json!(3)])]).as_number(),
        Some(-6.0)
    );
}

#[test]
fn product_no_args_errors() {
    let result = call_err(&ArrayModule, "product", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn product_non_array_errors() {
    let result = call_err(&ArrayModule, "product", &[n(1.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── mean ───────────────────────────────────────────────────────────

#[test]
fn mean_basic() {
    assert_eq!(
        call(
            &ArrayModule,
            "mean",
            &[arr(&[json!(1), json!(2), json!(3)])]
        )
        .as_number(),
        Some(2.0)
    );
}

#[test]
fn mean_single() {
    assert_eq!(
        call(&ArrayModule, "mean", &[arr(&[json!(42)])]).as_number(),
        Some(42.0)
    );
}

#[test]
fn mean_empty() {
    assert_eq!(
        call(&ArrayModule, "mean", &[arr(&[])]).as_number(),
        Some(0.0)
    );
}

#[test]
fn mean_negative() {
    assert_eq!(
        call(
            &ArrayModule,
            "mean",
            &[arr(&[json!(-4), json!(0), json!(4)])]
        )
        .as_number(),
        Some(0.0)
    );
}

#[test]
fn mean_no_args_errors() {
    let result = call_err(&ArrayModule, "mean", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn mean_non_array_errors() {
    let result = call_err(&ArrayModule, "mean", &[s("x")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── median ─────────────────────────────────────────────────────────

#[test]
fn median_odd() {
    assert_eq!(
        call(
            &ArrayModule,
            "median",
            &[arr(&[json!(3), json!(1), json!(2)])]
        )
        .as_number(),
        Some(2.0)
    );
}

#[test]
fn median_even() {
    assert_eq!(
        call(
            &ArrayModule,
            "median",
            &[arr(&[json!(1), json!(2), json!(3), json!(4)])]
        )
        .as_number(),
        Some(2.5)
    );
}

#[test]
fn median_single() {
    assert_eq!(
        call(&ArrayModule, "median", &[arr(&[json!(5)])]).as_number(),
        Some(5.0)
    );
}

#[test]
fn median_empty() {
    assert_eq!(
        call(&ArrayModule, "median", &[arr(&[])]).as_number(),
        Some(0.0)
    );
}

#[test]
fn median_unsorted() {
    assert_eq!(
        call(
            &ArrayModule,
            "median",
            &[arr(&[json!(5), json!(1), json!(3), json!(2), json!(4)])]
        )
        .as_number(),
        Some(3.0)
    );
}

#[test]
fn median_no_args_errors() {
    let result = call_err(&ArrayModule, "median", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn median_non_array_errors() {
    let result = call_err(&ArrayModule, "median", &[n(1.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── stddev ─────────────────────────────────────────────────────────

#[test]
fn stddev_basic() {
    let result = call(
        &ArrayModule,
        "stddev",
        &[arr(&[
            json!(2),
            json!(4),
            json!(4),
            json!(4),
            json!(5),
            json!(5),
            json!(7),
            json!(9),
        ])],
    )
    .as_number()
    .unwrap();
    assert!((result - 2.0).abs() < 1e-10);
}

#[test]
fn stddev_single() {
    assert_eq!(
        call(&ArrayModule, "stddev", &[arr(&[json!(5)])]).as_number(),
        Some(0.0)
    );
}

#[test]
fn stddev_empty() {
    assert_eq!(
        call(&ArrayModule, "stddev", &[arr(&[])]).as_number(),
        Some(0.0)
    );
}

#[test]
fn stddev_uniform() {
    assert_eq!(
        call(
            &ArrayModule,
            "stddev",
            &[arr(&[json!(3), json!(3), json!(3)])]
        )
        .as_number(),
        Some(0.0)
    );
}

#[test]
fn stddev_no_args_errors() {
    let result = call_err(&ArrayModule, "stddev", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn stddev_non_array_errors() {
    let result = call_err(&ArrayModule, "stddev", &[s("x")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}
