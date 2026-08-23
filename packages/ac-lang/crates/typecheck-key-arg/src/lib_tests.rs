use super::*;
use ac_typecheck_core::Type;
use serde_json::json;

struct JsonInterpreter;

impl SchemaInterpreter for JsonInterpreter {
    fn interpret(&self, schema: &JsonValue) -> Type {
        match schema.get("type").and_then(|v| v.as_str()) {
            Some("string") => Type::String,
            Some("number") | Some("integer") => Type::Number,
            Some("boolean") => Type::Boolean,
            _ => Type::Unknown}
    }
}

#[test]
fn no_arg_is_ok() {
    let interp = JsonInterpreter;
    let schema = json!({ "type": "number" });
    assert!(check_field("x", &schema, &interp).is_ok());
}

#[test]
fn arg_type_matches() {
    let interp = JsonInterpreter;
    let schema = json!({ "type": "number", "arg": { "type": "number" } });
    assert!(check_field("x", &schema, &interp).is_ok());
}

#[test]
fn arg_type_mismatch() {
    let interp = JsonInterpreter;
    let schema = json!({ "type": "number", "arg": { "type": "string" } });
    let result = check_field("x", &schema, &interp);
    assert!(result.is_err());
}

#[test]
fn unknown_arg_type_passes() {
    let interp = JsonInterpreter;
    let schema = json!({ "type": "number", "arg": {} });
    assert!(check_field("x", &schema, &interp).is_ok());
}

#[test]
fn check_multiple_fields() {
    let interp = JsonInterpreter;
    let schemas = vec![
        (
            "ok".to_string(),
            json!({ "type": "number", "arg": { "type": "number" } }),
        ),
        (
            "bad".to_string(),
            json!({ "type": "string", "arg": { "type": "number" } }),
        ),
    ];
    let errors = check(&schemas, &interp);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].field, "bad");
}
