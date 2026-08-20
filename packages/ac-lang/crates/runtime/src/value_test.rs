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
