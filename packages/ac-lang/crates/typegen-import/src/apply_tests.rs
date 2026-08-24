use ac_runtime_values::NonFunctionValue;
use serde_json::json;

use ac_dialect::Dialect;
use ac_dialect_canonical::AC_DIALECT;
use ac_fields::extract_fields;
use ac_module_resolver_virtual::VirtualResolver;
use ac_typegen_core::TypegenContext;

use crate::apply_imports;

fn build_ctx(schemas: &[(String, serde_json::Value)], merge_names: &[String]) -> TypegenContext {
    let vocab = AC_DIALECT.document_vocabulary();
    let ac_keys: Vec<&str> = vocab.reserved_field_keys();
    let mut ctx = TypegenContext::from_schemas(schemas);
    ctx.remove_fields(merge_names);
    ctx.strip_keys(&ac_keys);
    ctx
}

#[test]
fn apply_imports_static() {
    let mut resolver = VirtualResolver::new();
    resolver.add(
        "base.yml",
        json!({
            "$": {
                "host": { "type": "string" },
                "port": { "type": "number" }
            }
        }),
    );

    let doc = json!({
        "$": {
            "import": {
                "type": "object",
                "expr": "import('base.yml')",
                "merge": true
            },
            "name": { "type": "string" }
        }
    });
    let extracted = extract_fields(NonFunctionValue::new(doc), &AC_DIALECT).unwrap();
    let data = extracted.data.inner().clone();
    let mut ctx = build_ctx(&extracted.schemas, &extracted.merge);
    apply_imports(
        &mut ctx,
        &extracted.schemas,
        &extracted.merge,
        &data,
        &resolver,
        None,
        AC_DIALECT.document_vocabulary(),
    );

    assert!(!ctx.fields.iter().any(|(n, _)| n.as_str() == "import"));
    assert!(ctx.fields.iter().any(|(n, _)| n.as_str() == "name"));
    assert!(ctx.fields.iter().any(|(n, _)| n.as_str() == "host"));
    assert!(ctx.fields.iter().any(|(n, _)| n.as_str() == "port"));
}

#[test]
fn apply_imports_data_driven() {
    let mut resolver = VirtualResolver::new();
    resolver.add(
        "base.yml",
        json!({
            "$": {
                "host": { "type": "string" },
                "port": { "type": "number" }
            }
        }),
    );

    let doc = json!({
        "$": {
            "import": {
                "type": "object",
                "arg": { "type": "string" },
                "expr": "import(arg)",
                "merge": true
            },
            "name": { "type": "string" }
        },
        "import": "base.yml"
    });
    let extracted = extract_fields(NonFunctionValue::new(doc), &AC_DIALECT).unwrap();
    let data = extracted.data.inner().clone();
    let mut ctx = build_ctx(&extracted.schemas, &extracted.merge);
    apply_imports(
        &mut ctx,
        &extracted.schemas,
        &extracted.merge,
        &data,
        &resolver,
        None,
        AC_DIALECT.document_vocabulary(),
    );

    assert!(!ctx.fields.iter().any(|(n, _)| n.as_str() == "import"));
    assert!(ctx.fields.iter().any(|(n, _)| n.as_str() == "name"));
    assert!(ctx.fields.iter().any(|(n, _)| n.as_str() == "host"));
    assert!(ctx.fields.iter().any(|(n, _)| n.as_str() == "port"));
}

#[test]
fn apply_imports_parent_wins() {
    let mut resolver = VirtualResolver::new();
    resolver.add(
        "base.yml",
        json!({
            "$": {
                "host": { "type": "string" },
                "port": { "type": "number" }
            }
        }),
    );

    let doc = json!({
        "$": {
            "import": {
                "type": "object",
                "expr": "import('base.yml')",
                "merge": true
            },
            "host": { "type": "number" }
        }
    });
    let extracted = extract_fields(NonFunctionValue::new(doc), &AC_DIALECT).unwrap();
    let data = extracted.data.inner().clone();
    let mut ctx = build_ctx(&extracted.schemas, &extracted.merge);
    apply_imports(
        &mut ctx,
        &extracted.schemas,
        &extracted.merge,
        &data,
        &resolver,
        None,
        AC_DIALECT.document_vocabulary(),
    );

    let (_, host_schema) = ctx
        .fields
        .iter()
        .find(|(n, _)| n.as_str() == "host")
        .unwrap();
    assert_eq!(host_schema["type"], "number");
    assert!(ctx.fields.iter().any(|(n, _)| n.as_str() == "port"));
}

#[test]
fn apply_imports_no_imports_is_noop() {
    let resolver = VirtualResolver::new();

    let doc = json!({
        "$": {
            "host": { "type": "string" },
            "port": { "type": "number" }
        }
    });
    let extracted = extract_fields(NonFunctionValue::new(doc), &AC_DIALECT).unwrap();
    let data = extracted.data.inner().clone();
    let mut ctx = build_ctx(&extracted.schemas, &extracted.merge);
    apply_imports(
        &mut ctx,
        &extracted.schemas,
        &extracted.merge,
        &data,
        &resolver,
        None,
        AC_DIALECT.document_vocabulary(),
    );

    assert_eq!(ctx.fields.len(), 2);
}