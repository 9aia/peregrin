use ac_ast::Expr;
use ac_lexer::Lexer;
use ac_lexer_canonical::CanonicalLexer;
use ac_parser::{ParseError, Parser};
use crate::CanonicalParser;
use ac_runtime::NonFunctionValue;

use serde_json::json;

fn parse(input: &str) -> Result<Expr<NonFunctionValue>, ParseError> {
    let tokens = CanonicalLexer::default().lex(input).map_err(|e| ParseError::LexError {
        message: e.to_string(),
    })?;
    CanonicalParser.parse(tokens)
}

#[test]
fn parse_string_literal() {
    let expr = parse(r#""hello""#).unwrap();
    assert_eq!(expr, Expr::Value(NonFunctionValue::new(json!("hello"))));
}

#[test]
fn parse_number_literal() {
    let expr = parse("42").unwrap();
    assert_eq!(expr, Expr::Value(NonFunctionValue::new(json!(42.0))));
}

#[test]
fn parse_bool_literal() {
    let expr = parse("true").unwrap();
    assert_eq!(expr, Expr::Value(NonFunctionValue::new(json!(true))));
}

#[test]
fn parse_null_literal() {
    let expr = parse("null").unwrap();
    assert_eq!(expr, Expr::Value(NonFunctionValue::new(json!(null))));
}

#[test]
fn parse_simple_call() {
    let expr = parse("add(1, 2)").unwrap();
    assert_eq!(
        expr,
        Expr::call("add", vec![
            Expr::Value(NonFunctionValue::new(json!(1.0))),
            Expr::Value(NonFunctionValue::new(json!(2.0))),
        ])
    );
}

#[test]
fn parse_nested_call() {
    let expr = parse("add(mul(2, 3), 4)").unwrap();
    assert_eq!(
        expr,
        Expr::call("add", vec![
            Expr::call("mul", vec![
                Expr::Value(NonFunctionValue::new(json!(2.0))),
                Expr::Value(NonFunctionValue::new(json!(3.0))),
            ]),
            Expr::Value(NonFunctionValue::new(json!(4.0))),
        ])
    );
}

#[test]
fn parse_namespaced_call() {
    let expr = parse("math.sqrt(16)").unwrap();
    assert_eq!(
        expr,
        Expr::call("math.sqrt", vec![
            Expr::Value(NonFunctionValue::new(json!(16.0))),
        ])
    );
}

#[test]
fn parse_call_no_args() {
    let expr = parse("now()").unwrap();
    assert_eq!(expr, Expr::call("now", vec![]));
}

#[test]
fn parse_handle_expression() {
    let expr = parse(r#"handle("fetch"(url) as k: k(42))"#).unwrap();
    match expr {
        Expr::Handle { body, handlers } => {
            assert_eq!(handlers.len(), 1);
            assert_eq!(handlers[0].op_name, "fetch");
            assert_eq!(handlers[0].params, vec!["url"]);
            assert_eq!(handlers[0].continuation_name, "k");
            // body is the handler clause's body
            let _ = body;
        }
        other => panic!("expected Handle, got {other:?}"),
    }
}

#[test]
fn parse_unexpected_token_errors() {
    let result = parse(",");
    assert!(result.is_err());
}

#[test]
fn parse_unclosed_call_errors() {
    let result = parse("add(1, 2");
    assert!(result.is_err());
}

#[test]
fn parse_empty_input_errors() {
    let result = parse("");
    assert!(result.is_err());
}

// --- Array literals ---

#[test]
fn parse_empty_array() {
    let expr = parse("[]").unwrap();
    assert_eq!(expr, Expr::array(vec![]));
}

#[test]
fn parse_array_of_literals() {
    let expr = parse("[1, 2, 3]").unwrap();
    assert_eq!(
        expr,
        Expr::array(vec![
            Expr::Value(NonFunctionValue::new(json!(1.0))),
            Expr::Value(NonFunctionValue::new(json!(2.0))),
            Expr::Value(NonFunctionValue::new(json!(3.0))),
        ])
    );
}

#[test]
fn parse_array_of_mixed() {
    let expr = parse(r#"["hello", 42, true, null]"#).unwrap();
    assert_eq!(
        expr,
        Expr::array(vec![
            Expr::Value(NonFunctionValue::new(json!("hello"))),
            Expr::Value(NonFunctionValue::new(json!(42.0))),
            Expr::Value(NonFunctionValue::new(json!(true))),
            Expr::Value(NonFunctionValue::new(json!(null))),
        ])
    );
}

#[test]
fn parse_array_with_calls() {
    let expr = parse("[add(1, 2), mul(3, 4)]").unwrap();
    assert_eq!(
        expr,
        Expr::array(vec![
            Expr::call("add", vec![
                Expr::Value(NonFunctionValue::new(json!(1.0))),
                Expr::Value(NonFunctionValue::new(json!(2.0))),
            ]),
            Expr::call("mul", vec![
                Expr::Value(NonFunctionValue::new(json!(3.0))),
                Expr::Value(NonFunctionValue::new(json!(4.0))),
            ]),
        ])
    );
}

#[test]
fn parse_nested_array() {
    let expr = parse("[[1, 2], [3, 4]]").unwrap();
    assert_eq!(
        expr,
        Expr::array(vec![
            Expr::array(vec![
                Expr::Value(NonFunctionValue::new(json!(1.0))),
                Expr::Value(NonFunctionValue::new(json!(2.0))),
            ]),
            Expr::array(vec![
                Expr::Value(NonFunctionValue::new(json!(3.0))),
                Expr::Value(NonFunctionValue::new(json!(4.0))),
            ]),
        ])
    );
}

#[test]
fn parse_unclosed_array_errors() {
    let result = parse("[1, 2");
    assert!(result.is_err());
}

// --- Object literals ---

#[test]
fn parse_empty_object() {
    let expr = parse("{}").unwrap();
    assert_eq!(expr, Expr::object(vec![]));
}

#[test]
fn parse_object_with_identifier_keys() {
    let expr = parse("{a: 1, b: 2}").unwrap();
    assert_eq!(
        expr,
        Expr::object(vec![
            ("a".into(), Expr::Value(NonFunctionValue::new(json!(1.0)))),
            ("b".into(), Expr::Value(NonFunctionValue::new(json!(2.0)))),
        ])
    );
}

#[test]
fn parse_object_with_string_keys() {
    let expr = parse(r#"{"key": "value"}"#).unwrap();
    assert_eq!(
        expr,
        Expr::object(vec![
            ("key".into(), Expr::Value(NonFunctionValue::new(json!("value")))),
        ])
    );
}

#[test]
fn parse_object_with_call_values() {
    let expr = parse("{result: add(1, 2)}").unwrap();
    assert_eq!(
        expr,
        Expr::object(vec![
            ("result".into(), Expr::call("add", vec![
                Expr::Value(NonFunctionValue::new(json!(1.0))),
                Expr::Value(NonFunctionValue::new(json!(2.0))),
            ])),
        ])
    );
}

#[test]
fn parse_nested_object() {
    let expr = parse("{outer: {inner: 42}}").unwrap();
    assert_eq!(
        expr,
        Expr::object(vec![
            ("outer".into(), Expr::object(vec![
                ("inner".into(), Expr::Value(NonFunctionValue::new(json!(42.0)))),
            ])),
        ])
    );
}

#[test]
fn parse_object_in_array() {
    let expr = parse("[{a: 1}, {b: 2}]").unwrap();
    assert_eq!(
        expr,
        Expr::array(vec![
            Expr::object(vec![("a".into(), Expr::Value(NonFunctionValue::new(json!(1.0))))]),
            Expr::object(vec![("b".into(), Expr::Value(NonFunctionValue::new(json!(2.0))))]),
        ])
    );
}

#[test]
fn parse_unclosed_object_errors() {
    let result = parse("{a: 1");
    assert!(result.is_err());
}
