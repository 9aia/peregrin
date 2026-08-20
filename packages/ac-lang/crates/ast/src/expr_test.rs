use super::*;

#[test]
fn value_expr() {
    let expr = Expr::value(42);
    assert_eq!(expr, Expr::Value(42));
}

#[test]
fn call_expr() {
    let expr = Expr::call("add", vec![Expr::value(1), Expr::value(2)]);
    match expr {
        Expr::Call { name, args } => {
            assert_eq!(name, "add");
            assert_eq!(args.len(), 2);
        }
        _ => panic!("expected Call variant"),
    }
}

#[test]
fn expr_debug_and_clone() {
    let expr = Expr::call("mul", vec![Expr::value(3)]);
    let cloned = expr.clone();
    assert_eq!(expr, cloned);
}
