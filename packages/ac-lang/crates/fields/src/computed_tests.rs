use super::*;
use crate::test_helpers::{ConcatResolver, MulResolver};
use crate::types::{ArgSpec, ComputedField, Namespaces, NoFunctions};
use ac_dialect_canonical::AC_DIALECT;

#[test]
fn compute_fields_basic() {
    let mut data = serde_json::json!({"version": 3});
    let computed: Vec<ComputedField> = vec![ComputedField::from((
        "doubled".to_string(),
        "mul(version, 2)".to_string(),
    ))];

    let mut ns = Namespaces::new();
    ns.insert("doc".to_string(), data.clone());

    let errors = compute_fields(&mut data, &computed, &mut ns, &MulResolver, &AC_DIALECT);
    assert!(errors.is_empty());
    assert_eq!(data["doubled"].as_f64(), Some(6.0));
}

#[test]
fn compute_fields_path_lookup() {
    let mut data = serde_json::json!({"first": "alice", "last": "smith"});
    let computed: Vec<ComputedField> = vec![ComputedField::from((
        "full".to_string(),
        "concat(first, last)".to_string(),
    ))];

    let mut ns = Namespaces::new();
    ns.insert("doc".to_string(), data.clone());

    let errors = compute_fields(&mut data, &computed, &mut ns, &ConcatResolver, &AC_DIALECT);
    assert!(errors.is_empty());
    assert_eq!(data["full"], "alicesmith");
}

#[test]
fn compute_fields_chained() {
    let mut data = serde_json::json!({"version": 5});
    let computed: Vec<ComputedField> = vec![
        ComputedField::from(("doubled".to_string(), "mul(version, 2)".to_string())),
        ComputedField::from(("quadrupled".to_string(), "mul(doubled, 2)".to_string())),
    ];

    let mut ns = Namespaces::new();
    ns.insert("doc".to_string(), data.clone());

    let errors = compute_fields(&mut data, &computed, &mut ns, &MulResolver, &AC_DIALECT);
    assert!(errors.is_empty());
    assert_eq!(data["doubled"].as_f64(), Some(10.0));
    assert_eq!(data["quadrupled"].as_f64(), Some(20.0));
}

#[test]
fn compute_fields_overrides_existing() {
    let mut data = serde_json::json!({"version": 3, "doubled": 0});
    let computed: Vec<ComputedField> = vec![ComputedField::from((
        "doubled".to_string(),
        "mul(version, 2)".to_string(),
    ))];

    let mut ns = Namespaces::new();
    ns.insert("doc".to_string(), data.clone());

    let errors = compute_fields(&mut data, &computed, &mut ns, &MulResolver, &AC_DIALECT);
    assert!(errors.is_empty());
    assert_eq!(data["doubled"].as_f64(), Some(6.0));
}

#[test]
fn compute_fields_unresolvable() {
    let mut data = serde_json::json!({"version": 3});
    let computed: Vec<ComputedField> = vec![ComputedField::from((
        "bad".to_string(),
        "unknown_fn(version)".to_string(),
    ))];

    let mut ns = Namespaces::new();
    ns.insert("doc".to_string(), data.clone());

    let errors = compute_fields(&mut data, &computed, &mut ns, &NoFunctions, &AC_DIALECT);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].contains("bad"));
}

#[test]
fn compute_fields_empty_is_noop() {
    let mut data = serde_json::json!({"name": "ac"});
    let mut ns = Namespaces::new();
    let errors = compute_fields(&mut data, &[], &mut ns, &NoFunctions, &AC_DIALECT);
    assert!(errors.is_empty());
}

#[test]
fn compute_fields_with_arg_string() {
    let mut data = serde_json::json!({"greeting": "hello"});
    let computed: Vec<ComputedField> = vec![ComputedField {
        name: "greeting".to_string(),
        expr: "concat(arg, \"!\")".to_string(),
        arg: Some(ArgSpec::Untyped),
    }];

    let mut ns = Namespaces::new();
    ns.insert("doc".to_string(), data.clone());

    let errors = compute_fields(&mut data, &computed, &mut ns, &ConcatResolver, &AC_DIALECT);
    assert!(errors.is_empty());
    assert_eq!(data["greeting"], "hello!");
}

#[test]
fn compute_fields_with_arg_number() {
    let mut data = serde_json::json!({"doubled": 5});
    let computed: Vec<ComputedField> = vec![ComputedField {
        name: "doubled".to_string(),
        expr: "mul(arg, 2)".to_string(),
        arg: Some(ArgSpec::Untyped),
    }];

    let mut ns = Namespaces::new();
    ns.insert("doc".to_string(), data.clone());

    let errors = compute_fields(&mut data, &computed, &mut ns, &MulResolver, &AC_DIALECT);
    assert!(errors.is_empty());
    assert_eq!(data["doubled"].as_f64(), Some(10.0));
}

#[test]
fn compute_fields_with_arg_object_path() {
    let mut data = serde_json::json!({
        "endpoint": { "host": "example.com", "port": 8080 }
    });
    let computed: Vec<ComputedField> = vec![ComputedField {
        name: "endpoint".to_string(),
        expr: "arg.host".to_string(),
        arg: Some(ArgSpec::Untyped),
    }];

    let mut ns = Namespaces::new();
    ns.insert("doc".to_string(), data.clone());

    let errors = compute_fields(&mut data, &computed, &mut ns, &NoFunctions, &AC_DIALECT);
    assert!(errors.is_empty());
    assert_eq!(data["endpoint"], "example.com");
}

#[test]
fn compute_fields_with_arg_array_index() {
    let mut data = serde_json::json!({
        "first_item": ["a", "b", "c"]
    });
    let computed: Vec<ComputedField> = vec![ComputedField {
        name: "first_item".to_string(),
        expr: "arg.0".to_string(),
        arg: Some(ArgSpec::Untyped),
    }];

    let mut ns = Namespaces::new();
    ns.insert("doc".to_string(), data.clone());

    let errors = compute_fields(&mut data, &computed, &mut ns, &NoFunctions, &AC_DIALECT);
    assert!(errors.is_empty());
    assert_eq!(data["first_item"], "a");
}

#[test]
fn compute_fields_with_arg_typed_valid() {
    let mut data = serde_json::json!({"doubled": 5});
    let computed: Vec<ComputedField> = vec![ComputedField {
        name: "doubled".to_string(),
        expr: "mul(arg, 2)".to_string(),
        arg: Some(ArgSpec::Typed(serde_json::json!({"type": "number"}))),
    }];

    let mut ns = Namespaces::new();
    ns.insert("doc".to_string(), data.clone());

    let errors = compute_fields(&mut data, &computed, &mut ns, &MulResolver, &AC_DIALECT);
    assert!(errors.is_empty());
    assert_eq!(data["doubled"].as_f64(), Some(10.0));
}

#[test]
fn compute_fields_with_arg_typed_invalid() {
    let mut data = serde_json::json!({"doubled": "not-a-number"});
    let computed: Vec<ComputedField> = vec![ComputedField {
        name: "doubled".to_string(),
        expr: "mul(arg, 2)".to_string(),
        arg: Some(ArgSpec::Typed(serde_json::json!({"type": "number"}))),
    }];

    let mut ns = Namespaces::new();
    ns.insert("doc".to_string(), data.clone());

    let errors = compute_fields(&mut data, &computed, &mut ns, &MulResolver, &AC_DIALECT);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].contains("arg"));
    assert!(errors[0].contains("expected type number"));
}

#[test]
fn compute_fields_chained_with_stale_data() {
    // `doubled` has a stale data value (0). In pass 1, `doubled` computes
    // to 10 and the namespace is updated within the pass, so `quadrupled`
    // reads 10 (not 0) and computes 20 in the same pass.
    let mut data = serde_json::json!({"version": 5, "doubled": 0});
    let computed: Vec<ComputedField> = vec![
        ComputedField::from(("doubled".to_string(), "mul(version, 2)".to_string())),
        ComputedField::from(("quadrupled".to_string(), "mul(doubled, 2)".to_string())),
    ];

    let mut ns = Namespaces::new();
    ns.insert("doc".to_string(), data.clone());

    let errors = compute_fields(&mut data, &computed, &mut ns, &MulResolver, &AC_DIALECT);
    assert!(errors.is_empty());
    assert_eq!(data["doubled"].as_f64(), Some(10.0));
    assert_eq!(data["quadrupled"].as_f64(), Some(20.0));
}

#[test]
fn compute_fields_arg_without_declaration() {
    // `arg` is bound automatically when data is present, even without
    // an `arg:` declaration. The `arg:` property only adds type validation.
    let mut data = serde_json::json!({"greeting": "hello"});
    let computed: Vec<ComputedField> = vec![ComputedField {
        name: "greeting".to_string(),
        expr: "concat(arg, \"!\")".to_string(),
        arg: None,
    }];

    let mut ns = Namespaces::new();
    ns.insert("doc".to_string(), data.clone());

    let errors = compute_fields(&mut data, &computed, &mut ns, &ConcatResolver, &AC_DIALECT);
    assert!(errors.is_empty());
    assert_eq!(data["greeting"], "hello!");
}

#[test]
fn compute_fields_without_arg_backward_compat() {
    let mut data = serde_json::json!({"version": 3});
    let computed: Vec<ComputedField> = vec![ComputedField::from((
        "doubled".to_string(),
        "mul(version, 2)".to_string(),
    ))];

    let mut ns = Namespaces::new();
    ns.insert("doc".to_string(), data.clone());

    let errors = compute_fields(&mut data, &computed, &mut ns, &MulResolver, &AC_DIALECT);
    assert!(errors.is_empty());
    assert_eq!(data["doubled"].as_f64(), Some(6.0));
}
