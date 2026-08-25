use handl_vm_values_json::Error;

use super::test_helpers::*;

// ── keys ──────────────────────────────────────────────────────────

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

// ── values ────────────────────────────────────────────────────────

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

// ── entries ───────────────────────────────────────────────────────

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
