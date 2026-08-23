use ac_runtime::Error;

use super::test_helpers::*;

// ── merge ─────────────────────────────────────────────────────────

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

// ── deep_merge ────────────────────────────────────────────────────

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
