use ac_ast_common::StmtFactory;

use crate::{CanonicalStmtFactory, Expr, Stmt};

#[test]
fn stmt_expression() {
    let f = CanonicalStmtFactory;
    let s: Stmt<i32> = f.expression(Expr::value(42));
    assert_eq!(s, Stmt::Expression(Expr::value(42)));
}

#[test]
fn stmt_let() {
    let f = CanonicalStmtFactory;
    let s: Stmt<i32> = f.let_("x", Expr::value(42));
    assert_eq!(
        s,
        Stmt::Let {
            name: "x".to_string(),
            value: Expr::value(42)}
    );
}

#[test]
fn stmt_block() {
    let f = CanonicalStmtFactory;
    let s: Stmt<i32> = f.block(vec![
        f.let_("x", Expr::value(1)),
        f.let_("y", Expr::value(2)),
    ]);

    match s {
        Stmt::Block(stmts) => {
            assert_eq!(stmts.len(), 2);
        }
        _ => panic!("expected Block")}
}
