use super::*;
use ac_module_resolver_core::{ModuleResolver, ResolvedModule};
use ac_typecheck_schema_json::JsonSchemaInterpreter;
use serde_json::{Value as JsonValue, json};

struct StaticResolver {
    modules: std::collections::HashMap<String, JsonValue>,
}

impl StaticResolver {
    fn new() -> Self {
        StaticResolver {
            modules: std::collections::HashMap::new(),
        }
    }

    fn add_module(&mut self, path: &str, schemas: Vec<(String, JsonValue)>, merge: Vec<String>) {
        let data = json!({});
        self.modules.insert(
            path.to_string(),
            json!({
                "_schemas": schemas,
                "_merge": merge,
                "_data": data,
            }),
        );
    }
}

impl ModuleResolver for StaticResolver {
    fn resolve(&self, path: &str, _base: Option<&std::path::Path>) -> Option<ResolvedModule> {
        let entry = self.modules.get(path)?;
        let schemas: Vec<(String, JsonValue)> = entry
            .get("_schemas")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| {
                        v.as_array().and_then(|pair| {
                            pair.get(0)?
                                .as_str()
                                .map(|s| (s.to_string(), pair.get(1).cloned().unwrap_or(json!({}))))
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        let merge: Vec<String> = entry
            .get("_merge")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let data = entry.get("_data").cloned().unwrap_or(json!({}));

        Some(ResolvedModule {
            schemas,
            merge,
            data,
            base_path: None,
        })
    }
}

fn make_env(schemas: &[(String, JsonValue)]) -> TypeEnv {
    let interp = JsonSchemaInterpreter;
    let mut env = TypeEnv::new();
    for (name, schema) in schemas {
        env.insert(name.clone(), interp.interpret(schema));
    }
    env
}

#[test]
fn no_merge_fields_no_errors() {
    let resolver = StaticResolver::new();
    let interp = JsonSchemaInterpreter;
    let env = TypeEnv::new();
    let data = json!({});
    let errors = check_imports(&[], &[], &data, &resolver, &interp, &env, None);
    assert!(errors.is_empty());
}

#[test]
fn import_compatible_types_no_errors() {
    let mut resolver = StaticResolver::new();
    resolver.add_module(
        "base.yml",
        vec![("host".to_string(), json!({ "type": "string" }))],
        vec![],
    );

    let interp = JsonSchemaInterpreter;
    let local_schemas = vec![
        ("host".to_string(), json!({ "type": "string" })),
        (
            "_base".to_string(),
            json!({
                "type": "object",
                "merge": true,
                "expr": "import('base.yml')"
            }),
        ),
    ];
    let env = make_env(&local_schemas);
    let data = json!({});
    let errors = check_imports(
        &local_schemas,
        &["_base".to_string()],
        &data,
        &resolver,
        &interp,
        &env,
        None,
    );
    assert!(errors.is_empty());
}

#[test]
fn import_incompatible_types_reports_error() {
    let mut resolver = StaticResolver::new();
    resolver.add_module(
        "base.yml",
        vec![("host".to_string(), json!({ "type": "number" }))],
        vec![],
    );

    let interp = JsonSchemaInterpreter;
    let local_schemas = vec![
        ("host".to_string(), json!({ "type": "string" })),
        (
            "_base".to_string(),
            json!({
                "type": "object",
                "merge": true,
                "expr": "import('base.yml')"
            }),
        ),
    ];
    let env = make_env(&local_schemas);
    let data = json!({});
    let errors = check_imports(
        &local_schemas,
        &["_base".to_string()],
        &data,
        &resolver,
        &interp,
        &env,
        None,
    );
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].field, "host");
}

#[test]
fn import_new_field_no_error() {
    let mut resolver = StaticResolver::new();
    resolver.add_module(
        "base.yml",
        vec![("port".to_string(), json!({ "type": "number" }))],
        vec![],
    );

    let interp = JsonSchemaInterpreter;
    let local_schemas = vec![
        ("host".to_string(), json!({ "type": "string" })),
        (
            "_base".to_string(),
            json!({
                "type": "object",
                "merge": true,
                "expr": "import('base.yml')"
            }),
        ),
    ];
    let env = make_env(&local_schemas);
    let data = json!({});
    let errors = check_imports(
        &local_schemas,
        &["_base".to_string()],
        &data,
        &resolver,
        &interp,
        &env,
        None,
    );
    assert!(errors.is_empty());
}

#[test]
fn unresolvable_import_reports_error() {
    let resolver = StaticResolver::new();
    let interp = JsonSchemaInterpreter;
    let local_schemas = vec![(
        "_base".to_string(),
        json!({
            "type": "object",
            "merge": true,
            "expr": "import('missing.yml')"
        }),
    )];
    let env = make_env(&local_schemas);
    let data = json!({});
    let errors = check_imports(
        &local_schemas,
        &["_base".to_string()],
        &data,
        &resolver,
        &interp,
        &env,
        None,
    );
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("cannot resolve"));
}

#[test]
fn data_driven_import() {
    let mut resolver = StaticResolver::new();
    resolver.add_module(
        "base.yml",
        vec![("host".to_string(), json!({ "type": "string" }))],
        vec![],
    );

    let interp = JsonSchemaInterpreter;
    let local_schemas = vec![
        ("host".to_string(), json!({ "type": "string" })),
        (
            "_base".to_string(),
            json!({
                "type": "object",
                "merge": true,
                "expr": "import(arg)"
            }),
        ),
    ];
    let env = make_env(&local_schemas);
    let data = json!({ "_base": "base.yml" });
    let errors = check_imports(
        &local_schemas,
        &["_base".to_string()],
        &data,
        &resolver,
        &interp,
        &env,
        None,
    );
    assert!(errors.is_empty());
}

#[test]
fn circular_import_no_infinite_loop() {
    let mut resolver = StaticResolver::new();
    resolver.add_module(
        "a.yml",
        vec![(
            "_b".to_string(),
            json!({
                "type": "object",
                "merge": true,
                "expr": "import('b.yml')"
            }),
        )],
        vec!["_b".to_string()],
    );
    resolver.add_module(
        "b.yml",
        vec![(
            "_a".to_string(),
            json!({
                "type": "object",
                "merge": true,
                "expr": "import('a.yml')"
            }),
        )],
        vec!["_a".to_string()],
    );

    let interp = JsonSchemaInterpreter;
    let local_schemas = vec![(
        "_a".to_string(),
        json!({
            "type": "object",
            "merge": true,
            "expr": "import('a.yml')"
        }),
    )];
    let env = make_env(&local_schemas);
    let data = json!({});
    let errors = check_imports(
        &local_schemas,
        &["_a".to_string()],
        &data,
        &resolver,
        &interp,
        &env,
        None,
    );
    assert!(errors.is_empty());
}
