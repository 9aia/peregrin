use super::*;

#[test]
fn merge_fields_basic() {
    let mut data = serde_json::json!({
        "_base": { "host": "example.com", "port": 8080 },
        "name": "handl"
    });
    let merge = vec!["_base".to_string()];
    let errors = merge_fields(&mut data, &merge);
    assert!(errors.is_empty());
    assert_eq!(data["host"], "example.com");
    assert_eq!(data["port"], 8080);
    assert_eq!(data["name"], "handl");
    assert!(data.as_object().unwrap().get("_base").is_none());
}

#[test]
fn merge_fields_parent_wins() {
    let mut data = serde_json::json!({
        "_base": { "host": "from-merge", "port": 8080 },
        "host": "from-parent"
    });
    let merge = vec!["_base".to_string()];
    let errors = merge_fields(&mut data, &merge);
    assert!(errors.is_empty());
    assert_eq!(data["host"], "from-parent");
    assert_eq!(data["port"], 8080);
    assert!(data.as_object().unwrap().get("_base").is_none());
}

#[test]
fn merge_fields_multiple_in_order() {
    let mut data = serde_json::json!({
        "_first": { "a": 1, "b": 2 },
        "_second": { "b": 99, "c": 3 }
    });
    let merge = vec!["_first".to_string(), "_second".to_string()];
    let errors = merge_fields(&mut data, &merge);
    assert!(errors.is_empty());
    assert_eq!(data["a"], 1);
    assert_eq!(data["b"], 2);
    assert_eq!(data["c"], 3);
    assert!(data.as_object().unwrap().get("_first").is_none());
    assert!(data.as_object().unwrap().get("_second").is_none());
}

#[test]
fn merge_fields_non_object_error() {
    let mut data = serde_json::json!({
        "_base": "not-an-object",
        "name": "handl"
    });
    let merge = vec!["_base".to_string()];
    let errors = merge_fields(&mut data, &merge);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].contains("_base"));
    assert!(errors[0].contains("not an object"));
    assert!(data.as_object().unwrap().contains_key("_base"));
}

#[test]
fn merge_fields_empty_is_noop() {
    let mut data = serde_json::json!({"name": "handl"});
    let errors = merge_fields(&mut data, &[]);
    assert!(errors.is_empty());
    assert_eq!(data["name"], "handl");
}

#[test]
fn merge_fields_missing_field_skipped() {
    let mut data = serde_json::json!({"name": "handl"});
    let merge = vec!["_base".to_string()];
    let errors = merge_fields(&mut data, &merge);
    assert!(errors.is_empty());
    assert_eq!(data["name"], "handl");
}
