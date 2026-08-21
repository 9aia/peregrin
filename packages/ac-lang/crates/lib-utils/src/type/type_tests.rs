use ac_runtime::NonFunctionValue;
use ac_runtime_core::Module;
use ac_vm::{Context, Error};

use super::r#type::TypeModule;

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
    assert_eq!(
        call(&TypeModule, "to_string", &[s("")]).as_str(),
        Some("")
    );
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
    let result = call(&TypeModule, "to_string", &[arr(&[
        serde_json::json!("a"),
        serde_json::json!("b"),
    ])]);
    assert_eq!(result.as_str(), Some("[\"a\",\"b\"]"));
}

#[test]
fn to_string_from_empty_array() {
    let result = call(&TypeModule, "to_string", &[arr(&[])]);
    assert_eq!(result.as_str(), Some("[]"));
}

#[test]
fn to_string_from_object() {
    let result = call(&TypeModule, "to_string", &[obj(&[("a", serde_json::json!(1))])]);
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
fn to_number_from_array_returns_length() {
    assert_eq!(
        call(&TypeModule, "to_number", &[arr(&[
            serde_json::json!(1),
            serde_json::json!(2),
            serde_json::json!(3),
        ])])
        .as_number(),
        Some(3.0)
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
fn to_number_from_object_returns_key_count() {
    assert_eq!(
        call(&TypeModule, "to_number", &[obj(&[
            ("a", serde_json::json!(1)),
            ("b", serde_json::json!(2)),
        ])])
        .as_number(),
        Some(2.0)
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
        call(&TypeModule, "to_bool", &[obj(&[("a", serde_json::json!(1))])]).as_bool(),
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
