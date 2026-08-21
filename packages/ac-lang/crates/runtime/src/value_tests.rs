use super::*;

#[test]
fn new_from_string() {
    let v = NonFunctionValue::new("hello");
    assert_eq!(v.as_str(), Some("hello"));
}

#[test]
fn new_from_json() {
    let v = NonFunctionValue::new(serde_json::json!({"name": "ac"}));
    assert_eq!(v.get("name").unwrap().as_str(), Some("ac"));
}

#[test]
fn predicates() {
    let null = NonFunctionValue::new(serde_json::Value::Null);
    assert!(null.is_null());

    let boolean = NonFunctionValue::new(true);
    assert!(boolean.is_bool());
    assert_eq!(boolean.as_bool(), Some(true));

    let number = NonFunctionValue::new(42);
    assert!(number.is_number());
    assert_eq!(number.as_number(), Some(42.0));

    let string = NonFunctionValue::new("hello");
    assert!(string.is_string());

    let array = NonFunctionValue::new(serde_json::json!([1, 2, 3]));
    assert!(array.is_array());
    assert_eq!(array.as_array().unwrap().len(), 3);

    let object = NonFunctionValue::new(serde_json::json!({"key": "value"}));
    assert!(object.is_object());
}

#[test]
fn inner_and_into_inner() {
    let v = NonFunctionValue::new(serde_json::json!({"a": 1}));
    assert_eq!(v.inner().get("a").unwrap().as_i64(), Some(1));

    let inner = v.into_inner();
    assert_eq!(inner.get("a").unwrap().as_i64(), Some(1));
}

#[test]
fn inner_mut() {
    let mut v = NonFunctionValue::new(serde_json::json!({"a": 1}));
    if let Some(obj) = v.inner_mut().as_object_mut() {
        obj.insert("b".to_string(), serde_json::json!(2));
    }
    assert_eq!(v.get("b").unwrap().as_i64(), Some(2));
}

#[test]
fn default_is_null() {
    let v = NonFunctionValue::default();
    assert!(v.is_null());
}

#[test]
fn host_value_is_alias() {
    let v: HostValue = NonFunctionValue::new("test");
    assert_eq!(v.as_str(), Some("test"));
}

#[test]
fn function_value_roundtrip() {
    let fn_val = FunctionValue {
        params: vec![
            ("x".to_string(), serde_json::json!({"type": "number"})),
            ("y".to_string(), serde_json::json!({"type": "number"})),
        ],
        return_schema: Some(serde_json::json!({"type": "number"})),
        body: "mul(x, y)".to_string(),
    };
    let json = fn_val.to_json();
    let decoded = FunctionValue::from_json(&json).unwrap();
    assert_eq!(decoded, fn_val);
}

#[test]
fn function_value_without_return() {
    let fn_val = FunctionValue {
        params: vec![("x".to_string(), serde_json::json!({"type": "string"}))],
        return_schema: None,
        body: "x".to_string(),
    };
    let json = fn_val.to_json();
    let decoded = FunctionValue::from_json(&json).unwrap();
    assert_eq!(decoded.params.len(), 1);
    assert_eq!(decoded.return_schema, None);
    assert_eq!(decoded.body, "x");
}

#[test]
fn function_value_from_non_function_json() {
    assert!(FunctionValue::from_json(&serde_json::json!(42)).is_none());
    assert!(FunctionValue::from_json(&serde_json::json!({"key": "value"})).is_none());
    assert!(FunctionValue::from_json(&serde_json::json!({"body": 42})).is_none());
}

#[test]
fn runtime_value_is_function() {
    let fn_val = FunctionValue {
        params: vec![("x".to_string(), serde_json::json!({"type": "number"}))],
        return_schema: Some(serde_json::json!({"type": "number"})),
        body: "mul(x, 2)".to_string(),
    };
    let rv = NonFunctionValue::new_function(fn_val);
    assert!(rv.is_function());
    assert!(rv.as_function().is_some());

    let plain = NonFunctionValue::new(42);
    assert!(!plain.is_function());
    assert!(plain.as_function().is_none());

    // An object without a `body` string is not a function
    let obj = NonFunctionValue::new(serde_json::json!({"key": "value"}));
    assert!(!obj.is_function());
    assert!(obj.as_function().is_none());
}

// ── FunctionValue map-format params ───────────────────────────────

#[test]
fn function_value_map_format_params() {
    let json = serde_json::json!({
        "params": {"x": {"type": "number"}, "y": {"type": "string"}},
        "body": "concat(x, y)",
    });
    let fn_val = FunctionValue::from_json(&json).unwrap();
    assert_eq!(fn_val.params.len(), 2);
    assert_eq!(fn_val.body, "concat(x, y)");
}

#[test]
fn function_value_array_format_params() {
    let json = serde_json::json!({
        "params": [
            {"name": "x", "schema": {"type": "number"}},
            {"name": "y", "schema": {"type": "string"}},
        ],
        "body": "concat(x, y)",
    });
    let fn_val = FunctionValue::from_json(&json).unwrap();
    assert_eq!(fn_val.params.len(), 2);
    assert_eq!(fn_val.params[0].0, "x");
    assert_eq!(fn_val.params[1].0, "y");
}

#[test]
fn function_value_no_params() {
    let json = serde_json::json!({"body": "42"});
    let fn_val = FunctionValue::from_json(&json).unwrap();
    assert!(fn_val.params.is_empty());
    assert_eq!(fn_val.body, "42");
    assert!(fn_val.return_schema.is_none());
}

#[test]
fn function_value_empty_params_array() {
    let json = serde_json::json!({"params": [], "body": "noop"});
    let fn_val = FunctionValue::from_json(&json).unwrap();
    assert!(fn_val.params.is_empty());
}

#[test]
fn function_value_with_null_schema() {
    let json = serde_json::json!({
        "params": [{"name": "x", "schema": null}],
        "body": "x",
    });
    let fn_val = FunctionValue::from_json(&json).unwrap();
    assert_eq!(fn_val.params.len(), 1);
    assert_eq!(fn_val.params[0].0, "x");
    assert_eq!(fn_val.params[0].1, serde_json::Value::Null);
}

#[test]
fn function_value_roundtrip_with_return_schema() {
    let fn_val = FunctionValue {
        params: vec![
            ("x".to_string(), serde_json::json!({"type": "number"})),
            ("y".to_string(), serde_json::json!({"type": "string"})),
        ],
        return_schema: Some(serde_json::json!({"type": "object"})),
        body: "transform(x, y)".to_string(),
    };
    let json = fn_val.to_json();
    let decoded = FunctionValue::from_json(&json).unwrap();
    assert_eq!(decoded, fn_val);
}

#[test]
fn function_value_roundtrip_no_params_no_return() {
    let fn_val = FunctionValue {
        params: vec![],
        return_schema: None,
        body: "answer".to_string(),
    };
    let json = fn_val.to_json();
    let decoded = FunctionValue::from_json(&json).unwrap();
    assert_eq!(decoded, fn_val);
}

// ── NonFunctionValue edge cases ───────────────────────────────────────

#[test]
fn runtime_value_new_from_bool() {
    let v = NonFunctionValue::new(false);
    assert!(v.is_bool());
    assert_eq!(v.as_bool(), Some(false));
}

#[test]
fn runtime_value_new_from_float() {
    let v = NonFunctionValue::new(3.14);
    assert!(v.is_number());
    assert_eq!(v.as_number(), Some(3.14));
}

#[test]
fn runtime_value_new_from_array() {
    let v = NonFunctionValue::new(vec!["a", "b", "c"]);
    assert!(v.is_array());
    assert_eq!(v.as_array().unwrap().len(), 3);
}

#[test]
fn runtime_value_new_from_nested_object() {
    let v = NonFunctionValue::new(serde_json::json!({
        "outer": {"inner": "value"},
        "list": [1, 2, 3],
    }));
    assert!(v.is_object());
    assert_eq!(
        v.get("outer").unwrap().get("inner").unwrap().as_str(),
        Some("value")
    );
    assert_eq!(v.get("list").unwrap().as_array().unwrap().len(), 3);
}

#[test]
fn runtime_value_as_object_mut() {
    let mut v = NonFunctionValue::new(serde_json::json!({"a": 1}));
    {
        let obj = v.as_object_mut().unwrap();
        obj.insert("b".to_string(), serde_json::json!(2));
    }
    assert_eq!(v.get("b").unwrap().as_i64(), Some(2));
}

#[test]
fn runtime_value_new_function_and_check() {
    let fn_val = FunctionValue {
        params: vec![("x".to_string(), serde_json::json!({"type": "number"}))],
        return_schema: None,
        body: "mul(x, 2)".to_string(),
    };
    let rv = NonFunctionValue::new_function(fn_val.clone());
    assert!(rv.is_function());
    let decoded = rv.as_function().unwrap();
    assert_eq!(decoded, fn_val);
}

#[test]
fn runtime_value_clone_eq() {
    let v1 = NonFunctionValue::new(serde_json::json!({"a": 1, "b": [2, 3]}));
    let v2 = v1.clone();
    assert_eq!(v1, v2);
}

#[test]
fn runtime_value_default_eq_null() {
    let v1 = NonFunctionValue::default();
    let v2 = NonFunctionValue::new(serde_json::Value::Null);
    assert_eq!(v1, v2);
}
