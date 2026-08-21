use ac_runtime::NonFunctionValue;
use ac_runtime_core::Module;
use ac_vm::{Context, Error, Eval};

use super::obj::ObjModule;

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

// ── eval infrastructure for higher-order functions ─────────────────

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

// ── module ─────────────────────────────────────────────────────────

#[test]
fn module_name() {
    assert_eq!(ObjModule.name(), "obj");
}

#[test]
fn unknown_function_returns_none() {
    assert!(ObjModule.function("nope").is_none());
}

// ── keys ───────────────────────────────────────────────────────────

#[test]
fn keys_returns_object_keys() {
    let result = call(
        &ObjModule,
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
fn keys_empty_object() {
    let result = call(&ObjModule, "keys", &[obj(&[])]);
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
fn keys_on_non_object_errors() {
    let result = call_err(&ObjModule, "keys", &[n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn keys_no_args_errors() {
    let result = call_err(&ObjModule, "keys", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── values ─────────────────────────────────────────────────────────

#[test]
fn values_returns_object_values() {
    let result = call(
        &ObjModule,
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
fn values_empty_object() {
    let result = call(&ObjModule, "values", &[obj(&[])]);
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
fn values_on_non_object_errors() {
    let result = call_err(&ObjModule, "values", &[n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn values_no_args_errors() {
    let result = call_err(&ObjModule, "values", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── entries ────────────────────────────────────────────────────────

#[test]
fn entries_returns_key_value_pairs() {
    let result = call(
        &ObjModule,
        "entries",
        &[obj(&[("a", serde_json::json!(1))])],
    );
    let entries = result.as_array().unwrap();
    assert_eq!(entries.len(), 1);
    let pair = entries[0].as_array().unwrap();
    assert_eq!(pair[0].as_str(), Some("a"));
    assert_eq!(pair[1].as_f64(), Some(1.0));
}

#[test]
fn entries_empty_object() {
    let result = call(&ObjModule, "entries", &[obj(&[])]);
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
fn entries_on_non_object_errors() {
    let result = call_err(&ObjModule, "entries", &[n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn entries_no_args_errors() {
    let result = call_err(&ObjModule, "entries", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── pick ───────────────────────────────────────────────────────────

#[test]
fn pick_selects_keys() {
    let result = call(
        &ObjModule,
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
        &ObjModule,
        "pick",
        &[
            obj(&[("a", serde_json::json!(1))]),
            arr(&[serde_json::json!("a"), serde_json::json!("z")]),
        ],
    );
    assert_eq!(result.as_object().unwrap().len(), 1);
}

#[test]
fn pick_empty_keys() {
    let result = call(
        &ObjModule,
        "pick",
        &[obj(&[("a", serde_json::json!(1))]), arr(&[])],
    );
    assert!(result.as_object().unwrap().is_empty());
}

#[test]
fn pick_no_args_errors() {
    let result = call_err(&ObjModule, "pick", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn pick_one_arg_errors() {
    let result = call_err(&ObjModule, "pick", &[obj(&[("a", serde_json::json!(1))])]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn pick_non_object_errors() {
    let result = call_err(
        &ObjModule,
        "pick",
        &[n(42.0), arr(&[serde_json::json!("a")])],
    );
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── omit ───────────────────────────────────────────────────────────

#[test]
fn omit_removes_keys() {
    let result = call(
        &ObjModule,
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

#[test]
fn omit_empty_keys() {
    let result = call(
        &ObjModule,
        "omit",
        &[obj(&[("a", serde_json::json!(1))]), arr(&[])],
    );
    assert_eq!(result.as_object().unwrap().len(), 1);
}

#[test]
fn omit_missing_key_ignored() {
    let result = call(
        &ObjModule,
        "omit",
        &[
            obj(&[("a", serde_json::json!(1))]),
            arr(&[serde_json::json!("z")]),
        ],
    );
    assert_eq!(result.as_object().unwrap().len(), 1);
}

#[test]
fn omit_no_args_errors() {
    let result = call_err(&ObjModule, "omit", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn omit_non_object_errors() {
    let result = call_err(
        &ObjModule,
        "omit",
        &[n(42.0), arr(&[serde_json::json!("a")])],
    );
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── merge ──────────────────────────────────────────────────────────

#[test]
fn merge_two_objects() {
    let result = call(
        &ObjModule,
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
        &ObjModule,
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
    let result = call_err(&ObjModule, "merge", &[n(1.0), obj(&[])]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn merge_no_args_errors() {
    let result = call_err(&ObjModule, "merge", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn merge_one_arg_errors() {
    let result = call_err(&ObjModule, "merge", &[obj(&[("a", serde_json::json!(1))])]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn merge_second_non_object_errors() {
    let result = call_err(&ObjModule, "merge", &[obj(&[]), n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn merge_both_empty() {
    let result = call(&ObjModule, "merge", &[obj(&[]), obj(&[])]);
    assert!(result.as_object().unwrap().is_empty());
}

// ── deep_get ───────────────────────────────────────────────────────

#[test]
fn deep_get_nested_object() {
    let value = obj(&[("a", serde_json::json!({"b": serde_json::json!({"c": 42})}))]);
    let result = call(&ObjModule, "deep_get", &[value, s("a.b.c")]);
    assert_eq!(result.as_number(), Some(42.0));
}

#[test]
fn deep_get_array_index() {
    let value = obj(&[("arr", serde_json::json!([10, 20, 30]))]);
    let result = call(&ObjModule, "deep_get", &[value, s("arr.1")]);
    assert_eq!(result.as_number(), Some(20.0));
}

#[test]
fn deep_get_missing_path_returns_null() {
    let value = obj(&[("a", serde_json::json!(1))]);
    let result = call(&ObjModule, "deep_get", &[value, s("a.b.c")]);
    assert!(result.is_null());
}

#[test]
fn deep_get_single_key() {
    let value = obj(&[("a", serde_json::json!(1))]);
    let result = call(&ObjModule, "deep_get", &[value, s("a")]);
    assert_eq!(result.as_number(), Some(1.0));
}

#[test]
fn deep_get_on_non_object_returns_null() {
    let result = call(&ObjModule, "deep_get", &[n(42.0), s("a")]);
    assert!(result.is_null());
}

#[test]
fn deep_get_no_args_errors() {
    let result = call_err(&ObjModule, "deep_get", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn deep_get_one_arg_errors() {
    let result = call_err(
        &ObjModule,
        "deep_get",
        &[obj(&[("a", serde_json::json!(1))])],
    );
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn deep_get_non_string_path_errors() {
    let result = call_err(
        &ObjModule,
        "deep_get",
        &[obj(&[("a", serde_json::json!(1))]), n(42.0)],
    );
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn deep_get_empty_path() {
    let result = call(
        &ObjModule,
        "deep_get",
        &[obj(&[("a", serde_json::json!(1))]), s("")],
    );
    assert_eq!(
        result.as_object().unwrap().get("a").unwrap().as_f64(),
        Some(1.0)
    );
}

#[test]
fn deep_get_array_non_numeric_index_returns_null() {
    let value = obj(&[("arr", serde_json::json!([10, 20]))]);
    let result = call(&ObjModule, "deep_get", &[value, s("arr.x")]);
    assert!(result.is_null());
}

#[test]
fn deep_get_array_out_of_bounds_returns_null() {
    let value = obj(&[("arr", serde_json::json!([10, 20]))]);
    let result = call(&ObjModule, "deep_get", &[value, s("arr.5")]);
    assert!(result.is_null());
}

#[test]
fn deep_get_deeply_nested() {
    let value = obj(&[(
        "a",
        serde_json::json!({"b": serde_json::json!({"c": serde_json::json!({"d": 99})})}),
    )]);
    let result = call(&ObjModule, "deep_get", &[value, s("a.b.c.d")]);
    assert_eq!(result.as_number(), Some(99.0));
}

#[test]
fn deep_get_trailing_dot() {
    let value = obj(&[("a", serde_json::json!(1))]);
    let result = call(&ObjModule, "deep_get", &[value, s("a.")]);
    assert_eq!(result.as_number(), Some(1.0));
}

#[test]
fn deep_get_on_array_root() {
    let result = call(
        &ObjModule,
        "deep_get",
        &[arr(&[serde_json::json!(10), serde_json::json!(20)]), s("1")],
    );
    assert_eq!(result.as_number(), Some(20.0));
}

// ── deep_merge ─────────────────────────────────────────────────────

#[test]
fn deep_merge_nested_objects() {
    let a = obj(&[("config", serde_json::json!({"a": 1, "b": 2}))]);
    let b = obj(&[("config", serde_json::json!({"b": 3, "c": 4}))]);
    let result = call(&ObjModule, "deep_merge", &[a, b]);
    let merged = result.as_object().unwrap();
    let config = merged.get("config").unwrap().as_object().unwrap();
    assert_eq!(config.len(), 3);
    assert_eq!(config.get("a").unwrap().as_f64(), Some(1.0));
    assert_eq!(config.get("b").unwrap().as_f64(), Some(3.0));
    assert_eq!(config.get("c").unwrap().as_f64(), Some(4.0));
}

#[test]
fn deep_merge_shallow_like_merge() {
    let a = obj(&[("a", serde_json::json!(1)), ("b", serde_json::json!(2))]);
    let b = obj(&[("b", serde_json::json!(3)), ("c", serde_json::json!(4))]);
    let result = call(&ObjModule, "deep_merge", &[a, b]);
    let merged = result.as_object().unwrap();
    assert_eq!(merged.len(), 3);
    assert_eq!(merged.get("a").unwrap().as_f64(), Some(1.0));
    assert_eq!(merged.get("b").unwrap().as_f64(), Some(3.0));
    assert_eq!(merged.get("c").unwrap().as_f64(), Some(4.0));
}

#[test]
fn deep_merge_on_non_object_errors() {
    let result = call_err(&ObjModule, "deep_merge", &[n(1.0), obj(&[])]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn deep_merge_no_args_errors() {
    let result = call_err(&ObjModule, "deep_merge", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn deep_merge_one_arg_errors() {
    let result = call_err(
        &ObjModule,
        "deep_merge",
        &[obj(&[("a", serde_json::json!(1))])],
    );
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn deep_merge_second_non_object_errors() {
    let result = call_err(&ObjModule, "deep_merge", &[obj(&[]), n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn deep_merge_both_empty() {
    let result = call(&ObjModule, "deep_merge", &[obj(&[]), obj(&[])]);
    assert!(result.as_object().unwrap().is_empty());
}

#[test]
fn deep_merge_non_overlapping_keys() {
    let a = obj(&[("a", serde_json::json!(1))]);
    let b = obj(&[("b", serde_json::json!(2))]);
    let result = call(&ObjModule, "deep_merge", &[a, b]);
    let merged = result.as_object().unwrap();
    assert_eq!(merged.len(), 2);
    assert_eq!(merged.get("a").unwrap().as_f64(), Some(1.0));
    assert_eq!(merged.get("b").unwrap().as_f64(), Some(2.0));
}

#[test]
fn deep_merge_right_overrides_non_object() {
    let a = obj(&[("a", serde_json::json!(1))]);
    let b = obj(&[("a", serde_json::json!("hello"))]);
    let result = call(&ObjModule, "deep_merge", &[a, b]);
    assert_eq!(
        result.as_object().unwrap().get("a").unwrap().as_str(),
        Some("hello")
    );
}

#[test]
fn deep_merge_three_levels_deep() {
    let a = obj(&[("x", serde_json::json!({"y": serde_json::json!({"z": 1})}))]);
    let b = obj(&[("x", serde_json::json!({"y": serde_json::json!({"w": 2})}))]);
    let result = call(&ObjModule, "deep_merge", &[a, b]);
    let x = result
        .as_object()
        .unwrap()
        .get("x")
        .unwrap()
        .as_object()
        .unwrap();
    let y = x.get("y").unwrap().as_object().unwrap();
    assert_eq!(y.len(), 2);
    assert_eq!(y.get("z").unwrap().as_f64(), Some(1.0));
    assert_eq!(y.get("w").unwrap().as_f64(), Some(2.0));
}

// ── from_entries ──────────────────────────────────────────────────

#[test]
fn from_entries_builds_object() {
    let result = call(
        &ObjModule,
        "from_entries",
        &[arr(&[
            serde_json::json!(["a", 1]),
            serde_json::json!(["b", 2]),
        ])],
    );
    let obj = result.as_object().unwrap();
    assert_eq!(obj.len(), 2);
    assert_eq!(obj.get("a").unwrap().as_f64(), Some(1.0));
    assert_eq!(obj.get("b").unwrap().as_f64(), Some(2.0));
}

#[test]
fn from_entries_empty() {
    let result = call(&ObjModule, "from_entries", &[arr(&[])]);
    assert!(result.as_object().unwrap().is_empty());
}

#[test]
fn from_entries_single() {
    let result = call(
        &ObjModule,
        "from_entries",
        &[arr(&[serde_json::json!(["a", 42])])],
    );
    assert_eq!(
        result.as_object().unwrap().get("a").unwrap().as_f64(),
        Some(42.0)
    );
}

#[test]
fn from_entries_overwrites_duplicate_keys() {
    let result = call(
        &ObjModule,
        "from_entries",
        &[arr(&[
            serde_json::json!(["a", 1]),
            serde_json::json!(["a", 2]),
        ])],
    );
    let obj = result.as_object().unwrap();
    assert_eq!(obj.len(), 1);
    assert_eq!(obj.get("a").unwrap().as_f64(), Some(2.0));
}

#[test]
fn from_entries_non_array_errors() {
    let result = call_err(&ObjModule, "from_entries", &[n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn from_entries_no_args_errors() {
    let result = call_err(&ObjModule, "from_entries", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn from_entries_non_string_key_errors() {
    let result = call_err(
        &ObjModule,
        "from_entries",
        &[arr(&[serde_json::json!([42, "a"])])],
    );
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── get ───────────────────────────────────────────────────────────

#[test]
fn get_returns_value_by_key() {
    let result = call(
        &ObjModule,
        "get",
        &[obj(&[("a", serde_json::json!(1))]), s("a")],
    );
    assert_eq!(result.as_number(), Some(1.0));
}

#[test]
fn get_missing_key_returns_null() {
    let result = call(
        &ObjModule,
        "get",
        &[obj(&[("a", serde_json::json!(1))]), s("b")],
    );
    assert!(result.is_null());
}

#[test]
fn get_nested_value() {
    let result = call(
        &ObjModule,
        "get",
        &[obj(&[("a", serde_json::json!({"b": 2}))]), s("a")],
    );
    assert_eq!(
        result.as_object().unwrap().get("b").unwrap().as_f64(),
        Some(2.0)
    );
}

#[test]
fn get_non_object_errors() {
    let result = call_err(&ObjModule, "get", &[n(42.0), s("a")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn get_non_string_key_errors() {
    let result = call_err(
        &ObjModule,
        "get",
        &[obj(&[("a", serde_json::json!(1))]), n(42.0)],
    );
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn get_no_args_errors() {
    let result = call_err(&ObjModule, "get", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── get_or ────────────────────────────────────────────────────────

#[test]
fn get_or_returns_value_when_key_exists() {
    let result = call(
        &ObjModule,
        "get_or",
        &[obj(&[("a", serde_json::json!(1))]), s("a"), n(0.0)],
    );
    assert_eq!(result.as_number(), Some(1.0));
}

#[test]
fn get_or_returns_default_when_missing() {
    let result = call(
        &ObjModule,
        "get_or",
        &[obj(&[("a", serde_json::json!(1))]), s("b"), n(99.0)],
    );
    assert_eq!(result.as_number(), Some(99.0));
}

#[test]
fn get_or_returns_default_when_null_value() {
    let result = call(
        &ObjModule,
        "get_or",
        &[obj(&[("a", serde_json::Value::Null)]), s("a"), n(99.0)],
    );
    assert_eq!(result.as_number(), Some(99.0));
}

#[test]
fn get_or_non_object_errors() {
    let result = call_err(&ObjModule, "get_or", &[n(42.0), s("a"), n(0.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn get_or_no_args_errors() {
    let result = call_err(&ObjModule, "get_or", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── set ───────────────────────────────────────────────────────────

#[test]
fn set_adds_new_key() {
    let result = call(
        &ObjModule,
        "set",
        &[obj(&[("a", serde_json::json!(1))]), s("b"), n(2.0)],
    );
    let obj = result.as_object().unwrap();
    assert_eq!(obj.len(), 2);
    assert_eq!(obj.get("a").unwrap().as_f64(), Some(1.0));
    assert_eq!(obj.get("b").unwrap().as_f64(), Some(2.0));
}

#[test]
fn set_overwrites_existing_key() {
    let result = call(
        &ObjModule,
        "set",
        &[obj(&[("a", serde_json::json!(1))]), s("a"), n(99.0)],
    );
    assert_eq!(
        result.as_object().unwrap().get("a").unwrap().as_f64(),
        Some(99.0)
    );
}

#[test]
fn set_preserves_original() {
    let original = obj(&[("a", serde_json::json!(1))]);
    let _ = call(&ObjModule, "set", &[original.clone(), s("b"), n(2.0)]);
    assert_eq!(original.as_object().unwrap().len(), 1);
}

#[test]
fn set_non_object_errors() {
    let result = call_err(&ObjModule, "set", &[n(42.0), s("a"), n(1.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn set_non_string_key_errors() {
    let result = call_err(&ObjModule, "set", &[obj(&[]), n(42.0), n(1.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn set_no_args_errors() {
    let result = call_err(&ObjModule, "set", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── update ────────────────────────────────────────────────────────

#[test]
fn update_existing_key() {
    let result = call_with_eval(
        &ObjModule,
        "update",
        &[
            obj(&[("a", serde_json::json!(1))]),
            s("a"),
            fn_value("add(x, 1)", &["x"]),
        ],
    );
    assert_eq!(
        result.as_object().unwrap().get("a").unwrap().as_f64(),
        Some(2.0)
    );
}

#[test]
fn update_missing_key_passes_null() {
    let result = call_with_eval(
        &ObjModule,
        "update",
        &[
            obj(&[("a", serde_json::json!(1))]),
            s("b"),
            fn_value("add(x, 0)", &["x"]),
        ],
    );
    let obj = result.as_object().unwrap();
    assert_eq!(obj.get("a").unwrap().as_f64(), Some(1.0));
    assert_eq!(obj.get("b").unwrap().as_f64(), Some(0.0));
}

#[test]
fn update_preserves_other_keys() {
    let result = call_with_eval(
        &ObjModule,
        "update",
        &[
            obj(&[("a", serde_json::json!(1)), ("b", serde_json::json!(2))]),
            s("a"),
            fn_value("add(x, 10)", &["x"]),
        ],
    );
    let obj = result.as_object().unwrap();
    assert_eq!(obj.len(), 2);
    assert_eq!(obj.get("a").unwrap().as_f64(), Some(11.0));
    assert_eq!(obj.get("b").unwrap().as_f64(), Some(2.0));
}

#[test]
fn update_without_eval_errors() {
    let result = call_err(
        &ObjModule,
        "update",
        &[
            obj(&[("a", serde_json::json!(1))]),
            s("a"),
            fn_value("add(x, 1)", &["x"]),
        ],
    );
    assert!(matches!(result, Err(Error::PrimitiveError(_))));
}

#[test]
fn update_non_object_errors() {
    let result = call_err_with_eval(
        &ObjModule,
        "update",
        &[n(42.0), s("a"), fn_value("add(x, 1)", &["x"])],
    );
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn update_non_function_errors() {
    let result = call_err_with_eval(
        &ObjModule,
        "update",
        &[obj(&[("a", serde_json::json!(1))]), s("a"), n(42.0)],
    );
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn update_no_args_errors() {
    let result = call_err_with_eval(&ObjModule, "update", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── has_key ───────────────────────────────────────────────────────

#[test]
fn has_key_true_when_exists() {
    let result = call(
        &ObjModule,
        "has_key",
        &[obj(&[("a", serde_json::json!(1))]), s("a")],
    );
    assert_eq!(result.as_bool(), Some(true));
}

#[test]
fn has_key_false_when_missing() {
    let result = call(
        &ObjModule,
        "has_key",
        &[obj(&[("a", serde_json::json!(1))]), s("b")],
    );
    assert_eq!(result.as_bool(), Some(false));
}

#[test]
fn has_key_false_for_empty_object() {
    let result = call(&ObjModule, "has_key", &[obj(&[]), s("a")]);
    assert_eq!(result.as_bool(), Some(false));
}

#[test]
fn has_key_non_object_errors() {
    let result = call_err(&ObjModule, "has_key", &[n(42.0), s("a")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn has_key_non_string_key_errors() {
    let result = call_err(&ObjModule, "has_key", &[obj(&[]), n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn has_key_no_args_errors() {
    let result = call_err(&ObjModule, "has_key", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── map_values ────────────────────────────────────────────────────

#[test]
fn map_values_transforms_values() {
    let result = call_with_eval(
        &ObjModule,
        "map_values",
        &[
            fn_value("add(x, 1)", &["x"]),
            obj(&[("a", serde_json::json!(1)), ("b", serde_json::json!(2))]),
        ],
    );
    let obj = result.as_object().unwrap();
    assert_eq!(obj.get("a").unwrap().as_f64(), Some(2.0));
    assert_eq!(obj.get("b").unwrap().as_f64(), Some(3.0));
}

#[test]
fn map_values_empty_object() {
    let result = call_with_eval(
        &ObjModule,
        "map_values",
        &[fn_value("add(x, 1)", &["x"]), obj(&[])],
    );
    assert!(result.as_object().unwrap().is_empty());
}

#[test]
fn map_values_preserves_keys() {
    let result = call_with_eval(
        &ObjModule,
        "map_values",
        &[
            fn_value("add(x, 10)", &["x"]),
            obj(&[("a", serde_json::json!(1)), ("b", serde_json::json!(2))]),
        ],
    );
    let obj = result.as_object().unwrap();
    assert_eq!(obj.len(), 2);
    assert!(obj.contains_key("a"));
    assert!(obj.contains_key("b"));
}

#[test]
fn map_values_without_eval_errors() {
    let result = call_err(
        &ObjModule,
        "map_values",
        &[
            fn_value("add(x, 1)", &["x"]),
            obj(&[("a", serde_json::json!(1))]),
        ],
    );
    assert!(matches!(result, Err(Error::PrimitiveError(_))));
}

#[test]
fn map_values_non_function_errors() {
    let result = call_err_with_eval(
        &ObjModule,
        "map_values",
        &[n(42.0), obj(&[("a", serde_json::json!(1))])],
    );
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn map_values_non_object_errors() {
    let result = call_err_with_eval(
        &ObjModule,
        "map_values",
        &[fn_value("add(x, 1)", &["x"]), n(42.0)],
    );
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn map_values_no_args_errors() {
    let result = call_err_with_eval(&ObjModule, "map_values", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── map_keys ──────────────────────────────────────────────────────

#[test]
fn map_keys_transforms_keys() {
    let result = call_with_eval(
        &ObjModule,
        "map_keys",
        &[
            fn_value("concat(x, \"_new\")", &["x"]),
            obj(&[("a", serde_json::json!(1)), ("b", serde_json::json!(2))]),
        ],
    );
    let obj = result.as_object().unwrap();
    assert_eq!(obj.len(), 2);
    assert_eq!(obj.get("a_new").unwrap().as_f64(), Some(1.0));
    assert_eq!(obj.get("b_new").unwrap().as_f64(), Some(2.0));
}

#[test]
fn map_keys_empty_object() {
    let result = call_with_eval(
        &ObjModule,
        "map_keys",
        &[fn_value("concat(x, \"_new\")", &["x"]), obj(&[])],
    );
    assert!(result.as_object().unwrap().is_empty());
}

#[test]
fn map_keys_preserves_values() {
    let result = call_with_eval(
        &ObjModule,
        "map_keys",
        &[
            fn_value("concat(x, \"_x\")", &["x"]),
            obj(&[("a", serde_json::json!(42))]),
        ],
    );
    let obj = result.as_object().unwrap();
    assert_eq!(obj.get("a_x").unwrap().as_f64(), Some(42.0));
}

#[test]
fn map_keys_without_eval_errors() {
    let result = call_err(
        &ObjModule,
        "map_keys",
        &[
            fn_value("concat(x, \"_new\")", &["x"]),
            obj(&[("a", serde_json::json!(1))]),
        ],
    );
    assert!(matches!(result, Err(Error::PrimitiveError(_))));
}

#[test]
fn map_keys_non_function_errors() {
    let result = call_err_with_eval(
        &ObjModule,
        "map_keys",
        &[n(42.0), obj(&[("a", serde_json::json!(1))])],
    );
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn map_keys_non_object_errors() {
    let result = call_err_with_eval(
        &ObjModule,
        "map_keys",
        &[fn_value("concat(x, \"_new\")", &["x"]), n(42.0)],
    );
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn map_keys_no_args_errors() {
    let result = call_err_with_eval(&ObjModule, "map_keys", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── filter_keys ───────────────────────────────────────────────────

#[test]
fn filter_keys_keeps_all_non_empty() {
    let result = call_with_eval(
        &ObjModule,
        "filter_keys",
        &[
            fn_value("len(x)", &["x"]),
            obj(&[("a", serde_json::json!(1)), ("bb", serde_json::json!(2))]),
        ],
    );
    let obj = result.as_object().unwrap();
    assert_eq!(obj.len(), 2);
}

#[test]
fn filter_keys_filters_all_out() {
    let result = call_with_eval(
        &ObjModule,
        "filter_keys",
        &[
            fn_value("mul(0, 0)", &["x"]),
            obj(&[("a", serde_json::json!(1)), ("b", serde_json::json!(2))]),
        ],
    );
    assert!(result.as_object().unwrap().is_empty());
}

#[test]
fn filter_keys_empty_object() {
    let result = call_with_eval(
        &ObjModule,
        "filter_keys",
        &[fn_value("len(x)", &["x"]), obj(&[])],
    );
    assert!(result.as_object().unwrap().is_empty());
}

#[test]
fn filter_keys_without_eval_errors() {
    let result = call_err(
        &ObjModule,
        "filter_keys",
        &[
            fn_value("len(x)", &["x"]),
            obj(&[("a", serde_json::json!(1))]),
        ],
    );
    assert!(matches!(result, Err(Error::PrimitiveError(_))));
}

#[test]
fn filter_keys_non_function_errors() {
    let result = call_err_with_eval(
        &ObjModule,
        "filter_keys",
        &[n(42.0), obj(&[("a", serde_json::json!(1))])],
    );
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn filter_keys_non_object_errors() {
    let result = call_err_with_eval(
        &ObjModule,
        "filter_keys",
        &[fn_value("len(x)", &["x"]), n(42.0)],
    );
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn filter_keys_no_args_errors() {
    let result = call_err_with_eval(&ObjModule, "filter_keys", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── filter_values ─────────────────────────────────────────────────

#[test]
fn filter_values_keeps_truthy() {
    let result = call_with_eval(
        &ObjModule,
        "filter_values",
        &[
            fn_value("add(x, 0)", &["x"]),
            obj(&[
                ("a", serde_json::json!(1)),
                ("b", serde_json::json!(0)),
                ("c", serde_json::json!(3)),
            ]),
        ],
    );
    let obj = result.as_object().unwrap();
    assert_eq!(obj.len(), 2);
    assert!(obj.contains_key("a"));
    assert!(obj.contains_key("c"));
    assert!(!obj.contains_key("b"));
}

#[test]
fn filter_values_filters_all_out() {
    let result = call_with_eval(
        &ObjModule,
        "filter_values",
        &[
            fn_value("mul(0, 0)", &["x"]),
            obj(&[("a", serde_json::json!(1)), ("b", serde_json::json!(2))]),
        ],
    );
    assert!(result.as_object().unwrap().is_empty());
}

#[test]
fn filter_values_empty_object() {
    let result = call_with_eval(
        &ObjModule,
        "filter_values",
        &[fn_value("add(x, 0)", &["x"]), obj(&[])],
    );
    assert!(result.as_object().unwrap().is_empty());
}

#[test]
fn filter_values_without_eval_errors() {
    let result = call_err(
        &ObjModule,
        "filter_values",
        &[
            fn_value("add(x, 0)", &["x"]),
            obj(&[("a", serde_json::json!(1))]),
        ],
    );
    assert!(matches!(result, Err(Error::PrimitiveError(_))));
}

#[test]
fn filter_values_non_function_errors() {
    let result = call_err_with_eval(
        &ObjModule,
        "filter_values",
        &[n(42.0), obj(&[("a", serde_json::json!(1))])],
    );
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn filter_values_non_object_errors() {
    let result = call_err_with_eval(
        &ObjModule,
        "filter_values",
        &[fn_value("add(x, 0)", &["x"]), n(42.0)],
    );
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn filter_values_no_args_errors() {
    let result = call_err_with_eval(&ObjModule, "filter_values", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── invert ────────────────────────────────────────────────────────

#[test]
fn invert_string_values() {
    let result = call(
        &ObjModule,
        "invert",
        &[obj(&[
            ("a", serde_json::json!("b")),
            ("c", serde_json::json!("d")),
        ])],
    );
    let obj = result.as_object().unwrap();
    assert_eq!(obj.len(), 2);
    assert_eq!(obj.get("b").unwrap().as_str(), Some("a"));
    assert_eq!(obj.get("d").unwrap().as_str(), Some("c"));
}

#[test]
fn invert_empty_object() {
    let result = call(&ObjModule, "invert", &[obj(&[])]);
    assert!(result.as_object().unwrap().is_empty());
}

#[test]
fn invert_number_values_coerced_to_string() {
    let result = call(&ObjModule, "invert", &[obj(&[("a", serde_json::json!(1))])]);
    let obj = result.as_object().unwrap();
    assert_eq!(obj.get("1").unwrap().as_str(), Some("a"));
}

#[test]
fn invert_non_object_errors() {
    let result = call_err(&ObjModule, "invert", &[n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn invert_no_args_errors() {
    let result = call_err(&ObjModule, "invert", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── path_or ───────────────────────────────────────────────────────

#[test]
fn path_or_returns_value_when_exists() {
    let value = obj(&[("a", serde_json::json!({"b": 1}))]);
    let result = call(&ObjModule, "path_or", &[value, s("a.b"), n(0.0)]);
    assert_eq!(result.as_number(), Some(1.0));
}

#[test]
fn path_or_returns_default_when_missing() {
    let value = obj(&[("a", serde_json::json!({"b": 1}))]);
    let result = call(&ObjModule, "path_or", &[value, s("a.c"), n(99.0)]);
    assert_eq!(result.as_number(), Some(99.0));
}

#[test]
fn path_or_returns_default_on_non_object() {
    let result = call(&ObjModule, "path_or", &[n(42.0), s("a"), n(99.0)]);
    assert_eq!(result.as_number(), Some(99.0));
}

#[test]
fn path_or_deeply_nested() {
    let value = obj(&[("a", serde_json::json!({"b": serde_json::json!({"c": 42})}))]);
    let result = call(&ObjModule, "path_or", &[value, s("a.b.c"), n(0.0)]);
    assert_eq!(result.as_number(), Some(42.0));
}

#[test]
fn path_or_non_string_path_errors() {
    let result = call_err(&ObjModule, "path_or", &[obj(&[]), n(42.0), n(0.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn path_or_no_args_errors() {
    let result = call_err(&ObjModule, "path_or", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── set_path ──────────────────────────────────────────────────────

#[test]
fn set_path_sets_nested_key() {
    let value = obj(&[("a", serde_json::json!({"b": 1}))]);
    let result = call(&ObjModule, "set_path", &[value, s("a.c"), n(2.0)]);
    let obj = result.as_object().unwrap();
    let a = obj.get("a").unwrap().as_object().unwrap();
    assert_eq!(a.get("b").unwrap().as_f64(), Some(1.0));
    assert_eq!(a.get("c").unwrap().as_f64(), Some(2.0));
}

#[test]
fn set_path_creates_intermediate_objects() {
    let result = call(&ObjModule, "set_path", &[obj(&[]), s("a.b.c"), n(42.0)]);
    let obj = result.as_object().unwrap();
    let a = obj.get("a").unwrap().as_object().unwrap();
    let b = a.get("b").unwrap().as_object().unwrap();
    assert_eq!(b.get("c").unwrap().as_f64(), Some(42.0));
}

#[test]
fn set_path_overwrites_leaf() {
    let value = obj(&[("a", serde_json::json!(1))]);
    let result = call(&ObjModule, "set_path", &[value, s("a"), n(99.0)]);
    assert_eq!(
        result.as_object().unwrap().get("a").unwrap().as_f64(),
        Some(99.0)
    );
}

#[test]
fn set_path_preserves_original() {
    let original = obj(&[("a", serde_json::json!({"b": 1}))]);
    let _ = call(
        &ObjModule,
        "set_path",
        &[original.clone(), s("a.c"), n(2.0)],
    );
    assert_eq!(
        original
            .as_object()
            .unwrap()
            .get("a")
            .unwrap()
            .as_object()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn set_path_non_string_path_errors() {
    let result = call_err(&ObjModule, "set_path", &[obj(&[]), n(42.0), n(1.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn set_path_no_args_errors() {
    let result = call_err(&ObjModule, "set_path", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── update_path ───────────────────────────────────────────────────

#[test]
fn update_path_existing_value() {
    let value = obj(&[("a", serde_json::json!({"b": 1}))]);
    let result = call_with_eval(
        &ObjModule,
        "update_path",
        &[value, s("a.b"), fn_value("add(x, 1)", &["x"])],
    );
    let obj = result.as_object().unwrap();
    assert_eq!(
        obj.get("a")
            .unwrap()
            .as_object()
            .unwrap()
            .get("b")
            .unwrap()
            .as_f64(),
        Some(2.0)
    );
}

#[test]
fn update_path_missing_passes_null() {
    let value = obj(&[("a", serde_json::json!({"b": 1}))]);
    let result = call_with_eval(
        &ObjModule,
        "update_path",
        &[value, s("a.c"), fn_value("add(x, 0)", &["x"])],
    );
    let obj = result.as_object().unwrap();
    assert_eq!(
        obj.get("a")
            .unwrap()
            .as_object()
            .unwrap()
            .get("c")
            .unwrap()
            .as_f64(),
        Some(0.0)
    );
}

#[test]
fn update_path_creates_intermediate() {
    let result = call_with_eval(
        &ObjModule,
        "update_path",
        &[obj(&[]), s("a.b"), fn_value("add(x, 42)", &["x"])],
    );
    let obj = result.as_object().unwrap();
    assert_eq!(
        obj.get("a")
            .unwrap()
            .as_object()
            .unwrap()
            .get("b")
            .unwrap()
            .as_f64(),
        Some(42.0)
    );
}

#[test]
fn update_path_without_eval_errors() {
    let result = call_err(
        &ObjModule,
        "update_path",
        &[
            obj(&[("a", serde_json::json!(1))]),
            s("a"),
            fn_value("add(x, 1)", &["x"]),
        ],
    );
    assert!(matches!(result, Err(Error::PrimitiveError(_))));
}

#[test]
fn update_path_non_string_path_errors() {
    let result = call_err_with_eval(
        &ObjModule,
        "update_path",
        &[obj(&[]), n(42.0), fn_value("add(x, 1)", &["x"])],
    );
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn update_path_non_function_errors() {
    let result = call_err_with_eval(
        &ObjModule,
        "update_path",
        &[obj(&[("a", serde_json::json!(1))]), s("a"), n(42.0)],
    );
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn update_path_no_args_errors() {
    let result = call_err_with_eval(&ObjModule, "update_path", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── size ──────────────────────────────────────────────────────────

#[test]
fn size_array() {
    let result = call(
        &ObjModule,
        "size",
        &[arr(&[
            serde_json::json!(1),
            serde_json::json!(2),
            serde_json::json!(3),
        ])],
    );
    assert_eq!(result.as_number(), Some(3.0));
}

#[test]
fn size_object() {
    let result = call(
        &ObjModule,
        "size",
        &[obj(&[
            ("a", serde_json::json!(1)),
            ("b", serde_json::json!(2)),
        ])],
    );
    assert_eq!(result.as_number(), Some(2.0));
}

#[test]
fn size_string() {
    let result = call(&ObjModule, "size", &[s("hello")]);
    assert_eq!(result.as_number(), Some(5.0));
}

#[test]
fn size_empty_array() {
    let result = call(&ObjModule, "size", &[arr(&[])]);
    assert_eq!(result.as_number(), Some(0.0));
}

#[test]
fn size_empty_object() {
    let result = call(&ObjModule, "size", &[obj(&[])]);
    assert_eq!(result.as_number(), Some(0.0));
}

#[test]
fn size_empty_string() {
    let result = call(&ObjModule, "size", &[s("")]);
    assert_eq!(result.as_number(), Some(0.0));
}

#[test]
fn size_null_returns_zero() {
    let result = call(&ObjModule, "size", &[null()]);
    assert_eq!(result.as_number(), Some(0.0));
}

#[test]
fn size_number_errors() {
    let result = call_err(&ObjModule, "size", &[n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn size_no_args_errors() {
    let result = call_err(&ObjModule, "size", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── clone_deep ────────────────────────────────────────────────────

#[test]
fn clone_deep_object() {
    let value = obj(&[("a", serde_json::json!({"b": [1, 2]}))]);
    let result = call(&ObjModule, "clone_deep", &[value]);
    let obj = result.as_object().unwrap();
    let a = obj.get("a").unwrap().as_object().unwrap();
    let b = a.get("b").unwrap().as_array().unwrap();
    assert_eq!(b.len(), 2);
    assert_eq!(b[0].as_f64(), Some(1.0));
    assert_eq!(b[1].as_f64(), Some(2.0));
}

#[test]
fn clone_deep_array() {
    let value = arr(&[serde_json::json!([1, 2]), serde_json::json!({"a": 3})]);
    let result = call(&ObjModule, "clone_deep", &[value]);
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0].as_array().unwrap()[0].as_f64(), Some(1.0));
    assert_eq!(
        arr[1].as_object().unwrap().get("a").unwrap().as_f64(),
        Some(3.0)
    );
}

#[test]
fn clone_deep_primitive() {
    let result = call(&ObjModule, "clone_deep", &[n(42.0)]);
    assert_eq!(result.as_number(), Some(42.0));
}

#[test]
fn clone_deep_null() {
    let result = call(&ObjModule, "clone_deep", &[null()]);
    assert!(result.is_null());
}

#[test]
fn clone_deep_string() {
    let result = call(&ObjModule, "clone_deep", &[s("hello")]);
    assert_eq!(result.as_str(), Some("hello"));
}

#[test]
fn clone_deep_no_args_errors() {
    let result = call_err(&ObjModule, "clone_deep", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}
