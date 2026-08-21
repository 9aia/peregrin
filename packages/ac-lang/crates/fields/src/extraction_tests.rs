use super::*;
use crate::types::ArgSpec;
use ac_dialect_canonical::AC_DIALECT;
use ac_runtime::RuntimeValue;

#[test]
fn extract_fields_basic() {
    let value = RuntimeValue::new(serde_json::json!({
        "$": {
            "name": { "type": "string" },
            "version": { "type": "number" }
        },
        "name": "ac",
        "version": 1
    }));
    let extracted = extract_fields(value, &AC_DIALECT).unwrap();
    assert_eq!(extracted.schemas.len(), 2);
    assert_eq!(extracted.schemas[0].0, "name");
    assert_eq!(extracted.schemas[1].0, "version");
    assert!(extracted.data.as_object().unwrap().get("$").is_none());
}

#[test]
fn extract_fields_with_functions_as_data() {
    // `functions:` is no longer a reserved key — it stays in data as regular fields.
    let value = RuntimeValue::new(serde_json::json!({
        "$": {
            "name": { "type": "string" },
            "functions": { "type": "object" }
        },
        "functions": {
            "double": {
                "params": { "x": { "type": "number" } },
                "return": { "type": "number" }
            }
        },
        "name": "ac"
    }));
    let extracted = extract_fields(value, &AC_DIALECT).unwrap();
    assert_eq!(extracted.schemas.len(), 2);
    // `functions` stays in data as a regular field
    assert!(
        extracted
            .data
            .as_object()
            .unwrap()
            .get("functions")
            .is_some()
    );
}

#[test]
fn extract_fields_reserved_name() {
    let value = RuntimeValue::new(serde_json::json!({
        "$": {
            "$": { "type": "string" }
        }
    }));
    assert!(extract_fields(value, &AC_DIALECT).is_err());
}

#[test]
fn extract_fields_no_fields_section() {
    let value = RuntimeValue::new(serde_json::json!({"name": "ac"}));
    let extracted = extract_fields(value, &AC_DIALECT).unwrap();
    assert!(extracted.schemas.is_empty());
    assert_eq!(extracted.data.get("name").unwrap().as_str(), Some("ac"));
}

#[test]
fn undeclared_fields_detection() {
    let data = RuntimeValue::new(serde_json::json!({
        "name": "ac",
        "version": 1,
        "extra": true
    }));
    let undeclared = undeclared_fields(&data, &["name", "version"]);
    assert_eq!(undeclared, vec!["extra"]);
}

#[test]
fn extract_fields_inheritable() {
    let value = RuntimeValue::new(serde_json::json!({
        "$": {
            "name": { "type": "string" },
            "permission_mode": { "type": "string", "inheritable": true },
            "env_name": { "type": "string", "inheritable": true, "mutable": false }
        },
        "name": "ac",
        "permission_mode": "read-only",
        "env_name": "prod"
    }));
    let extracted = extract_fields(value, &AC_DIALECT).unwrap();
    let inheritable_set: std::collections::HashSet<&str> =
        extracted.inheritable.iter().map(|s| s.as_str()).collect();
    assert_eq!(
        inheritable_set,
        ["permission_mode", "env_name"].into_iter().collect()
    );
    assert_eq!(extracted.immutable, vec!["env_name"]);
}

#[test]
fn extract_fields_with_function_body_in_data() {
    // Functions with `body` stay in data as regular values.
    // They are callable via `apply` at runtime.
    let value = RuntimeValue::new(serde_json::json!({
        "$": {
            "name": { "type": "string" }
        },
        "functions": {
            "double": {
                "params": { "x": { "type": "number" } },
                "return": { "type": "number" },
                "body": "mul(x, 2)"
            }
        },
        "name": "ac"
    }));
    let extracted = extract_fields(value, &AC_DIALECT).unwrap();
    // `functions` stays in data
    let functions = extracted.data.get("functions").unwrap();
    let double = functions.get("double").unwrap();
    assert_eq!(double.get("body").unwrap().as_str(), Some("mul(x, 2)"));
}

#[test]
fn extract_fields_with_computed() {
    let value = RuntimeValue::new(serde_json::json!({
        "$": {
            "version": { "type": "number" },
            "doubled": { "type": "number", "expr": "mul(version, 2)" }
        },
        "version": 1
    }));
    let extracted = extract_fields(value, &AC_DIALECT).unwrap();
    assert_eq!(extracted.computed.len(), 1);
    assert_eq!(extracted.computed[0].name, "doubled");
    assert_eq!(extracted.computed[0].expr, "mul(version, 2)");
    assert!(extracted.computed[0].arg.is_none());
}

#[test]
fn extract_fields_no_computed() {
    let value = RuntimeValue::new(serde_json::json!({
        "$": { "name": { "type": "string" } },
        "name": "ac"
    }));
    let extracted = extract_fields(value, &AC_DIALECT).unwrap();
    assert!(extracted.computed.is_empty());
}

#[test]
fn extract_fields_with_merge() {
    let value = RuntimeValue::new(serde_json::json!({
        "$": {
            "_base": { "type": "object", "merge": true, "expr": "import('./base.yml')" },
            "host": { "type": "string" }
        },
        "host": "example.com"
    }));
    let extracted = extract_fields(value, &AC_DIALECT).unwrap();
    assert_eq!(extracted.merge, vec!["_base"]);
}

#[test]
fn extract_fields_with_arg_untyped() {
    let value = RuntimeValue::new(serde_json::json!({
        "$": {
            "greeting": { "type": "string", "expr": "concat(arg, \"!\")", "arg": true }
        },
        "greeting": "hello"
    }));
    let extracted = extract_fields(value, &AC_DIALECT).unwrap();
    assert_eq!(extracted.computed.len(), 1);
    assert_eq!(extracted.computed[0].name, "greeting");
    assert!(matches!(extracted.computed[0].arg, Some(ArgSpec::Untyped)));
}

#[test]
fn extract_fields_with_arg_typed() {
    let value = RuntimeValue::new(serde_json::json!({
        "$": {
            "doubled": {
                "type": "number",
                "expr": "mul(arg, 2)",
                "arg": { "type": "number" }
            }
        },
        "doubled": 5
    }));
    let extracted = extract_fields(value, &AC_DIALECT).unwrap();
    assert_eq!(extracted.computed.len(), 1);
    assert!(matches!(extracted.computed[0].arg, Some(ArgSpec::Typed(_))));
}
