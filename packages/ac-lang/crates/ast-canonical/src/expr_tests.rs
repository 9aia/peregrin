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
        Expr::Call { name, args, .. } => {
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

// ── Handle expression tests ──────────────────────────────────────

#[test]
fn handle_expr_construction() {
    let body: Expr<String> = Expr::call("fetch", vec![Expr::value("data".to_string())]);
    let handler = HandlerClause {
        op_name: "fetch".to_string(),
        params: vec!["url".to_string()],
        continuation_name: "k".to_string(),
        body: Expr::value("intercepted".to_string()),
    };
    let expr = Expr::handle(body, vec![handler]);
    match expr {
        Expr::Handle { body, handlers } => {
            assert_eq!(handlers.len(), 1);
            assert_eq!(handlers[0].op_name, "fetch");
            match &*body {
                Expr::Call { name, .. } => assert_eq!(name, "fetch"),
                _ => panic!("expected Call body"),
            }
        }
        _ => panic!("expected Handle variant"),
    }
}

#[test]
fn handle_expr_clone_eq() {
    let expr: Expr<String> = Expr::handle(
        Expr::value("val".to_string()),
        vec![HandlerClause {
            op_name: "fetch".to_string(),
            params: vec!["url".to_string()],
            continuation_name: "k".to_string(),
            body: Expr::value("result".to_string()),
        }],
    );
    let cloned = expr.clone();
    assert_eq!(expr, cloned);
}

#[test]
fn handle_expr_with_empty_handlers() {
    let expr: Expr<i32> = Expr::handle(Expr::value(42), vec![]);
    match expr {
        Expr::Handle { body, handlers } => {
            assert!(handlers.is_empty());
            match &*body {
                Expr::Value(v) => assert_eq!(*v, 42),
                _ => panic!("expected Value body"),
            }
        }
        _ => panic!("expected Handle variant"),
    }
}

#[test]
fn handle_expr_with_multiple_handlers() {
    let expr: Expr<String> = Expr::handle(
        Expr::call("do_work", vec![]),
        vec![
            HandlerClause {
                op_name: "fetch".to_string(),
                params: vec![],
                continuation_name: "k".to_string(),
                body: Expr::value("fetched".to_string()),
            },
            HandlerClause {
                op_name: "read_file".to_string(),
                params: vec!["path".to_string()],
                continuation_name: "k".to_string(),
                body: Expr::value("read".to_string()),
            },
        ],
    );
    match expr {
        Expr::Handle { handlers, .. } => {
            assert_eq!(handlers.len(), 2);
            assert_eq!(handlers[0].op_name, "fetch");
            assert_eq!(handlers[1].op_name, "read_file");
        }
        _ => panic!("expected Handle variant"),
    }
}

#[test]
fn nested_handle_expr() {
    let inner: Expr<String> = Expr::handle(
        Expr::call("fetch", vec![Expr::value("inner".to_string())]),
        vec![HandlerClause {
            op_name: "fetch".to_string(),
            params: vec![],
            continuation_name: "k".to_string(),
            body: Expr::value("inner-handled".to_string()),
        }],
    );
    let outer = Expr::handle(
        inner,
        vec![HandlerClause {
            op_name: "read_file".to_string(),
            params: vec![],
            continuation_name: "k".to_string(),
            body: Expr::value("outer-handled".to_string()),
        }],
    );
    match outer {
        Expr::Handle { body, handlers } => {
            assert_eq!(handlers.len(), 1);
            assert_eq!(handlers[0].op_name, "read_file");
            match &*body {
                Expr::Handle { handlers, .. } => {
                    assert_eq!(handlers.len(), 1);
                    assert_eq!(handlers[0].op_name, "fetch");
                }
                _ => panic!("expected nested Handle"),
            }
        }
        _ => panic!("expected outer Handle"),
    }
}

#[test]
fn handler_clause_clone_eq() {
    let clause: HandlerClause<String> = HandlerClause {
        op_name: "fetch".to_string(),
        params: vec!["url".to_string()],
        continuation_name: "k".to_string(),
        body: Expr::value("result".to_string()),
    };
    let cloned = clause.clone();
    assert_eq!(clause, cloned);
}

#[test]
fn handler_clause_debug() {
    let clause: HandlerClause<String> = HandlerClause {
        op_name: "fetch".to_string(),
        params: vec!["url".to_string()],
        continuation_name: "k".to_string(),
        body: Expr::value("result".to_string()),
    };
    let debug = format!("{:?}", clause);
    assert!(debug.contains("fetch"));
    assert!(debug.contains("HandlerClause"));
}

#[test]
fn deeply_nested_call_expr() {
    // add(mul(sub(10, 3), 2), div(8, 4))
    let expr = Expr::call(
        "add",
        vec![
            Expr::call(
                "mul",
                vec![
                    Expr::call("sub", vec![Expr::value(10), Expr::value(3)]),
                    Expr::value(2),
                ],
            ),
            Expr::call("div", vec![Expr::value(8), Expr::value(4)]),
        ],
    );
    match expr {
        Expr::Call { name, args, .. } => {
            assert_eq!(name, "add");
            assert_eq!(args.len(), 2);
            match &args[0] {
                Expr::Call { name, args, .. } => {
                    assert_eq!(name, "mul");
                    assert_eq!(args.len(), 2);
                }
                _ => panic!("expected mul call"),
            }
        }
        _ => panic!("expected add call"),
    }
}

#[test]
fn call_with_empty_args() {
    let expr: Expr<i32> = Expr::call("now", vec![]);
    match expr {
        Expr::Call { name, args, .. } => {
            assert_eq!(name, "now");
            assert!(args.is_empty());
        }
        _ => panic!("expected Call variant"),
    }
}
