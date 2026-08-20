use super::*;
use crate::test_helpers::{MathResolver, MulResolver};
use crate::types::Namespaces;

#[test]
fn eval_body_user_defined() {
    let ns = Namespaces::new();

    let params = vec![("x".to_string(), serde_json::json!(5))];
    let result = eval_body("mul(x, 2)", &params, &ns, &MulResolver);
    assert_eq!(result.unwrap().as_f64(), Some(10.0));
}

#[test]
fn eval_body_composed_functions() {
    let ns = Namespaces::new();

    let params = vec![("x".to_string(), serde_json::json!(5))];
    let result = eval_body("double(double(x))", &params, &ns, &MathResolver);
    assert_eq!(result.unwrap().as_f64(), Some(20.0));
}

#[test]
fn eval_body_with_doc_namespace() {
    let mut ns = Namespaces::new();
    ns.insert("doc".to_string(), serde_json::json!({"version": 3}));

    let params = vec![("x".to_string(), serde_json::json!(2))];
    let result = eval_body("mul(x, doc.version)", &params, &ns, &MulResolver);
    assert_eq!(result.unwrap().as_f64(), Some(6.0));
}
