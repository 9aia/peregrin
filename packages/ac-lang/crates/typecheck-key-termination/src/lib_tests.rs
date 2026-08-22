use super::*;
use ac_dialect_canonical::AC_DIALECT;
use std::collections::HashMap;

#[test]
fn no_cycle_passes() {
    let mut bodies = HashMap::new();
    bodies.insert("foo".to_string(), "add(1, 2)".to_string());
    bodies.insert("bar".to_string(), "foo(3)".to_string());

    let errors = check(&bodies, &AC_DIALECT);
    assert!(errors.is_empty());
}

#[test]
fn direct_self_recursion_detected() {
    let mut bodies = HashMap::new();
    bodies.insert("loop".to_string(), "loop(1)".to_string());

    let errors = check(&bodies, &AC_DIALECT);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("loop -> loop"));
}

#[test]
fn mutual_recursion_detected() {
    let mut bodies = HashMap::new();
    bodies.insert("a".to_string(), "b(1)".to_string());
    bodies.insert("b".to_string(), "a(1)".to_string());

    let errors = check(&bodies, &AC_DIALECT);
    assert_eq!(errors.len(), 1);
}

#[test]
fn three_way_cycle_detected() {
    let mut bodies = HashMap::new();
    bodies.insert("a".to_string(), "b(1)".to_string());
    bodies.insert("b".to_string(), "c(1)".to_string());
    bodies.insert("c".to_string(), "a(1)".to_string());

    let errors = check(&bodies, &AC_DIALECT);
    assert_eq!(errors.len(), 1);
}

#[test]
fn cycle_through_host_function_not_flagged() {
    let mut bodies = HashMap::new();
    bodies.insert("foo".to_string(), "add(foo(1), 2)".to_string());

    let errors = check(&bodies, &AC_DIALECT);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("foo"));
}

#[test]
fn acyclic_graph_passes() {
    let mut bodies = HashMap::new();
    bodies.insert("a".to_string(), "b(1)".to_string());
    bodies.insert("b".to_string(), "c(1)".to_string());
    bodies.insert("c".to_string(), "add(1, 2)".to_string());

    let errors = check(&bodies, &AC_DIALECT);
    assert!(errors.is_empty());
}

#[test]
fn nested_calls_traversed() {
    let mut bodies = HashMap::new();
    bodies.insert("a".to_string(), "add(b(1), c(2))".to_string());
    bodies.insert("b".to_string(), "c(3)".to_string());
    bodies.insert("c".to_string(), "a(4)".to_string());

    let errors = check(&bodies, &AC_DIALECT);
    assert_eq!(errors.len(), 2);
}

#[test]
fn extract_bodies_from_data() {
    let data = serde_json::json!({
        "functions": {
            "foo": { "body": "add(1, 2)" },
            "bar": { "body": "foo(3)" }
        },
        "name": "test",
        "version": 1
    });

    let bodies = extract_function_bodies(&data);
    assert_eq!(bodies.len(), 2);
    assert_eq!(bodies.get("foo").unwrap(), "add(1, 2)");
    assert_eq!(bodies.get("bar").unwrap(), "foo(3)");
}

#[test]
fn extract_bodies_from_top_level() {
    let data = serde_json::json!({
        "my_func": { "body": "add(1, 2)" },
        "other": 42
    });

    let bodies = extract_function_bodies(&data);
    assert_eq!(bodies.len(), 1);
    assert_eq!(bodies.get("my_func").unwrap(), "add(1, 2)");
}

#[test]
fn unparseable_body_skipped() {
    let mut bodies = HashMap::new();
    bodies.insert("bad".to_string(), "(((invalid".to_string());
    bodies.insert("good".to_string(), "add(1, 2)".to_string());

    let errors = check(&bodies, &AC_DIALECT);
    assert!(errors.is_empty());
}
