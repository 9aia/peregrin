use serde_json::json;

use super::*;
use ac_fields::extract_fields;
use ac_runtime::RuntimeValue;

#[test]
fn context_excludes_merge_fields() {
    let doc = json!({
        "fields": {
            "host": { "type": "string" },
            "port": { "type": "number" },
            "_base": { "type": "object", "merge": true, "expr": "import('./base.yml')" }
        },
        "host": "localhost",
        "port": 8080
    });
    let extracted = extract_fields(RuntimeValue::new(doc)).unwrap();
    let data = extracted.data.inner().clone();
    let ctx = TypegenContext::from_extracted(&extracted, &data, None);

    assert!(ctx.fields.iter().any(|(n, _)| n == "host"));
    assert!(ctx.fields.iter().any(|(n, _)| n == "port"));
    assert!(!ctx.fields.iter().any(|(n, _)| n == "_base"));
}

#[test]
fn context_strips_ac_keys() {
    let doc = json!({
        "fields": {
            "host": { "type": "string", "expr": "compute()", "arg": true },
            "port": { "type": "number", "inheritable": true, "mutable": false }
        }
    });
    let extracted = extract_fields(RuntimeValue::new(doc)).unwrap();
    let data = extracted.data.inner().clone();
    let ctx = TypegenContext::from_extracted(&extracted, &data, None);

    let (_, host_schema) = ctx.fields.iter().find(|(n, _)| n == "host").unwrap();
    assert!(!host_schema.as_object().unwrap().contains_key("expr"));
    assert!(!host_schema.as_object().unwrap().contains_key("arg"));

    let (_, port_schema) = ctx.fields.iter().find(|(n, _)| n == "port").unwrap();
    assert!(!port_schema.as_object().unwrap().contains_key("inheritable"));
    assert!(!port_schema.as_object().unwrap().contains_key("mutable"));
}

#[test]
fn context_preserves_schema_vocabulary() {
    let doc = json!({
        "fields": {
            "host": { "type": "string", "pattern": "^[a-z]+$" },
            "port": { "type": "number", "minimum": 1, "maximum": 65535 }
        }
    });
    let extracted = extract_fields(RuntimeValue::new(doc)).unwrap();
    let data = extracted.data.inner().clone();
    let ctx = TypegenContext::from_extracted(&extracted, &data, None);

    let (_, host_schema) = ctx.fields.iter().find(|(n, _)| n == "host").unwrap();
    assert_eq!(host_schema["type"], "string");
    assert_eq!(host_schema["pattern"], "^[a-z]+$");

    let (_, port_schema) = ctx.fields.iter().find(|(n, _)| n == "port").unwrap();
    assert_eq!(port_schema["type"], "number");
    assert_eq!(port_schema["minimum"], 1);
    assert_eq!(port_schema["maximum"], 65535);
}

#[test]
fn context_with_no_fields() {
    let doc = json!({});
    let extracted = extract_fields(RuntimeValue::new(doc)).unwrap();
    let data = extracted.data.inner().clone();
    let ctx = TypegenContext::from_extracted(&extracted, &data, None);
    assert!(ctx.fields.is_empty());
}

#[test]
fn context_with_no_merge_fields() {
    let doc = json!({
        "fields": {
            "host": { "type": "string" },
            "port": { "type": "number" }
        }
    });
    let extracted = extract_fields(RuntimeValue::new(doc)).unwrap();
    let data = extracted.data.inner().clone();
    let ctx = TypegenContext::from_extracted(&extracted, &data, None);

    assert_eq!(ctx.fields.len(), 2);
    assert!(ctx.resolved_imports.is_empty());
}
