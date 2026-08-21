use super::*;
use ac_typecheck_core::Type;
use serde_json::json;

#[test]
fn interprets_string() {
    assert_eq!(interpret_schema(&json!({ "type": "string" })), Type::String);
}

#[test]
fn interprets_number() {
    assert_eq!(interpret_schema(&json!({ "type": "number" })), Type::Number);
    assert_eq!(
        interpret_schema(&json!({ "type": "integer" })),
        Type::Number
    );
}

#[test]
fn interprets_boolean() {
    assert_eq!(
        interpret_schema(&json!({ "type": "boolean" })),
        Type::Boolean
    );
}

#[test]
fn interprets_null() {
    assert_eq!(interpret_schema(&json!({ "type": "null" })), Type::Null);
}

#[test]
fn interprets_array_with_items() {
    let schema = json!({ "type": "array", "items": { "type": "string" } });
    assert_eq!(
        interpret_schema(&schema),
        Type::Array(Box::new(Type::String))
    );
}

#[test]
fn interprets_array_without_items() {
    let schema = json!({ "type": "array" });
    assert_eq!(
        interpret_schema(&schema),
        Type::Array(Box::new(Type::Unknown))
    );
}

#[test]
fn interprets_object_with_properties() {
    let schema = json!({
        "type": "object",
        "properties": {
            "name": { "type": "string" },
            "version": { "type": "number" }
        }
    });
    let ty = interpret_schema(&schema);
    if let Type::Object(props) = ty {
        assert_eq!(props.get("name"), Some(&Type::String));
        assert_eq!(props.get("version"), Some(&Type::Number));
    } else {
        panic!("expected object type");
    }
}

#[test]
fn interprets_object_without_properties() {
    let schema = json!({ "type": "object" });
    let ty = interpret_schema(&schema);
    assert!(matches!(ty, Type::Object(_)));
}

#[test]
fn unknown_type_returns_unknown() {
    assert_eq!(
        interpret_schema(&json!({ "type": "custom" })),
        Type::Unknown
    );
}

#[test]
fn no_type_field_returns_unknown() {
    assert_eq!(
        interpret_schema(&json!({ "description": "no type" })),
        Type::Unknown
    );
}

#[test]
fn enum_infers_type_from_first_value() {
    assert_eq!(
        interpret_schema(&json!({ "enum": ["alice", "bob"] })),
        Type::String
    );
    assert_eq!(
        interpret_schema(&json!({ "enum": [1, 2, 3] })),
        Type::Number
    );
}

#[test]
fn interprets_nested_object() {
    let schema = json!({
        "type": "object",
        "properties": {
            "config": {
                "type": "object",
                "properties": {
                    "host": { "type": "string" }
                }
            }
        }
    });
    let ty = interpret_schema(&schema);
    if let Type::Object(props) = ty {
        if let Some(Type::Object(inner)) = props.get("config") {
            assert_eq!(inner.get("host"), Some(&Type::String));
        } else {
            panic!("expected nested object");
        }
    } else {
        panic!("expected object type");
    }
}

#[test]
fn interpret_function_builds_function_type() {
    let params = vec![
        ("x".to_string(), json!({ "type": "number" })),
        ("y".to_string(), json!({ "type": "number" })),
    ];
    let return_schema = json!({ "type": "number" });
    let fn_type = interpret_function(&params, Some(&return_schema));
    assert_eq!(fn_type.params.len(), 2);
    assert_eq!(fn_type.params[0].1, Type::Number);
    assert_eq!(*fn_type.return_type, Type::Number);
}

#[test]
fn interpret_function_without_return() {
    let params = vec![("x".to_string(), json!({ "type": "string" }))];
    let fn_type = interpret_function(&params, None);
    assert_eq!(*fn_type.return_type, Type::Unknown);
}

#[test]
fn json_schema_interpreter_trait() {
    let interp = JsonSchemaInterpreter;
    assert_eq!(interp.interpret(&json!({ "type": "string" })), Type::String);
    assert_eq!(interp.interpret(&json!({ "type": "number" })), Type::Number);
}

#[test]
fn interprets_function_type() {
    let schema = json!({
        "type": "function",
        "params": [
            {"name": "x", "schema": {"type": "number"}},
            {"name": "y", "schema": {"type": "number"}}
        ],
        "return": {"type": "number"}
    });
    let ty = interpret_schema(&schema);
    match ty {
        Type::Function(fn_type) => {
            assert_eq!(fn_type.params.len(), 2);
            assert_eq!(fn_type.params[0].0, "x");
            assert_eq!(fn_type.params[0].1, Type::Number);
            assert_eq!(fn_type.params[1].0, "y");
            assert_eq!(*fn_type.return_type, Type::Number);
        }
        _ => panic!("expected function type"),
    }
}

#[test]
fn interprets_function_without_return() {
    let schema = json!({
        "type": "function",
        "params": [{"name": "x", "schema": {"type": "string"}}]
    });
    let ty = interpret_schema(&schema);
    match ty {
        Type::Function(fn_type) => {
            assert_eq!(fn_type.params.len(), 1);
            assert_eq!(*fn_type.return_type, Type::Unknown);
        }
        _ => panic!("expected function type"),
    }
}

#[test]
fn infer_json_type_detects_function_value() {
    let fn_json = json!({
        "params": [{"name": "x", "schema": {"type": "number"}}],
        "return": {"type": "number"},
        "body": "mul(x, 2)"
    });
    let ty = infer_json_type(&fn_json);
    match ty {
        Type::Function(fn_type) => {
            assert_eq!(fn_type.params.len(), 1);
            assert_eq!(fn_type.params[0].0, "x");
            assert_eq!(*fn_type.return_type, Type::Number);
        }
        _ => panic!("expected function type"),
    }
}

#[test]
fn infer_json_type_regular_object_stays_unknown() {
    let ty = infer_json_type(&json!({"key": "value"}));
    assert_eq!(ty, Type::Unknown);
}
