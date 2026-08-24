use ac_runtime_values::Error;

use super::test_helpers::*;

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
            fn_value("add(0, 0)", &["x"]),
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
        &[value, s("a.c"), fn_value("add(0, 0)", &["x"])],
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
        &[obj(&[]), s("a.b"), fn_value("add(0, 42)", &["x"])],
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
