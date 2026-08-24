use ac_runtime_values::NonFunctionValue;
use serde_json::json;

use crate::{JsonSchemaBackend, compose_schema};
use ac_dialect::Dialect;
use ac_dialect_canonical::AC_DIALECT;
use ac_fields::extract_fields;
use ac_typegen_core::{TypegenBackend, TypegenContext};

fn build_ctx(extracted: &ac_fields::ExtractedFields) -> TypegenContext {
    let vocab = AC_DIALECT.document_vocabulary();
    let ac_keys: Vec<&str> = vocab.reserved_field_keys();
    let mut ctx = TypegenContext::from_schemas(&extracted.schemas);
    ctx.remove_fields(&extracted.merge);
    ctx.strip_keys(&ac_keys);
    ctx
}

#[test]
fn end_to_end_simple_fields() {
    let doc = json!({
        "$": {
            "host": { "type": "string" },
            "port": { "type": "number" }
        }
    });
    let extracted = extract_fields(NonFunctionValue::new(doc), &AC_DIALECT).unwrap();
    let ctx = build_ctx(&extracted);

    let backend = JsonSchemaBackend;
    let schema = backend.generate(&ctx);

    assert_eq!(schema["$schema"], "http://json-schema.org/draft-07/schema#");
    assert_eq!(schema["type"], "object");
    assert_eq!(schema["additionalProperties"], false);
    assert_eq!(schema["properties"]["host"]["type"], "string");
    assert_eq!(schema["properties"]["port"]["type"], "number");
}

#[test]
fn end_to_end_with_ac_keys_stripped() {
    let doc = json!({
        "$": {
            "host": { "type": "string", "expr": "compute()", "arg": true },
            "port": { "type": "number", "inheritable": true, "mutable": false }
        }
    });
    let extracted = extract_fields(NonFunctionValue::new(doc), &AC_DIALECT).unwrap();
    let ctx = build_ctx(&extracted);

    let schema = compose_schema(&ctx.fields);

    let host = &schema["properties"]["host"];
    assert!(!host.as_object().unwrap().contains_key("expr"));
    assert!(!host.as_object().unwrap().contains_key("arg"));
    assert_eq!(host["type"], "string");

    let port = &schema["properties"]["port"];
    assert!(!port.as_object().unwrap().contains_key("inheritable"));
    assert!(!port.as_object().unwrap().contains_key("mutable"));
    assert_eq!(port["type"], "number");
}

#[test]
fn end_to_end_merge_field_excluded() {
    let doc = json!({
        "$": {
            "host": { "type": "string" },
            "port": { "type": "number" },
            "_base": { "type": "object", "merge": true, "expr": "import('./base.yml')" }
        }
    });
    let extracted = extract_fields(NonFunctionValue::new(doc), &AC_DIALECT).unwrap();
    let ctx = build_ctx(&extracted);

    let schema = compose_schema(&ctx.fields);

    let props = schema["properties"].as_object().unwrap();
    assert!(props.contains_key("host"));
    assert!(props.contains_key("port"));
    assert!(!props.contains_key("_base"));
}

#[test]
fn end_to_end_backend_trait() {
    let doc = json!({
        "$": {
            "host": { "type": "string" }
        }
    });
    let extracted = extract_fields(NonFunctionValue::new(doc), &AC_DIALECT).unwrap();
    let ctx = build_ctx(&extracted);

    let backend = JsonSchemaBackend;
    assert_eq!(backend.extension(), "schema.json");
    assert_eq!(
        backend.ide_hint("main.schema.json"),
        "# yaml-language-server: $schema=main.schema.json"
    );

    let schema = backend.generate(&ctx);
    assert_eq!(schema["type"], "object");
}

#[test]
fn end_to_end_with_merged_fields() {
    let doc = json!({
        "$": {
            "host": { "type": "number" }
        }
    });
    let extracted = extract_fields(NonFunctionValue::new(doc), &AC_DIALECT).unwrap();
    let mut ctx = build_ctx(&extracted);

    let imported = vec![
        ("host".to_string(), json!({ "type": "string" })),
        ("port".to_string(), json!({ "type": "number" })),
    ];
    ctx.merge_fields(&imported);

    let schema = compose_schema(&ctx.fields);

    let host = &schema["properties"]["host"];
    assert_eq!(host["type"], "number");

    let port = &schema["properties"]["port"];
    assert_eq!(port["type"], "number");
}
