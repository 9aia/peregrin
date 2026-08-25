use handl_vm_values_json::Error;

use super::test_helpers::*;

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

// ── deep_get ──────────────────────────────────────────────────────

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
