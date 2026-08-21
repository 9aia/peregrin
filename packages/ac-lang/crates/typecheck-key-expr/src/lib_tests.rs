use super::*;
use ac_typecheck_core::{FunctionType, SchemaInterpreter, Type, TypeEnv};
use ac_typecheck_infer::FunctionTable;
use serde_json::{Value as JsonValue, json};

struct JsonInterpreter;

impl SchemaInterpreter for JsonInterpreter {
    fn interpret(&self, schema: &JsonValue) -> Type {
        match schema.get("type").and_then(|v| v.as_str()) {
            Some("string") => Type::String,
            Some("number") | Some("integer") => Type::Number,
            Some("boolean") => Type::Boolean,
            Some("array") => Type::Array(Box::new(
                schema
                    .get("items")
                    .map(|i| self.interpret(i))
                    .unwrap_or(Type::Unknown),
            )),
            _ => Type::Unknown,
        }
    }
}

fn make_env() -> (TypeEnv, FunctionTable) {
    let mut env = TypeEnv::new();
    env.insert("version", Type::Number);

    let mut funcs = FunctionTable::new();
    funcs.insert(
        "mul",
        FunctionType {
            params: vec![
                ("x".to_string(), Type::Number),
                ("y".to_string(), Type::Number),
            ],
            return_type: Box::new(Type::Number),
        },
    );

    (env, funcs)
}

#[test]
fn no_expr_is_ok() {
    let (env, funcs) = make_env();
    let interp = JsonInterpreter;
    let schema = json!({ "type": "number" });
    assert!(check_field("version", &schema, &env, &funcs, &interp).is_ok());
}

#[test]
fn expr_type_matches() {
    let (env, funcs) = make_env();
    let interp = JsonInterpreter;
    let schema = json!({ "type": "number", "expr": "mul(version, 2)" });
    assert!(check_field("doubled", &schema, &env, &funcs, &interp).is_ok());
}

#[test]
fn expr_type_mismatch() {
    let (env, funcs) = make_env();
    let interp = JsonInterpreter;
    let schema = json!({ "type": "string", "expr": "mul(version, 2)" });
    let result = check_field("doubled", &schema, &env, &funcs, &interp);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.field, "doubled");
}

#[test]
fn unknown_inferred_type_passes() {
    let (env, funcs) = make_env();
    let interp = JsonInterpreter;
    let schema = json!({ "type": "string", "expr": "unknown_func(1)" });
    assert!(check_field("x", &schema, &env, &funcs, &interp).is_ok());
}

#[test]
fn check_multiple_fields() {
    let (env, funcs) = make_env();
    let interp = JsonInterpreter;
    let schemas = vec![
        (
            "ok".to_string(),
            json!({ "type": "number", "expr": "mul(version, 2)" }),
        ),
        (
            "bad".to_string(),
            json!({ "type": "string", "expr": "mul(version, 2)" }),
        ),
    ];
    let errors = check(&schemas, &env, &funcs, &interp);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].field, "bad");
}
