use super::*;

#[test]
fn new_from_string() {
    let v = RuntimeValue::new("hello");
    assert_eq!(v.as_str(), Some("hello"));
}

#[test]
fn new_from_json() {
    let v = RuntimeValue::new(serde_json::json!({"name": "ac"}));
    assert_eq!(v.get("name").unwrap().as_str(), Some("ac"));
}

#[test]
fn predicates() {
    let null = RuntimeValue::new(serde_json::Value::Null);
    assert!(null.is_null());

    let boolean = RuntimeValue::new(true);
    assert!(boolean.is_bool());
    assert_eq!(boolean.as_bool(), Some(true));

    let number = RuntimeValue::new(42);
    assert!(number.is_number());
    assert_eq!(number.as_number(), Some(42.0));

    let string = RuntimeValue::new("hello");
    assert!(string.is_string());

    let array = RuntimeValue::new(serde_json::json!([1, 2, 3]));
    assert!(array.is_array());
    assert_eq!(array.as_array().unwrap().len(), 3);

    let object = RuntimeValue::new(serde_json::json!({"key": "value"}));
    assert!(object.is_object());
}

#[test]
fn inner_and_into_inner() {
    let v = RuntimeValue::new(serde_json::json!({"a": 1}));
    assert_eq!(v.inner().get("a").unwrap().as_i64(), Some(1));

    let inner = v.into_inner();
    assert_eq!(inner.get("a").unwrap().as_i64(), Some(1));
}

#[test]
fn inner_mut() {
    let mut v = RuntimeValue::new(serde_json::json!({"a": 1}));
    if let Some(obj) = v.inner_mut().as_object_mut() {
        obj.insert("b".to_string(), serde_json::json!(2));
    }
    assert_eq!(v.get("b").unwrap().as_i64(), Some(2));
}

#[test]
fn default_is_null() {
    let v = RuntimeValue::default();
    assert!(v.is_null());
}

#[test]
fn host_value_is_alias() {
    let v: HostValue = RuntimeValue::new("test");
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
    let rv = RuntimeValue::new_function(fn_val);
    assert!(rv.is_function());
    assert!(rv.as_function().is_some());

    let plain = RuntimeValue::new(42);
    assert!(!plain.is_function());
    assert!(plain.as_function().is_none());

    // An object without a `body` string is not a function
    let obj = RuntimeValue::new(serde_json::json!({"key": "value"}));
    assert!(!obj.is_function());
    assert!(obj.as_function().is_none());
}
