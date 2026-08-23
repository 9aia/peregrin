use super::*;
use crate::test_helpers::{DefaultHost, MathHost};
use crate::types::Namespaces;
use ac_dialect_canonical::AC_DIALECT;

#[test]
fn interpolate_basic() {
    let mut ns = Namespaces::new();
    ns.insert("doc".to_string(), serde_json::json!({"name": "alice"}));
    ns.insert(
        "env".to_string(),
        serde_json::json!({"HOME": "/home/alice"}),
    );

    let value = serde_json::json!({
        "greeting": "hello, ${doc.name}!",
        "home": "${env.HOME}"
    });
    let result = interpolate(&value, &ns, &DefaultHost::default(), &AC_DIALECT);
    assert_eq!(result["greeting"], "hello, alice!");
    assert_eq!(result["home"], "/home/alice");
}

#[test]
fn interpolate_nested() {
    let mut ns = Namespaces::new();
    ns.insert(
        "doc".to_string(),
        serde_json::json!({"author": {"email": "alice@example.com"}}),
    );

    let value = serde_json::json!("${doc.author.email}");
    let result = interpolate(&value, &ns, &DefaultHost::default(), &AC_DIALECT);
    assert_eq!(result, "alice@example.com");
}

#[test]
fn interpolate_unresolved() {
    let ns = Namespaces::new();
    let value = serde_json::json!("${unknown.key}");
    let result = interpolate(&value, &ns, &DefaultHost::default(), &AC_DIALECT);
    assert_eq!(result, "${unknown.key}");
}

#[test]
fn interpolate_shorthand() {
    let mut ns = Namespaces::new();
    ns.insert("doc".to_string(), serde_json::json!({"name": "alice"}));

    let value = serde_json::json!("${name}");
    let result = interpolate(&value, &ns, &DefaultHost::default(), &AC_DIALECT);
    assert_eq!(result, "alice");
}

#[test]
fn interpolate_function_call() {
    let mut ns = Namespaces::new();
    ns.insert("doc".to_string(), serde_json::json!({"version": 3}));

    let value = serde_json::json!("${add(1, 2)}");
    let result = interpolate(&value, &ns, &DefaultHost::default(), &AC_DIALECT);
    assert_eq!(result, 3.0);
}

#[test]
fn interpolate_function_with_path_arg() {
    let mut ns = Namespaces::new();
    ns.insert("doc".to_string(), serde_json::json!({"version": 3}));

    let value = serde_json::json!("${double(version)}");
    let result = interpolate(&value, &ns, &MathHost::default(), &AC_DIALECT);
    assert_eq!(result, 6.0);
}

#[test]
fn interpolate_nested_function_calls() {
    let ns = Namespaces::new();

    let value = serde_json::json!("${add(mul(2, 3), 4)}");
    let result = interpolate(&value, &ns, &MathHost::default(), &AC_DIALECT);
    assert_eq!(result, 10.0);
}

#[test]
fn interpolate_function_preserves_type() {
    let ns = Namespaces::new();

    let value = serde_json::json!("${len(\"hello\")}");
    let result = interpolate(&value, &ns, &DefaultHost::default(), &AC_DIALECT);
    assert_eq!(result, 5);
    assert!(result.is_number());
}

#[test]
fn interpolate_scoped_inheritance() {
    let mut ns = Namespaces::new();
    ns.insert(
        "doc".to_string(),
        serde_json::json!({"permission_mode": "read-only"}),
    );

    let value = serde_json::json!({
        "config": {
            "message": "${permission_mode}",
            "server": {
                "permission_mode": "read-write",
                "message": "${permission_mode}"
            },
            "deep": {
                "message": "${permission_mode}"
            }
        },
        "server": {
            "message": "${permission_mode}"
        }
    });

    let inheritable = vec!["permission_mode".to_string()];
    let result = interpolate_scoped(
        &value,
        &ns,
        &inheritable,
        &DefaultHost::default(),
        &AC_DIALECT,
    );

    assert_eq!(result["config"]["message"], "read-only");
    assert_eq!(result["config"]["server"]["message"], "read-write");
    assert_eq!(result["config"]["deep"]["message"], "read-only");
    assert_eq!(result["server"]["message"], "read-only");
}

#[test]
fn interpolate_scoped_empty_inheritable_matches_interpolate() {
    let mut ns = Namespaces::new();
    ns.insert("doc".to_string(), serde_json::json!({"name": "alice"}));

    let value = serde_json::json!({"greeting": "hello, ${doc.name}!"});
    let result_scoped = interpolate_scoped(&value, &ns, &[], &DefaultHost::default(), &AC_DIALECT);
    let result_plain = interpolate(&value, &ns, &DefaultHost::default(), &AC_DIALECT);
    assert_eq!(result_scoped, result_plain);
}
