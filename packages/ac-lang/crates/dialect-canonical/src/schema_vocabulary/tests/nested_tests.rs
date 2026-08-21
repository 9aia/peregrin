use super::*;
use ac_dialect::Schema;
use ac_runtime::RuntimeValue;

// ── nested / complex schemas ───────────────────────────────────

#[test]
fn validate_nested_object() {
    let schema = JsonSchema::new(serde_json::json!({
        "type": "object",
        "properties": {
            "user": {
                "type": "object",
                "properties": {
                    "name": {"type": "string"},
                    "tags": {
                        "type": "array",
                        "items": {"type": "string"},
                        "uniqueItems": true
                    }
                },
                "required": ["name"]
            }
        },
        "required": ["user"]
    }));
    assert!(schema.validate(&RuntimeValue::new(serde_json::json!({
        "user": {"name": "Luis", "tags": ["dev", "rust"]}
    }))).is_ok());
    assert!(schema.validate(&RuntimeValue::new(serde_json::json!({
        "user": {"tags": ["dev"]}
    }))).is_err());
    assert!(schema.validate(&RuntimeValue::new(serde_json::json!({}))).is_err());
}

#[test]
fn validate_error_has_path() {
    let schema = JsonSchema::new(serde_json::json!({
        "type": "object",
        "properties": {
            "age": {"type": "integer", "minimum": 0}
        }
    }));
    let result = schema.validate(&RuntimeValue::new(serde_json::json!({"age": -1})));
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.path.contains("age"), "error path should contain 'age': got '{}'", err.path);
}

#[test]
fn validate_error_has_location_field() {
    let schema = JsonSchema::new(serde_json::json!({
        "type": "object",
        "properties": {
            "age": {"type": "integer", "minimum": 0}
        }
    }));
    let result = schema.validate(&RuntimeValue::new(serde_json::json!({"age": -1})));
    let err = result.unwrap_err();
    assert!(err.location.is_none(), "location should be None from validator; compiler resolves it post-hoc");
}

#[test]
fn validate_error_display_with_location() {
    use ac_source::SourceLocation;
    let schema = JsonSchema::new(serde_json::json!({
        "type": "object",
        "properties": {
            "age": {"type": "integer", "minimum": 0}
        }
    }));
    let result = schema.validate(&RuntimeValue::new(serde_json::json!({"age": -1})));
    let mut err = result.unwrap_err();
    err.location = Some(SourceLocation::new(3, 5));
    let displayed = err.to_string();
    assert!(displayed.contains("[3:5]"), "display should contain [3:5]: got '{displayed}'");
    assert!(displayed.contains("age"), "display should contain path: got '{displayed}'");
}
