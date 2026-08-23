use ac_runtime::Error;

use super::test_helpers::*;

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
