use serde_json::json;

use ac_dialect::AC_DIALECT;
use ac_fields::extract_fields;
use ac_runtime::RuntimeValue;
use ac_typegen_core::TypegenContext;
use ac_typegen_key_merge as merge;
use ac_typegen_key_inherit as inherit;
use ac_typegen_key_mutable as mutable;
use ac_typegen_key_expr as expr;
use ac_typegen_key_arg as arg;
use ac_resolver_virtual::VirtualResolver;

use crate::apply_imports;

const AC_KEYS: &[&str] = &[
    merge::KEY,
    inherit::KEY,
    mutable::KEY,
    expr::KEY,
    arg::KEY,
];

fn build_ctx(schemas: &[(String, serde_json::Value)], merge_names: &[String]) -> TypegenContext {
    let mut ctx = TypegenContext::from_schemas(schemas);
    merge::filter_fields(&mut ctx, merge_names);
    ctx.strip_keys(AC_KEYS);
    ctx
}

#[test]
fn apply_imports_static() {
    let mut resolver = VirtualResolver::new();
    resolver.add(
        "base.yml",
        json!({
            "fields": {
                "host": { "type": "string" },
                "port": { "type": "number" }
            }
        }),
    );

    let doc = json!({
        "fields": {
            "import": {
                "type": "object",
                "expr": "import('base.yml')",
                "merge": true
            },
            "name": { "type": "string" }
        }
    });
    let extracted = extract_fields(RuntimeValue::new(doc), &AC_DIALECT).unwrap();
    let data = extracted.data.inner().clone();
    let mut ctx = build_ctx(&extracted.schemas, &extracted.merge);
    apply_imports(&mut ctx, &extracted.schemas, &extracted.merge, &data, &resolver, None, AC_KEYS);

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
            "fields": {
                "host": { "type": "string" },
                "port": { "type": "number" }
            }
        }),
    );

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
        "import": "base.yml"
    });
    let extracted = extract_fields(RuntimeValue::new(doc), &AC_DIALECT).unwrap();
    let data = extracted.data.inner().clone();
    let mut ctx = build_ctx(&extracted.schemas, &extracted.merge);
    apply_imports(&mut ctx, &extracted.schemas, &extracted.merge, &data, &resolver, None, AC_KEYS);

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
            "fields": {
                "host": { "type": "string" },
                "port": { "type": "number" }
            }
        }),
    );

    let doc = json!({
        "fields": {
            "import": {
                "type": "object",
                "expr": "import('base.yml')",
                "merge": true
            },
            "host": { "type": "number" }
        }
    });
    let extracted = extract_fields(RuntimeValue::new(doc), &AC_DIALECT).unwrap();
    let data = extracted.data.inner().clone();
    let mut ctx = build_ctx(&extracted.schemas, &extracted.merge);
    apply_imports(&mut ctx, &extracted.schemas, &extracted.merge, &data, &resolver, None, AC_KEYS);

    let (_, host_schema) = ctx.fields.iter().find(|(n, _)| n.as_str() == "host").unwrap();
    assert_eq!(host_schema["type"], "number");
    assert!(ctx.fields.iter().any(|(n, _)| n.as_str() == "port"));
}

#[test]
fn apply_imports_no_imports_is_noop() {
    let resolver = VirtualResolver::new();

    let doc = json!({
        "fields": {
            "host": { "type": "string" },
            "port": { "type": "number" }
        }
    });
    let extracted = extract_fields(RuntimeValue::new(doc), &AC_DIALECT).unwrap();
    let data = extracted.data.inner().clone();
    let mut ctx = build_ctx(&extracted.schemas, &extracted.merge);
    apply_imports(&mut ctx, &extracted.schemas, &extracted.merge, &data, &resolver, None, AC_KEYS);

    assert_eq!(ctx.fields.len(), 2);
}
