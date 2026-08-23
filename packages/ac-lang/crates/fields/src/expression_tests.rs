use super::*;
use crate::test_helpers::{DefaultHost, MathHost};
use crate::types::Namespaces;
use ac_dialect_canonical::AC_DIALECT;
use ac_runtime::FunctionValue;

#[test]
fn eval_body_user_defined() {
    let ns = Namespaces::new();

    let params = vec![("x".to_string(), serde_json::json!(5))];
    let result = eval_body("mul(x, 2)", &params, &ns, &DefaultHost::default(), &AC_DIALECT);
    assert_eq!(result.unwrap().as_f64(), Some(10.0));
}

#[test]
fn eval_body_composed_functions() {
    let ns = Namespaces::new();

    let params = vec![("x".to_string(), serde_json::json!(5))];
    let result = eval_body(
        "double(double(x))",
        &params,
        &ns,
        &MathHost::default(),
        &AC_DIALECT,
    );
    assert_eq!(result.unwrap().as_f64(), Some(20.0));
}

#[test]
fn eval_body_with_doc_namespace() {
    let mut ns = Namespaces::new();
    ns.insert("doc".to_string(), serde_json::json!({"version": 3}));

    let params = vec![("x".to_string(), serde_json::json!(2))];
    let result = eval_body(
        "mul(x, doc.version)",
        &params,
        &ns,
        &DefaultHost::default(),
        &AC_DIALECT,
    );
    assert_eq!(result.unwrap().as_f64(), Some(6.0));
}

#[test]
fn eval_expression_open_paren_no_close() {
    let ns = Namespaces::new();
    let result = eval_expression("(", &ns, &DefaultHost::default(), &AC_DIALECT);
    assert_eq!(result, None);
}

#[test]
fn eval_expression_open_paren_only() {
    let ns = Namespaces::new();
    let result = eval_expression("foo(", &ns, &DefaultHost::default(), &AC_DIALECT);
    assert_eq!(result, None);
}

#[test]
fn eval_expression_unmatched_parens() {
    let ns = Namespaces::new();
    let result = eval_expression("foo(bar", &ns, &DefaultHost::default(), &AC_DIALECT);
    assert_eq!(result, None);
}

#[test]
fn call_function_value_from_param() {
    let mut ns = Namespaces::new();
    let fn_val = FunctionValue {
        params: vec![("x".to_string(), serde_json::json!({"type": "number"}))],
        return_schema: None,
        body: "mul(x, 2)".to_string(),
    };
    ns.insert("doc".to_string(), fn_val.to_json());

    let resolver = DefaultHost::default();
    let result = eval_expression("doc(5)", &ns, &resolver, &AC_DIALECT);
    assert_eq!(result.unwrap().as_f64(), Some(10.0));
}

#[test]
fn call_function_value_from_shorthand() {
    let mut ns = Namespaces::new();
    let fn_val = FunctionValue {
        params: vec![("x".to_string(), serde_json::json!({"type": "number"}))],
        return_schema: None,
        body: "mul(x, 3)".to_string(),
    };
    let mut doc = serde_json::Map::new();
    doc.insert("transform".to_string(), fn_val.to_json());
    ns.insert("doc".to_string(), serde_json::Value::Object(doc));

    let resolver = DefaultHost::default();
    let result = eval_expression("transform(4)", &ns, &resolver, &AC_DIALECT);
    assert_eq!(result.unwrap().as_f64(), Some(12.0));
}

#[test]
fn call_function_value_nested_in_body() {
    let mut ns = Namespaces::new();
    let double_fn = FunctionValue {
        params: vec![("x".to_string(), serde_json::json!({"type": "number"}))],
        return_schema: None,
        body: "mul(x, 2)".to_string(),
    };
    let mut doc = serde_json::Map::new();
    doc.insert("double".to_string(), double_fn.to_json());
    ns.insert("doc".to_string(), serde_json::Value::Object(doc));

    let resolver = DefaultHost::default();
    let result = eval_expression("mul(double(5), 3)", &ns, &resolver, &AC_DIALECT);
    assert_eq!(result.unwrap().as_f64(), Some(30.0));
}
