use serde_json::json;

use super::*;

#[test]
fn strip_keys_removes_specified_keys() {
    let input = json!({
        "type": "string",
        "expr": "compute()",
        "arg": true,
        "pattern": "^[a-z]+$"
    });
    let output = strip_keys(&input, &["expr", "arg"]);
    assert_eq!(
        output,
        json!({
            "type": "string",
            "pattern": "^[a-z]+$"
        })
    );
}

#[test]
fn strip_keys_is_recursive() {
    let input = json!({
        "type": "object",
        "properties": {
            "host": { "type": "string", "expr": "something()" },
            "port": { "type": "number" }
        },
        "required": ["host"]
    });
    let output = strip_keys(&input, &["expr"]);
    assert_eq!(
        output,
        json!({
            "type": "object",
            "properties": {
                "host": { "type": "string" },
                "port": { "type": "number" }
            },
            "required": ["host"]
        })
    );
}

#[test]
fn strip_keys_handles_arrays() {
    let input = json!({
        "type": "array",
        "items": {
            "type": "object",
            "properties": {
                "name": { "type": "string", "expr": "compute()" }
            }
        }
    });
    let output = strip_keys(&input, &["expr"]);
    assert_eq!(
        output,
        json!({
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "name": { "type": "string" }
                }
            }
        })
    );
}

#[test]
fn strip_keys_non_object_unchanged() {
    let input = json!("string");
    let output = strip_keys(&input, &["expr"]);
    assert_eq!(output, input);
}

#[test]
fn from_schemas_clones_raw() {
    let schemas = vec![("host".to_string(), json!({ "type": "string", "expr": "x" }))];
    let ctx = TypegenContext::from_schemas(&schemas);
    assert_eq!(ctx.fields.len(), 1);
    assert_eq!(ctx.fields["host"]["type"], "string");
    assert_eq!(ctx.fields["host"]["expr"], "x");
}

#[test]
fn remove_fields_excludes_named() {
    let schemas = vec![
        ("host".to_string(), json!({ "type": "string" })),
        ("port".to_string(), json!({ "type": "number" })),
        ("_base".to_string(), json!({ "type": "object" })),
    ];
    let mut ctx = TypegenContext::from_schemas(&schemas);
    ctx.remove_fields(&["_base".to_string()]);
    assert_eq!(ctx.fields.len(), 2);
    assert!(!ctx.fields.contains_key("_base"));
}

#[test]
fn ctx_strip_keys_strips_all_fields() {
    let schemas = vec![
        (
            "host".to_string(),
            json!({ "type": "string", "expr": "x", "arg": true }),
        ),
        (
            "port".to_string(),
            json!({ "type": "number", "inheritable": true }),
        ),
    ];
    let mut ctx = TypegenContext::from_schemas(&schemas);
    ctx.strip_keys(&["expr", "arg", "inheritable"]);

    let host = &ctx.fields["host"];
    assert!(!host.as_object().unwrap().contains_key("expr"));
    assert!(!host.as_object().unwrap().contains_key("arg"));

    let port = &ctx.fields["port"];
    assert!(!port.as_object().unwrap().contains_key("inheritable"));
}

#[test]
fn merge_fields_parent_wins() {
    let schemas = vec![("host".to_string(), json!({ "type": "number" }))];
    let mut ctx = TypegenContext::from_schemas(&schemas);

    let imported = vec![
        ("host".to_string(), json!({ "type": "string" })),
        ("port".to_string(), json!({ "type": "number" })),
    ];
    ctx.merge_fields(&imported);

    let host_schema = &ctx.fields["host"];
    assert_eq!(host_schema["type"], "number");
    assert!(ctx.fields.contains_key("port"));
}
