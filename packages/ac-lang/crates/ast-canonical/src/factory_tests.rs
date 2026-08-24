use ac_ast_common::ExprFactory;

use crate::{CanonicalExprFactory, Expr};

#[test]
fn factory_value() {
    let f = CanonicalExprFactory;
    assert_eq!(f.value(42), Expr::value(42));
}

#[test]
fn factory_call() {
    let f = CanonicalExprFactory;
    let expr = f.call("add", vec![f.value(1), f.value(2)]);
    assert_eq!(
        expr,
        Expr::call("add", vec![Expr::value(1), Expr::value(2)])
    );
}

#[test]
fn factory_array() {
    let f = CanonicalExprFactory;
    let expr = f.array(vec![f.value(1), f.value(2)]);
    assert_eq!(expr, Expr::array(vec![Expr::value(1), Expr::value(2)]));
}

#[test]
fn factory_object() {
    let f = CanonicalExprFactory;
    let expr = f.object(vec![("x".to_string(), f.value(1))]);
    assert_eq!(expr, Expr::object(vec![("x".to_string(), Expr::value(1))]));
}