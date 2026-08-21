use ac_vm::Error;

use super::test_helpers::*;

// ── pick ──────────────────────────────────────────────────────────

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

// ── omit ──────────────────────────────────────────────────────────

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
