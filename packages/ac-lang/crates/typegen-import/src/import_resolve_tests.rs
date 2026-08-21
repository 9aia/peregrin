use serde_json::json;

use ac_dialect::Dialect;
use ac_dialect_canonical::AC_DIALECT;
use ac_module_resolver_virtual::VirtualResolver;

use crate::resolve_imports;

fn make_resolver() -> VirtualResolver {
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
    resolver
}

#[test]
fn resolves_static_import() {
    let resolver = make_resolver();

    let schemas = vec![
        (
            "import".to_string(),
            json!({
                "type": "object",
                "expr": "import('base.yml')",
                "merge": true
            }),
        ),
        ("name".to_string(), json!({ "type": "string" })),
    ];
    let merge = vec!["import".to_string()];
    let data = json!({});

    let results = resolve_imports(&schemas, &merge, &data, &resolver, None, AC_DIALECT.document_vocabulary());

    assert_eq!(results.len(), 1);
    let (path, fields) = &results[0];
    assert_eq!(path, "base.yml");
    assert_eq!(fields.len(), 2);
    assert!(fields.iter().any(|(n, _)| n == "host"));
    assert!(fields.iter().any(|(n, _)| n == "port"));
}

#[test]
fn resolves_data_driven_import() {
    let resolver = make_resolver();

    let schemas = vec![
        (
            "import".to_string(),
            json!({
                "type": "object",
                "arg": { "type": "string" },
                "expr": "import(arg)",
                "merge": true
            }),
        ),
        ("name".to_string(), json!({ "type": "string" })),
    ];
    let merge = vec!["import".to_string()];
    let data = json!({ "import": "base.yml" });

    let results = resolve_imports(&schemas, &merge, &data, &resolver, None, AC_DIALECT.document_vocabulary());

    assert_eq!(results.len(), 1);
    let (_, fields) = &results[0];
    assert_eq!(fields.len(), 2);
    assert!(fields.iter().any(|(n, _)| n == "host"));
}

#[test]
fn skips_non_import_merge_field() {
    let resolver = make_resolver();

    let schemas = vec![
        (
            "config".to_string(),
            json!({
                "type": "object",
                "arg": { "type": "object" },
                "expr": "arg",
                "merge": true
            }),
        ),
    ];
    let merge = vec!["config".to_string()];
    let data = json!({});

    let results = resolve_imports(&schemas, &merge, &data, &resolver, None, AC_DIALECT.document_vocabulary());
    assert!(results.is_empty());
}

#[test]
fn handles_missing_module_gracefully() {
    let resolver = VirtualResolver::new();

    let schemas = vec![
        (
            "import".to_string(),
            json!({
                "type": "object",
                "expr": "import('nonexistent.yml')",
                "merge": true
            }),
        ),
    ];
    let merge = vec!["import".to_string()];
    let data = json!({});

    let results = resolve_imports(&schemas, &merge, &data, &resolver, None, AC_DIALECT.document_vocabulary());
    assert!(results.is_empty());
}

#[test]
fn strips_ac_keys_from_imported_schemas() {
    let mut resolver = VirtualResolver::new();
    resolver.add(
        "base3.yml",
        json!({
            "fields": {
                "host": {
                    "type": "string",
                    "expr": "compute()",
                    "arg": true
                }
            }
        }),
    );

    let schemas = vec![
        (
            "import".to_string(),
            json!({
                "type": "object",
                "expr": "import('base3.yml')",
                "merge": true
            }),
        ),
    ];
    let merge = vec!["import".to_string()];
    let data = json!({});

    let results = resolve_imports(&schemas, &merge, &data, &resolver, None, AC_DIALECT.document_vocabulary());

    assert_eq!(results.len(), 1);
    let (_, fields) = &results[0];
    let (_, host_schema) = fields.iter().find(|(n, _)| n == "host").unwrap();
    assert!(!host_schema.as_object().unwrap().contains_key("expr"));
    assert!(!host_schema.as_object().unwrap().contains_key("arg"));
}

#[test]
fn resolves_nested_imports() {
    let mut resolver = VirtualResolver::new();
    resolver.add(
        "child.yml",
        json!({
            "fields": {
                "child_field": { "type": "string" }
            }
        }),
    );
    resolver.add(
        "parent.yml",
        json!({
            "fields": {
                "parent_field": { "type": "number" },
                "nested": {
                    "type": "object",
                    "expr": "import('child.yml')",
                    "merge": true
                }
            }
        }),
    );

    let schemas = vec![
        (
            "import".to_string(),
            json!({
                "type": "object",
                "expr": "import('parent.yml')",
                "merge": true
            }),
        ),
    ];
    let merge = vec!["import".to_string()];
    let data = json!({});

    let results = resolve_imports(&schemas, &merge, &data, &resolver, None, AC_DIALECT.document_vocabulary());

    assert_eq!(results.len(), 1);
    let (_, fields) = &results[0];
    assert!(fields.iter().any(|(n, _)| n == "parent_field"));
    assert!(fields.iter().any(|(n, _)| n == "child_field"));
    assert!(!fields.iter().any(|(n, _)| n == "nested"));
}
