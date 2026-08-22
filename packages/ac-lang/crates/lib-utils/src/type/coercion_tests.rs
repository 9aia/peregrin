use ac_runtime::NonFunctionValue;
use ac_runtime_core::Module;
use ac_vm::{Context, Error};

use super::TypeModule;

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

fn s(v: &str) -> NonFunctionValue {
    NonFunctionValue::new(v.to_string())
}

fn n(v: f64) -> NonFunctionValue {
    NonFunctionValue::new(serde_json::json!(v))
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
    for (key, value) in pairs {
        map.insert(key.to_string(), value.clone());
    }
    NonFunctionValue::new(serde_json::Value::Object(map))
}

// ── module metadata ──────────────────────────────────────────────

#[test]
fn module_name() {
    assert_eq!(TypeModule.name(), "type");
}

#[test]
fn unknown_function_returns_none() {
    assert!(TypeModule.function("nope").is_none());
}

// ── to_string ────────────────────────────────────────────────────

#[test]
fn to_string_from_string() {
    assert_eq!(
        call(&TypeModule, "to_string", &[s("hello")]).as_str(),
        Some("hello")
    );
}

#[test]
fn to_string_from_empty_string() {
    assert_eq!(call(&TypeModule, "to_string", &[s("")]).as_str(), Some(""));
}

#[test]
fn to_string_from_integer() {
    assert_eq!(
        call(&TypeModule, "to_string", &[n(42.0)]).as_str(),
        Some("42")
    );
}

#[test]
fn to_string_from_float() {
    assert_eq!(
        call(&TypeModule, "to_string", &[n(3.14)]).as_str(),
        Some("3.14")
    );
}

#[test]
fn to_string_from_bool_true() {
    assert_eq!(
        call(&TypeModule, "to_string", &[b(true)]).as_str(),
        Some("true")
    );
}

#[test]
fn to_string_from_bool_false() {
    assert_eq!(
        call(&TypeModule, "to_string", &[b(false)]).as_str(),
        Some("false")
    );
}

#[test]
fn to_string_from_null() {
    assert_eq!(
        call(&TypeModule, "to_string", &[null()]).as_str(),
        Some("null")
    );
}

#[test]
fn to_string_from_array() {
    let result = call(
        &TypeModule,
        "to_string",
        &[arr(&[serde_json::json!("a"), serde_json::json!("b")])],
    );
    assert_eq!(result.as_str(), Some("[\"a\",\"b\"]"));
}

#[test]
fn to_string_from_empty_array() {
    let result = call(&TypeModule, "to_string", &[arr(&[])]);
    assert_eq!(result.as_str(), Some("[]"));
}

#[test]
fn to_string_from_object() {
    let result = call(
        &TypeModule,
        "to_string",
        &[obj(&[("a", serde_json::json!(1))])],
    );
    assert_eq!(result.as_str(), Some("{\"a\":1}"));
}

#[test]
fn to_string_from_empty_object() {
    let result = call(&TypeModule, "to_string", &[obj(&[])]);
    assert_eq!(result.as_str(), Some("{}"));
}

#[test]
fn to_string_no_args_errors() {
    let result = call_err(&TypeModule, "to_string", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── to_number ────────────────────────────────────────────────────

#[test]
fn to_number_from_number() {
    assert_eq!(
        call(&TypeModule, "to_number", &[n(42.0)]).as_number(),
        Some(42.0)
    );
}

#[test]
fn to_number_from_negative_number() {
    assert_eq!(
        call(&TypeModule, "to_number", &[n(-7.5)]).as_number(),
        Some(-7.5)
    );
}

#[test]
fn to_number_from_zero() {
    assert_eq!(
        call(&TypeModule, "to_number", &[n(0.0)]).as_number(),
        Some(0.0)
    );
}

#[test]
fn to_number_from_bool_true() {
    assert_eq!(
        call(&TypeModule, "to_number", &[b(true)]).as_number(),
        Some(1.0)
    );
}

#[test]
fn to_number_from_bool_false() {
    assert_eq!(
        call(&TypeModule, "to_number", &[b(false)]).as_number(),
        Some(0.0)
    );
}

#[test]
fn to_number_from_string_integer() {
    assert_eq!(
        call(&TypeModule, "to_number", &[s("42")]).as_number(),
        Some(42.0)
    );
}

#[test]
fn to_number_from_string_float() {
    assert_eq!(
        call(&TypeModule, "to_number", &[s("3.14")]).as_number(),
        Some(3.14)
    );
}

#[test]
fn to_number_from_string_negative() {
    assert_eq!(
        call(&TypeModule, "to_number", &[s("-5")]).as_number(),
        Some(-5.0)
    );
}

#[test]
fn to_number_from_string_whitespace() {
    assert_eq!(
        call(&TypeModule, "to_number", &[s("  42  ")]).as_number(),
        Some(42.0)
    );
}

#[test]
fn to_number_from_string_invalid() {
    assert_eq!(
        call(&TypeModule, "to_number", &[s("hello")]).as_number(),
        Some(0.0)
    );
}

#[test]
fn to_number_from_empty_string() {
    assert_eq!(
        call(&TypeModule, "to_number", &[s("")]).as_number(),
        Some(0.0)
    );
}

#[test]
fn to_number_from_null() {
    assert_eq!(
        call(&TypeModule, "to_number", &[null()]).as_number(),
        Some(0.0)
    );
}

#[test]
fn to_number_from_array_returns_zero() {
    assert_eq!(
        call(
            &TypeModule,
            "to_number",
            &[arr(&[
                serde_json::json!(1),
                serde_json::json!(2),
                serde_json::json!(3),
            ])]
        )
        .as_number(),
        Some(0.0)
    );
}

#[test]
fn to_number_from_empty_array() {
    assert_eq!(
        call(&TypeModule, "to_number", &[arr(&[])]).as_number(),
        Some(0.0)
    );
}

#[test]
fn to_number_from_object_returns_zero() {
    assert_eq!(
        call(
            &TypeModule,
            "to_number",
            &[obj(&[
                ("a", serde_json::json!(1)),
                ("b", serde_json::json!(2)),
            ])]
        )
        .as_number(),
        Some(0.0)
    );
}

#[test]
fn to_number_from_empty_object() {
    assert_eq!(
        call(&TypeModule, "to_number", &[obj(&[])]).as_number(),
        Some(0.0)
    );
}

#[test]
fn to_number_no_args_errors() {
    let result = call_err(&TypeModule, "to_number", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── to_bool ──────────────────────────────────────────────────────

#[test]
fn to_bool_from_bool_true() {
    assert_eq!(
        call(&TypeModule, "to_bool", &[b(true)]).as_bool(),
        Some(true)
    );
}

#[test]
fn to_bool_from_bool_false() {
    assert_eq!(
        call(&TypeModule, "to_bool", &[b(false)]).as_bool(),
        Some(false)
    );
}

#[test]
fn to_bool_from_number_nonzero() {
    assert_eq!(
        call(&TypeModule, "to_bool", &[n(42.0)]).as_bool(),
        Some(true)
    );
}

#[test]
fn to_bool_from_number_zero() {
    assert_eq!(
        call(&TypeModule, "to_bool", &[n(0.0)]).as_bool(),
        Some(false)
    );
}

#[test]
fn to_bool_from_number_negative() {
    assert_eq!(
        call(&TypeModule, "to_bool", &[n(-1.0)]).as_bool(),
        Some(true)
    );
}

#[test]
fn to_bool_from_string_nonempty() {
    assert_eq!(
        call(&TypeModule, "to_bool", &[s("hello")]).as_bool(),
        Some(true)
    );
}

#[test]
fn to_bool_from_string_empty() {
    assert_eq!(
        call(&TypeModule, "to_bool", &[s("")]).as_bool(),
        Some(false)
    );
}

#[test]
fn to_bool_from_null() {
    assert_eq!(
        call(&TypeModule, "to_bool", &[null()]).as_bool(),
        Some(false)
    );
}

#[test]
fn to_bool_from_array_nonempty() {
    assert_eq!(
        call(&TypeModule, "to_bool", &[arr(&[serde_json::json!(1)])]).as_bool(),
        Some(true)
    );
}

#[test]
fn to_bool_from_array_empty() {
    assert_eq!(
        call(&TypeModule, "to_bool", &[arr(&[])]).as_bool(),
        Some(false)
    );
}

#[test]
fn to_bool_from_object_nonempty() {
    assert_eq!(
        call(
            &TypeModule,
            "to_bool",
            &[obj(&[("a", serde_json::json!(1))])]
        )
        .as_bool(),
        Some(true)
    );
}

#[test]
fn to_bool_from_object_empty() {
    assert_eq!(
        call(&TypeModule, "to_bool", &[obj(&[])]).as_bool(),
        Some(false)
    );
}

#[test]
fn to_bool_no_args_errors() {
    let result = call_err(&TypeModule, "to_bool", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── edge cases: large numbers, NaN, Infinity ─────────────────────

#[test]
fn to_string_from_large_integer() {
    let result = call(&TypeModule, "to_string", &[n(1e20)]);
    assert_eq!(result.as_str(), Some("100000000000000000000"));
}

#[test]
fn to_string_from_very_small_float() {
    let result = call(&TypeModule, "to_string", &[n(0.0001)]);
    assert_eq!(result.as_str(), Some("0.0001"));
}

#[test]
fn to_string_from_nan() {
    let result = call(
        &TypeModule,
        "to_string",
        &[NonFunctionValue::new(serde_json::json!(f64::NAN))],
    );
    // serde_json serializes NaN as null, so this becomes "null"
    assert_eq!(result.as_str(), Some("null"));
}

#[test]
fn to_string_from_infinity() {
    let result = call(
        &TypeModule,
        "to_string",
        &[NonFunctionValue::new(serde_json::json!(f64::INFINITY))],
    );
    // serde_json serializes Infinity as null
    assert_eq!(result.as_str(), Some("null"));
}

#[test]
fn to_string_from_nested_array() {
    let result = call(
        &TypeModule,
        "to_string",
        &[arr(&[serde_json::json!([1, 2])])],
    );
    assert_eq!(result.as_str(), Some("[[1,2]]"));
}

#[test]
fn to_string_from_nested_object() {
    let result = call(
        &TypeModule,
        "to_string",
        &[obj(&[("a", serde_json::json!({"b": 2}))])],
    );
    assert_eq!(result.as_str(), Some("{\"a\":{\"b\":2}}"));
}

#[test]
fn to_string_from_array_with_mixed_types() {
    let result = call(
        &TypeModule,
        "to_string",
        &[arr(&[
            serde_json::json!(1),
            serde_json::json!("two"),
            serde_json::json!(true),
            serde_json::json!(null),
        ])],
    );
    assert_eq!(result.as_str(), Some("[1,\"two\",true,null]"));
}

#[test]
fn to_number_from_string_inf_returns_zero() {
    assert_eq!(
        call(&TypeModule, "to_number", &[s("inf")]).as_number(),
        Some(0.0)
    );
}

#[test]
fn to_number_from_string_infinity_returns_zero() {
    assert_eq!(
        call(&TypeModule, "to_number", &[s("infinity")]).as_number(),
        Some(0.0)
    );
}

#[test]
fn to_number_from_string_nan_returns_zero() {
    assert_eq!(
        call(&TypeModule, "to_number", &[s("nan")]).as_number(),
        Some(0.0)
    );
}

#[test]
fn to_number_from_string_scientific_notation() {
    assert_eq!(
        call(&TypeModule, "to_number", &[s("1e5")]).as_number(),
        Some(100000.0)
    );
}

#[test]
fn to_number_from_string_with_leading_plus() {
    assert_eq!(
        call(&TypeModule, "to_number", &[s("+42")]).as_number(),
        Some(42.0)
    );
}

#[test]
fn to_number_from_string_float_with_exponent() {
    assert_eq!(
        call(&TypeModule, "to_number", &[s("1.5e3")]).as_number(),
        Some(1500.0)
    );
}

#[test]
fn to_number_from_nested_array_returns_zero() {
    assert_eq!(
        call(
            &TypeModule,
            "to_number",
            &[arr(&[serde_json::json!([1, 2]), serde_json::json!("x"),])]
        )
        .as_number(),
        Some(0.0)
    );
}

#[test]
fn to_number_from_nested_object_returns_zero() {
    assert_eq!(
        call(
            &TypeModule,
            "to_number",
            &[obj(&[
                ("a", serde_json::json!({"inner": 1})),
                ("b", serde_json::json!(2)),
                ("c", serde_json::json!([1, 2, 3])),
            ])]
        )
        .as_number(),
        Some(0.0)
    );
}

#[test]
fn to_bool_from_nan() {
    let result = call(
        &TypeModule,
        "to_bool",
        &[NonFunctionValue::new(serde_json::json!(f64::NAN))],
    );
    // serde_json serializes NaN as null, so is_null() is true → false
    assert_eq!(result.as_bool(), Some(false));
}

#[test]
fn to_bool_from_infinity() {
    let result = call(
        &TypeModule,
        "to_bool",
        &[NonFunctionValue::new(serde_json::json!(f64::INFINITY))],
    );
    // serde_json serializes Infinity as null → false
    assert_eq!(result.as_bool(), Some(false));
}

#[test]
fn to_bool_from_nested_array_nonempty() {
    assert_eq!(
        call(&TypeModule, "to_bool", &[arr(&[serde_json::json!([1])])]).as_bool(),
        Some(true)
    );
}

#[test]
fn to_bool_from_nested_object_nonempty() {
    assert_eq!(
        call(
            &TypeModule,
            "to_bool",
            &[obj(&[("a", serde_json::json!({}))])]
        )
        .as_bool(),
        Some(true)
    );
}

// ── extra args silently ignored (consistent with other modules) ──

#[test]
fn to_string_extra_args_ignored() {
    assert_eq!(
        call(&TypeModule, "to_string", &[s("hi"), n(42.0)]).as_str(),
        Some("hi")
    );
}

#[test]
fn to_number_extra_args_ignored() {
    assert_eq!(
        call(&TypeModule, "to_number", &[n(1.0), n(2.0)]).as_number(),
        Some(1.0)
    );
}

#[test]
fn to_bool_extra_args_ignored() {
    assert_eq!(
        call(&TypeModule, "to_bool", &[b(true), b(false)]).as_bool(),
        Some(true)
    );
}

// ── VM integration tests ─────────────────────────────────────────

use ac_ast::Expr;
use ac_runtime_core::ModuleHost;
use ac_vm::{Host, Vm};

fn eval_expr(expr: &Expr<NonFunctionValue>) -> NonFunctionValue {
    let host = ModuleHost::new().with(TypeModule);
    let vm = Vm::new(host);
    let mut ctx = Context::new();
    vm.eval(expr, &mut ctx).expect("eval succeeds")
}

fn eval_expr_result(expr: &Expr<NonFunctionValue>) -> Result<NonFunctionValue, Error> {
    let host = ModuleHost::new().with(TypeModule);
    let vm = Vm::new(host);
    let mut ctx = Context::new();
    vm.eval(expr, &mut ctx)
}

#[test]
fn vm_to_string_from_number() {
    let expr = Expr::call("type.to_string", vec![Expr::value(n(42.0))]);
    let result = eval_expr(&expr);
    assert_eq!(result.as_str(), Some("42"));
}

#[test]
fn vm_to_string_from_bool() {
    let expr = Expr::call("type.to_string", vec![Expr::value(b(true))]);
    let result = eval_expr(&expr);
    assert_eq!(result.as_str(), Some("true"));
}

#[test]
fn vm_to_string_from_null() {
    let expr = Expr::call("type.to_string", vec![Expr::value(null())]);
    let result = eval_expr(&expr);
    assert_eq!(result.as_str(), Some("null"));
}

#[test]
fn vm_to_number_from_string() {
    let expr = Expr::call("type.to_number", vec![Expr::value(s("3.14"))]);
    let result = eval_expr(&expr);
    assert_eq!(result.as_number(), Some(3.14));
}

#[test]
fn vm_to_number_from_bool() {
    let expr = Expr::call("type.to_number", vec![Expr::value(b(true))]);
    let result = eval_expr(&expr);
    assert_eq!(result.as_number(), Some(1.0));
}

#[test]
fn vm_to_bool_from_number() {
    let expr = Expr::call("type.to_bool", vec![Expr::value(n(0.0))]);
    let result = eval_expr(&expr);
    assert_eq!(result.as_bool(), Some(false));
}

#[test]
fn vm_to_bool_from_string() {
    let expr = Expr::call("type.to_bool", vec![Expr::value(s("hello"))]);
    let result = eval_expr(&expr);
    assert_eq!(result.as_bool(), Some(true));
}

#[test]
fn vm_unknown_function_errors() {
    let expr = Expr::call("type.nonexistent", vec![]);
    let result = eval_expr_result(&expr);
    assert!(result.is_err());
}

#[test]
fn vm_resolves_via_module_host() {
    let host = ModuleHost::new().with(TypeModule);
    assert!(host.function("type.to_string").is_some());
    assert!(host.function("type.to_number").is_some());
    assert!(host.function("type.to_bool").is_some());
    assert!(host.function("type.nonexistent").is_none());
}
