use serde_json::json;

use ac_fields::extract_fields;
use ac_runtime::RuntimeValue;
use ac_typegen::TypegenContext;
use ac_typegen::TypegenBackend;
use crate::{JsonSchemaBackend, compose_schema};

#[test]
fn end_to_end_simple_fields() {
    let doc = json!({
        "fields": {
            "host": { "type": "string" },
            "port": { "type": "number" }
        }
    });
    let extracted = extract_fields(RuntimeValue::new(doc)).unwrap();
    let data = extracted.data.inner().clone();
    let ctx = TypegenContext::from_extracted(&extracted, &data, None);

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
        "fields": {
            "host": { "type": "string", "expr": "compute()", "arg": true },
            "port": { "type": "number", "inheritable": true, "mutable": false }
        }
    });
    let extracted = extract_fields(RuntimeValue::new(doc)).unwrap();
    let data = extracted.data.inner().clone();
    let ctx = TypegenContext::from_extracted(&extracted, &data, None);

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
        "fields": {
            "host": { "type": "string" },
            "port": { "type": "number" },
            "_base": { "type": "object", "merge": true, "expr": "import('./base.yml')" }
        }
    });
    let extracted = extract_fields(RuntimeValue::new(doc)).unwrap();
    let data = extracted.data.inner().clone();
    let ctx = TypegenContext::from_extracted(&extracted, &data, None);

    let schema = compose_schema(&ctx.fields);

    let props = schema["properties"].as_object().unwrap();
    assert!(props.contains_key("host"));
    assert!(props.contains_key("port"));
    assert!(!props.contains_key("_base"));
}

#[test]
fn end_to_end_with_static_import() {
    let base_content = "fields:\n  host:\n    type: string\n  port:\n    type: number\n";
    let mut base_path = std::env::temp_dir();
    base_path.push(format!("ac_typegen_e2e_base_{}.yml", std::process::id()));
    std::fs::write(&base_path, base_content).unwrap();

    let base_dir = base_path.parent().unwrap();
    let base_name = base_path.file_name().unwrap().to_str().unwrap();

    let doc = json!({
        "fields": {
            "import": {
                "type": "object",
                "expr": format!("import('{base_name}')"),
                "merge": true
            },
            "name": { "type": "string" }
        }
    });
    let extracted = extract_fields(RuntimeValue::new(doc)).unwrap();
    let data = extracted.data.inner().clone();
    let ctx = TypegenContext::from_extracted(&extracted, &data, Some(base_dir));

    let schema = compose_schema(&ctx.fields);

    let props = schema["properties"].as_object().unwrap();
    assert!(!props.contains_key("import"));
    assert!(props.contains_key("name"));
    assert!(props.contains_key("host"));
    assert!(props.contains_key("port"));

    let _ = std::fs::remove_file(&base_path);
}

#[test]
fn end_to_end_data_driven_import() {
    let base_content = "fields:\n  host:\n    type: string\n  port:\n    type: number\n";
    let mut base_path = std::env::temp_dir();
    base_path.push(format!("ac_typegen_e2e_dd_{}.yml", std::process::id()));
    std::fs::write(&base_path, base_content).unwrap();

    let base_dir = base_path.parent().unwrap();
    let base_name = base_path.file_name().unwrap().to_str().unwrap();

    let doc = json!({
        "fields": {
            "import": {
                "type": "object",
                "arg": { "type": "string" },
                "expr": "import(arg)",
                "merge": true
            },
            "name": { "type": "string" }
        },
        "import": base_name
    });
    let extracted = extract_fields(RuntimeValue::new(doc)).unwrap();
    let data = extracted.data.inner().clone();
    let ctx = TypegenContext::from_extracted(&extracted, &data, Some(base_dir));

    let schema = compose_schema(&ctx.fields);

    let props = schema["properties"].as_object().unwrap();
    assert!(!props.contains_key("import"));
    assert!(props.contains_key("name"));
    assert!(props.contains_key("host"));
    assert!(props.contains_key("port"));

    let _ = std::fs::remove_file(&base_path);
}

#[test]
fn end_to_end_parent_wins_on_conflict() {
    let base_content = "fields:\n  host:\n    type: string\n  port:\n    type: number\n";
    let mut base_path = std::env::temp_dir();
    base_path.push(format!("ac_typegen_e2e_conflict_{}.yml", std::process::id()));
    std::fs::write(&base_path, base_content).unwrap();

    let base_dir = base_path.parent().unwrap();
    let base_name = base_path.file_name().unwrap().to_str().unwrap();

    let doc = json!({
        "fields": {
            "import": {
                "type": "object",
                "expr": format!("import('{base_name}')"),
                "merge": true
            },
            "host": { "type": "number" }
        }
    });
    let extracted = extract_fields(RuntimeValue::new(doc)).unwrap();
    let data = extracted.data.inner().clone();
    let ctx = TypegenContext::from_extracted(&extracted, &data, Some(base_dir));

    let schema = compose_schema(&ctx.fields);

    let host = &schema["properties"]["host"];
    assert_eq!(host["type"], "number");

    let port = &schema["properties"]["port"];
    assert_eq!(port["type"], "number");

    let _ = std::fs::remove_file(&base_path);
}

#[test]
fn end_to_end_backend_trait() {
    let doc = json!({
        "fields": {
            "host": { "type": "string" }
        }
    });
    let extracted = extract_fields(RuntimeValue::new(doc)).unwrap();
    let data = extracted.data.inner().clone();
    let ctx = TypegenContext::from_extracted(&extracted, &data, None);

    let backend = JsonSchemaBackend;
    assert_eq!(backend.extension(), "schema.json");
    assert_eq!(
        backend.ide_hint("main.schema.json"),
        "# yaml-language-server: $schema=main.schema.json"
    );

    let schema = backend.generate(&ctx);
    assert_eq!(schema["type"], "object");
}
