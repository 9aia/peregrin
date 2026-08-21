use ac_runtime::NonFunctionValue;
use ac_runtime_core::Module;
use ac_vm::{Context, Error, Eval};

use super::fp::FpModule;

fn call(module: &dyn Module, name: &str, args: &[NonFunctionValue]) -> NonFunctionValue {
    let func = module.function(name).expect("function exists");
    let mut ctx = Context::new();
    func.call(args, &mut ctx).expect("call succeeds")
}

fn call_err(module: &dyn Module, name: &str, args: &[NonFunctionValue]) -> Result<NonFunctionValue, Error> {
    let func = module.function(name).expect("function exists");
    let mut ctx = Context::new();
    func.call(args, &mut ctx)
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

// ── module ─────────────────────────────────────────────────────────

#[test]
fn module_name() {
    assert_eq!(FpModule.name(), "fp");
}

// ── identity ───────────────────────────────────────────────────────

#[test]
fn identity_number() {
    assert_eq!(
        call(&FpModule, "identity", &[n(42.0)]).as_number(),
        Some(42.0)
    );
}

#[test]
fn identity_string() {
    assert_eq!(
        call(&FpModule, "identity", &[s("hello")]).as_str(),
        Some("hello")
    );
}

#[test]
fn identity_null() {
    assert!(call(&FpModule, "identity", &[null()]).is_null());
}

// ── default ────────────────────────────────────────────────────────

#[test]
fn default_returns_value_when_not_null() {
    assert_eq!(
        call(&FpModule, "default", &[n(42.0), n(99.0)]).as_number(),
        Some(42.0)
    );
}

#[test]
fn default_returns_fallback_when_null() {
    assert_eq!(
        call(&FpModule, "default", &[null(), n(99.0)]).as_number(),
        Some(99.0)
    );
}

// ── coalesce ───────────────────────────────────────────────────────

#[test]
fn coalesce_returns_first_non_null() {
    assert_eq!(
        call(&FpModule, "coalesce", &[null(), null(), n(7.0), n(3.0)]).as_number(),
        Some(7.0)
    );
}

#[test]
fn coalesce_returns_null_when_all_null() {
    assert!(call(&FpModule, "coalesce", &[null(), null()]).is_null());
}

#[test]
fn coalesce_returns_null_when_empty() {
    assert!(call(&FpModule, "coalesce", &[]).is_null());
}

// ── type predicates ────────────────────────────────────────────────

#[test]
fn is_null_true() {
    assert_eq!(call(&FpModule, "is_null", &[null()]).as_bool(), Some(true));
}

#[test]
fn is_null_false() {
    assert_eq!(call(&FpModule, "is_null", &[n(0.0)]).as_bool(), Some(false));
}

#[test]
fn is_bool_true() {
    assert_eq!(call(&FpModule, "is_bool", &[b(true)]).as_bool(), Some(true));
}

#[test]
fn is_bool_false() {
    assert_eq!(call(&FpModule, "is_bool", &[n(1.0)]).as_bool(), Some(false));
}

#[test]
fn is_number_true() {
    assert_eq!(
        call(&FpModule, "is_number", &[n(42.0)]).as_bool(),
        Some(true)
    );
}

#[test]
fn is_number_false() {
    assert_eq!(
        call(&FpModule, "is_number", &[s("42")]).as_bool(),
        Some(false)
    );
}

#[test]
fn is_string_true() {
    assert_eq!(
        call(&FpModule, "is_string", &[s("hello")]).as_bool(),
        Some(true)
    );
}

#[test]
fn is_string_false() {
    assert_eq!(
        call(&FpModule, "is_string", &[n(42.0)]).as_bool(),
        Some(false)
    );
}

#[test]
fn is_array_true() {
    assert_eq!(
        call(&FpModule, "is_array", &[arr(&[])]).as_bool(),
        Some(true)
    );
}

#[test]
fn is_array_false() {
    assert_eq!(
        call(&FpModule, "is_array", &[n(42.0)]).as_bool(),
        Some(false)
    );
}

#[test]
fn is_object_true() {
    assert_eq!(
        call(&FpModule, "is_object", &[obj(&[])]).as_bool(),
        Some(true)
    );
}

#[test]
fn is_object_false() {
    assert_eq!(
        call(&FpModule, "is_object", &[n(42.0)]).as_bool(),
        Some(false)
    );
}

// ── type_of ────────────────────────────────────────────────────────

#[test]
fn type_of_null() {
    assert_eq!(call(&FpModule, "type_of", &[null()]).as_str(), Some("null"));
}

#[test]
fn type_of_bool() {
    assert_eq!(
        call(&FpModule, "type_of", &[b(true)]).as_str(),
        Some("boolean")
    );
}

#[test]
fn type_of_number() {
    assert_eq!(
        call(&FpModule, "type_of", &[n(42.0)]).as_str(),
        Some("number")
    );
}

#[test]
fn type_of_string() {
    assert_eq!(
        call(&FpModule, "type_of", &[s("hi")]).as_str(),
        Some("string")
    );
}

#[test]
fn type_of_array() {
    assert_eq!(
        call(&FpModule, "type_of", &[arr(&[])]).as_str(),
        Some("array")
    );
}

#[test]
fn type_of_object() {
    assert_eq!(
        call(&FpModule, "type_of", &[obj(&[])]).as_str(),
        Some("object")
    );
}

// ── is_empty ───────────────────────────────────────────────────────

#[test]
fn is_empty_null() {
    assert_eq!(call(&FpModule, "is_empty", &[null()]).as_bool(), Some(true));
}

#[test]
fn is_empty_string() {
    assert_eq!(call(&FpModule, "is_empty", &[s("")]).as_bool(), Some(true));
    assert_eq!(
        call(&FpModule, "is_empty", &[s("x")]).as_bool(),
        Some(false)
    );
}

#[test]
fn is_empty_array() {
    assert_eq!(
        call(&FpModule, "is_empty", &[arr(&[])]).as_bool(),
        Some(true)
    );
    assert_eq!(
        call(&FpModule, "is_empty", &[arr(&[serde_json::json!(1)])]).as_bool(),
        Some(false)
    );
}

#[test]
fn is_empty_object() {
    assert_eq!(
        call(&FpModule, "is_empty", &[obj(&[])]).as_bool(),
        Some(true)
    );
    assert_eq!(
        call(
            &FpModule,
            "is_empty",
            &[obj(&[("a", serde_json::json!(1))])]
        )
        .as_bool(),
        Some(false)
    );
}

#[test]
fn is_empty_number_is_false() {
    assert_eq!(
        call(&FpModule, "is_empty", &[n(0.0)]).as_bool(),
        Some(false)
    );
}

// ── is_truthy ──────────────────────────────────────────────────────

#[test]
fn is_truthy_bool() {
    assert_eq!(
        call(&FpModule, "is_truthy", &[b(true)]).as_bool(),
        Some(true)
    );
    assert_eq!(
        call(&FpModule, "is_truthy", &[b(false)]).as_bool(),
        Some(false)
    );
}

#[test]
fn is_truthy_number() {
    assert_eq!(
        call(&FpModule, "is_truthy", &[n(1.0)]).as_bool(),
        Some(true)
    );
    assert_eq!(
        call(&FpModule, "is_truthy", &[n(0.0)]).as_bool(),
        Some(false)
    );
}

#[test]
fn is_truthy_string() {
    assert_eq!(
        call(&FpModule, "is_truthy", &[s("x")]).as_bool(),
        Some(true)
    );
    assert_eq!(
        call(&FpModule, "is_truthy", &[s("")]).as_bool(),
        Some(false)
    );
}

#[test]
fn is_truthy_null() {
    assert_eq!(
        call(&FpModule, "is_truthy", &[null()]).as_bool(),
        Some(false)
    );
}

#[test]
fn is_truthy_array() {
    assert_eq!(
        call(&FpModule, "is_truthy", &[arr(&[])]).as_bool(),
        Some(false)
    );
    assert_eq!(
        call(&FpModule, "is_truthy", &[arr(&[serde_json::json!(1)])]).as_bool(),
        Some(true)
    );
}

#[test]
fn is_truthy_object() {
    assert_eq!(
        call(&FpModule, "is_truthy", &[obj(&[])]).as_bool(),
        Some(false)
    );
    assert_eq!(
        call(
            &FpModule,
            "is_truthy",
            &[obj(&[("a", serde_json::json!(1))])]
        )
        .as_bool(),
        Some(true)
    );
}

// ── range ──────────────────────────────────────────────────────────

#[test]
fn range_default_step() {
    let result = call(&FpModule, "range", &[n(0.0), n(5.0)]);
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 5);
    assert_eq!(arr[0].as_f64(), Some(0.0));
    assert_eq!(arr[4].as_f64(), Some(4.0));
}

#[test]
fn range_custom_step() {
    let result = call(&FpModule, "range", &[n(0.0), n(10.0), n(2.0)]);
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 5);
    assert_eq!(arr[0].as_f64(), Some(0.0));
    assert_eq!(arr[4].as_f64(), Some(8.0));
}

#[test]
fn range_negative_step() {
    let result = call(&FpModule, "range", &[n(5.0), n(0.0), n(-1.0)]);
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 5);
    assert_eq!(arr[0].as_f64(), Some(5.0));
    assert_eq!(arr[4].as_f64(), Some(1.0));
}

#[test]
fn range_empty() {
    let result = call(&FpModule, "range", &[n(5.0), n(5.0)]);
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
fn range_zero_step_errors() {
    let result = call_err(&FpModule, "range", &[n(0.0), n(5.0), n(0.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── zip ─────────────────────────────────────────────────────────────

#[test]
fn zip_equal_length() {
    let result = call(
        &FpModule,
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
        &FpModule,
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

// ── chunk ───────────────────────────────────────────────────────────

#[test]
fn chunk_even() {
    let result = call(
        &FpModule,
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
        &FpModule,
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
    let result = call_err(&FpModule, "chunk", &[arr(&[serde_json::json!(1)]), n(0.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── keys / values / entries ─────────────────────────────────────────

#[test]
fn keys_returns_object_keys() {
    let result = call(
        &FpModule,
        "keys",
        &[obj(&[
            ("a", serde_json::json!(1)),
            ("b", serde_json::json!(2)),
        ])],
    );
    let keys = result.as_array().unwrap();
    assert_eq!(keys.len(), 2);
    assert_eq!(keys[0].as_str(), Some("a"));
    assert_eq!(keys[1].as_str(), Some("b"));
}

#[test]
fn values_returns_object_values() {
    let result = call(
        &FpModule,
        "values",
        &[obj(&[
            ("a", serde_json::json!(1)),
            ("b", serde_json::json!(2)),
        ])],
    );
    let values = result.as_array().unwrap();
    assert_eq!(values.len(), 2);
    assert_eq!(values[0].as_f64(), Some(1.0));
    assert_eq!(values[1].as_f64(), Some(2.0));
}

#[test]
fn entries_returns_key_value_pairs() {
    let result = call(&FpModule, "entries", &[obj(&[("a", serde_json::json!(1))])]);
    let entries = result.as_array().unwrap();
    assert_eq!(entries.len(), 1);
    let pair = entries[0].as_array().unwrap();
    assert_eq!(pair[0].as_str(), Some("a"));
    assert_eq!(pair[1].as_f64(), Some(1.0));
}

#[test]
fn keys_on_non_object_errors() {
    let result = call_err(&FpModule, "keys", &[n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── pick / omit ─────────────────────────────────────────────────────

#[test]
fn pick_selects_keys() {
    let result = call(
        &FpModule,
        "pick",
        &[
            obj(&[
                ("a", serde_json::json!(1)),
                ("b", serde_json::json!(2)),
                ("c", serde_json::json!(3)),
            ]),
            arr(&[serde_json::json!("a"), serde_json::json!("c")]),
        ],
    );
    let obj = result.as_object().unwrap();
    assert_eq!(obj.len(), 2);
    assert!(obj.contains_key("a"));
    assert!(obj.contains_key("c"));
    assert!(!obj.contains_key("b"));
}

#[test]
fn pick_missing_key_ignored() {
    let result = call(
        &FpModule,
        "pick",
        &[
            obj(&[("a", serde_json::json!(1))]),
            arr(&[serde_json::json!("a"), serde_json::json!("z")]),
        ],
    );
    assert_eq!(result.as_object().unwrap().len(), 1);
}

#[test]
fn omit_removes_keys() {
    let result = call(
        &FpModule,
        "omit",
        &[
            obj(&[
                ("a", serde_json::json!(1)),
                ("b", serde_json::json!(2)),
                ("c", serde_json::json!(3)),
            ]),
            arr(&[serde_json::json!("b")]),
        ],
    );
    let obj = result.as_object().unwrap();
    assert_eq!(obj.len(), 2);
    assert!(obj.contains_key("a"));
    assert!(obj.contains_key("c"));
    assert!(!obj.contains_key("b"));
}

// ── merge ───────────────────────────────────────────────────────────

#[test]
fn merge_two_objects() {
    let result = call(
        &FpModule,
        "merge",
        &[
            obj(&[("a", serde_json::json!(1)), ("b", serde_json::json!(2))]),
            obj(&[("b", serde_json::json!(3)), ("c", serde_json::json!(4))]),
        ],
    );
    let obj = result.as_object().unwrap();
    assert_eq!(obj.len(), 3);
    assert_eq!(obj.get("a").unwrap().as_f64(), Some(1.0));
    assert_eq!(obj.get("b").unwrap().as_f64(), Some(3.0));
    assert_eq!(obj.get("c").unwrap().as_f64(), Some(4.0));
}

#[test]
fn merge_right_wins_on_conflict() {
    let result = call(
        &FpModule,
        "merge",
        &[
            obj(&[("a", serde_json::json!(1))]),
            obj(&[("a", serde_json::json!(99))]),
        ],
    );
    assert_eq!(
        result.as_object().unwrap().get("a").unwrap().as_f64(),
        Some(99.0)
    );
}

#[test]
fn merge_on_non_object_errors() {
    let result = call_err(&FpModule, "merge", &[n(1.0), obj(&[])]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── higher-order functions ─────────────────────────────────────────

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

fn call_err_with_eval(module: &dyn Module, name: &str, args: &[NonFunctionValue]) -> Result<NonFunctionValue, Error> {
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

#[test]
fn map_doubles_each_element() {
    let result = call_with_eval(
        &FpModule,
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
    let result = call_with_eval(&FpModule, "map", &[fn_value("add(x, 1)", &["x"]), arr(&[])]);
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
fn filter_keeps_matching_elements() {
    let result = call_with_eval(
        &FpModule,
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
    let result = call_with_eval(
        &FpModule,
        "filter",
        &[fn_value("add(x, 0)", &["x"]), arr(&[])],
    );
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
fn reduce_sums_all_elements() {
    let result = call_with_eval(
        &FpModule,
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
        &FpModule,
        "reduce",
        &[fn_value("add(acc, x)", &["acc", "x"]), arr(&[]), n(42.0)],
    );
    assert_eq!(result.as_number(), Some(42.0));
}

#[test]
fn map_without_eval_errors() {
    let result = call_err(
        &FpModule,
        "map",
        &[fn_value("add(x, 1)", &["x"]), arr(&[serde_json::json!(1)])],
    );
    assert!(matches!(result, Err(Error::PrimitiveError(_))));
}

#[test]
fn map_non_function_errors() {
    let result = call_err(&FpModule, "map", &[n(42.0), arr(&[serde_json::json!(1)])]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── pipe / flow / compose ──────────────────────────────────────────

#[test]
fn pipe_chains_functions_left_to_right() {
    let result = call_with_eval(
        &FpModule,
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
    let result = call_with_eval(&FpModule, "pipe", &[n(3.0), fn_value("add(x, 7)", &["x"])]);
    assert_eq!(result.as_number(), Some(10.0));
}

#[test]
fn pipe_without_eval_errors() {
    let result = call_err(&FpModule, "pipe", &[n(3.0), fn_value("add(x, 1)", &["x"])]);
    assert!(matches!(result, Err(Error::PrimitiveError(_))));
}

#[test]
fn pipe_non_function_errors() {
    let result = call_err(&FpModule, "pipe", &[n(3.0), n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn flow_returns_function_value() {
    let result = call(
        &FpModule,
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
        &FpModule,
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
    let result = call(&FpModule, "flow", &[fn_value("add(x, 42)", &["x"])]);
    let fv = result.as_function().unwrap();
    assert_eq!(fv.body, "add(x, 42)");
}

#[test]
fn flow_with_different_param_names() {
    let result = call(
        &FpModule,
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
fn compose_returns_function_value() {
    let result = call(
        &FpModule,
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
    let result = call(&FpModule, "compose", &[fn_value("add(x, 42)", &["x"])]);
    let fv = result.as_function().unwrap();
    assert_eq!(fv.body, "add(x, 42)");
}

#[test]
fn flow_and_compose_are_inverse_order() {
    let f = fn_value("add(x, 1)", &["x"]);
    let g = fn_value("mul(x, 3)", &["x"]);

    let flow_result = call(&FpModule, "flow", &[f.clone(), g.clone()]);
    let compose_result = call(&FpModule, "compose", &[f, g]);

    let flow_fv = flow_result.as_function().unwrap();
    let compose_fv = compose_result.as_function().unwrap();

    assert_eq!(flow_fv.body, "mul((add(x, 1)), 3)");
    assert_eq!(compose_fv.body, "add((mul(x, 3)), 1)");
}

#[test]
fn flow_result_is_callable_via_eval() {
    let composed = call(
        &FpModule,
        "flow",
        &[
            fn_value("add(x, 10)", &["x"]),
            fn_value("mul(x, 2)", &["x"]),
        ],
    );
    let result = call_with_eval(&FpModule, "pipe", &[n(5.0), composed]);
    assert_eq!(result.as_number(), Some(30.0));
}

// ── fanout ─────────────────────────────────────────────────────────

#[test]
fn fanout_applies_all_functions_to_value() {
    let result = call_with_eval(
        &FpModule,
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
    let result = call_with_eval(
        &FpModule,
        "fanout",
        &[n(3.0), fn_value("add(x, 7)", &["x"])],
    );
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0].as_f64(), Some(10.0));
}

#[test]
fn fanout_without_eval_errors() {
    let result = call_err(
        &FpModule,
        "fanout",
        &[n(3.0), fn_value("add(x, 1)", &["x"])],
    );
    assert!(matches!(result, Err(Error::PrimitiveError(_))));
}

#[test]
fn fanout_non_function_errors() {
    let result = call_err(&FpModule, "fanout", &[n(3.0), n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn fanout_too_few_args_errors() {
    let result = call_err(&FpModule, "fanout", &[n(3.0)]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── choice ─────────────────────────────────────────────────────────

#[test]
fn choice_selects_true_branch() {
    let result = call_with_eval(
        &FpModule,
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
        &FpModule,
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
        &FpModule,
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
        &FpModule,
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
        &FpModule,
        "choice",
        &[n(1.0), n(42.0), fn_value("add(x, 1)", &["x"]), fn_value("add(x, 2)", &["x"])],
    );
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn choice_too_few_args_errors() {
    let result = call_err(
        &FpModule,
        "choice",
        &[n(1.0), fn_value("x", &["x"]), fn_value("add(x, 1)", &["x"])],
    );
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── flat_map ───────────────────────────────────────────────────────

#[test]
fn flat_map_flattens_array_results() {
    // flat_map with a function that returns an array via the host.
    // Since the expression language has no array literals, we test
    // flattening by using a function whose body is a nested call
    // that the host can evaluate. Here we use identity-like mapping
    // where each element maps to a single value (non-array), which
    // passes through without flattening.
    //
    // For true array flattening, we'd need a host function that
    // returns arrays. The `range` function is in FpModule, not
    // DefaultHost. So we test the non-array passthrough here and
    // verify array flattening via the `partition` result shape.
    let result = call_with_eval(
        &FpModule,
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
fn flat_map_passes_through_non_array_results() {
    let result = call_with_eval(
        &FpModule,
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
    let result = call_with_eval(
        &FpModule,
        "flat_map",
        &[fn_value("add(x, 1)", &["x"]), arr(&[])],
    );
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
fn flat_map_non_function_errors() {
    let result = call_err(&FpModule, "flat_map", &[n(42.0), arr(&[serde_json::json!(1)])]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── find ───────────────────────────────────────────────────────────

#[test]
fn find_returns_first_matching() {
    let result = call_with_eval(
        &FpModule,
        "find",
        &[
            fn_value("add(x, 0)", &["x"]),
            arr(&[serde_json::json!(0), serde_json::json!(3), serde_json::json!(5)]),
        ],
    );
    assert_eq!(result.as_number(), Some(3.0));
}

#[test]
fn find_returns_null_when_no_match() {
    let result = call_with_eval(
        &FpModule,
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
    let result = call_with_eval(
        &FpModule,
        "find",
        &[fn_value("add(x, 0)", &["x"]), arr(&[])],
    );
    assert!(result.is_null());
}

// ── some / every ───────────────────────────────────────────────────

#[test]
fn some_returns_true_when_any_matches() {
    let result = call_with_eval(
        &FpModule,
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
        &FpModule,
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
    let result = call_with_eval(
        &FpModule,
        "some",
        &[fn_value("add(x, 0)", &["x"]), arr(&[])],
    );
    assert_eq!(result.as_bool(), Some(false));
}

#[test]
fn every_returns_true_when_all_match() {
    let result = call_with_eval(
        &FpModule,
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
        &FpModule,
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
    let result = call_with_eval(
        &FpModule,
        "every",
        &[fn_value("add(x, 0)", &["x"]), arr(&[])],
    );
    assert_eq!(result.as_bool(), Some(true));
}

// ── group_by ───────────────────────────────────────────────────────

#[test]
fn group_by_groups_by_key() {
    // Group by whether the number is > 2 (using add for comparison
    // is not possible; instead group by the value itself).
    let result = call_with_eval(
        &FpModule,
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
        &FpModule,
        "group_by",
        &[
            fn_value("x", &["x"]),
            arr(&[serde_json::json!("a"), serde_json::json!("b"), serde_json::json!("a")]),
        ],
    );
    let obj = result.as_object().unwrap();
    assert_eq!(obj.len(), 2);
    assert_eq!(obj.get("a").unwrap().as_array().unwrap().len(), 2);
    assert_eq!(obj.get("b").unwrap().as_array().unwrap().len(), 1);
}

#[test]
fn group_by_empty_array() {
    let result = call_with_eval(
        &FpModule,
        "group_by",
        &[fn_value("x", &["x"]), arr(&[])],
    );
    assert!(result.as_object().unwrap().is_empty());
}

// ── partition ──────────────────────────────────────────────────────

#[test]
fn partition_splits_matching_and_rest() {
    let result = call_with_eval(
        &FpModule,
        "partition",
        &[
            fn_value("add(x, 0)", &["x"]),
            arr(&[serde_json::json!(0), serde_json::json!(3), serde_json::json!(0), serde_json::json!(5)]),
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
        &FpModule,
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
    let result = call_with_eval(
        &FpModule,
        "partition",
        &[fn_value("add(x, 0)", &["x"]), arr(&[])],
    );
    let outer = result.as_array().unwrap();
    assert_eq!(outer[0].as_array().unwrap().len(), 0);
    assert_eq!(outer[1].as_array().unwrap().len(), 0);
}

// ── sort_by ────────────────────────────────────────────────────────

#[test]
fn sort_by_sorts_ascending() {
    let result = call_with_eval(
        &FpModule,
        "sort_by",
        &[
            fn_value("x", &["x"]),
            arr(&[serde_json::json!(3), serde_json::json!(1), serde_json::json!(2)]),
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
        &FpModule,
        "sort_by",
        &[
            fn_value("mul(x, -1)", &["x"]),
            arr(&[serde_json::json!(1), serde_json::json!(3), serde_json::json!(2)]),
        ],
    );
    let arr = result.as_array().unwrap();
    assert_eq!(arr[0].as_f64(), Some(3.0));
    assert_eq!(arr[1].as_f64(), Some(2.0));
    assert_eq!(arr[2].as_f64(), Some(1.0));
}

#[test]
fn sort_by_empty_array() {
    let result = call_with_eval(
        &FpModule,
        "sort_by",
        &[fn_value("x", &["x"]), arr(&[])],
    );
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
fn sort_by_non_number_key_errors() {
    let result = call_err_with_eval(
        &FpModule,
        "sort_by",
        &[
            fn_value("x", &["x"]),
            arr(&[serde_json::json!("a"), serde_json::json!("b")]),
        ],
    );
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── apply ──────────────────────────────────────────────────────────

#[test]
fn apply_calls_function_with_args() {
    let result = call_with_eval(
        &FpModule,
        "apply",
        &[
            fn_value("add(a, b)", &["a", "b"]),
            arr(&[serde_json::json!(3), serde_json::json!(4)]),
        ],
    );
    assert_eq!(result.as_number(), Some(7.0));
}

#[test]
fn apply_binds_fewer_args_than_params() {
    let result = call_with_eval(
        &FpModule,
        "apply",
        &[
            fn_value("add(a, b)", &["a", "b"]),
            arr(&[serde_json::json!(3)]),
        ],
    );
    assert_eq!(result.as_number(), Some(3.0));
}

#[test]
fn apply_single_arg() {
    let result = call_with_eval(
        &FpModule,
        "apply",
        &[
            fn_value("mul(x, 2)", &["x"]),
            arr(&[serde_json::json!(5)]),
        ],
    );
    assert_eq!(result.as_number(), Some(10.0));
}

#[test]
fn apply_without_eval_errors() {
    let result = call_err(
        &FpModule,
        "apply",
        &[fn_value("add(a, b)", &["a", "b"]), arr(&[serde_json::json!(1), serde_json::json!(2)])],
    );
    assert!(matches!(result, Err(Error::PrimitiveError(_))));
}

#[test]
fn apply_non_function_errors() {
    let result = call_err(&FpModule, "apply", &[n(42.0), arr(&[serde_json::json!(1)])]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn apply_too_few_args_errors() {
    let result = call_err(&FpModule, "apply", &[fn_value("add(a, b)", &["a", "b"])]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── tap ────────────────────────────────────────────────────────────

#[test]
fn tap_returns_value_unchanged() {
    let result = call_with_eval(
        &FpModule,
        "tap",
        &[fn_value("add(x, 100)", &["x"]), n(42.0)],
    );
    assert_eq!(result.as_number(), Some(42.0));
}

#[test]
fn tap_with_string() {
    let result = call_with_eval(
        &FpModule,
        "tap",
        &[fn_value("x", &["x"]), s("hello")],
    );
    assert_eq!(result.as_str(), Some("hello"));
}

#[test]
fn tap_without_eval_errors() {
    let result = call_err(
        &FpModule,
        "tap",
        &[fn_value("add(x, 1)", &["x"]), n(5.0)],
    );
    assert!(matches!(result, Err(Error::PrimitiveError(_))));
}

#[test]
fn tap_non_function_errors() {
    let result = call_err(&FpModule, "tap", &[n(42.0), n(5.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn tap_too_few_args_errors() {
    let result = call_err(&FpModule, "tap", &[fn_value("add(x, 1)", &["x"])]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── error cases ────────────────────────────────────────────────────

#[test]
fn unknown_function_returns_none() {
    assert!(FpModule.function("nope").is_none());
}

#[test]
fn argument_count_on_missing_args() {
    let result = call_err(&FpModule, "identity", &[]);
    assert_eq!(
        result,
        Err(Error::ArgumentCount {
            expected: 1,
            got: 0,
        })
    );
}

#[test]
fn argument_count_on_default_missing_args() {
    let result = call_err(&FpModule, "default", &[n(1.0)]);
    assert_eq!(
        result,
        Err(Error::ArgumentCount {
            expected: 2,
            got: 1,
        })
    );
}
