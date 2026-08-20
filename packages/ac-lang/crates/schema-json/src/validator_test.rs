use super::*;
use ac_schema::Schema;
use ac_runtime::RuntimeValue;

#[test]
fn validate_string() {
    let schema = JsonSchema::new(serde_json::json!({"type": "string"}));
    let value = RuntimeValue::new("hello");
    assert!(schema.validate(&value).is_ok());

    let value = RuntimeValue::new(42);
    assert!(schema.validate(&value).is_err());
}

#[test]
fn validate_required_fields() {
    let schema = JsonSchema::new(serde_json::json!({
        "type": "object",
        "required": ["name", "age"]
    }));
    let value = RuntimeValue::new(serde_json::json!({"name": "Luis", "age": 30}));
    assert!(schema.validate(&value).is_ok());

    let value = RuntimeValue::new(serde_json::json!({"name": "Luis"}));
    assert!(schema.validate(&value).is_err());
}

#[test]
fn validate_integer() {
    let schema = JsonSchema::new(serde_json::json!({"type": "integer"}));
    let value = RuntimeValue::new(42);
    assert!(schema.validate(&value).is_ok());

    let value = RuntimeValue::new(3.14);
    assert!(schema.validate(&value).is_err());
}
