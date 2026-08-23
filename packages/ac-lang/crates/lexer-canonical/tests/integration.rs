//! Integration tests for the canonical lexer.
//!
//! Exercises the public API from outside the crate to verify
//! end-to-end lexing behavior.

use ac_lexer::Lexer;
use ac_lexer_canonical::{CanonicalKind, CanonicalLexer, canonical_config};

fn lex(input: &str) -> Vec<CanonicalKind> {
    CanonicalLexer::default()
        .lex(input)
        .unwrap()
        .into_iter()
        .map(|t| t.kind)
        .collect()
}

#[test]
fn lex_basic_expression() {
    let tokens = lex("add(1, 2)");
    assert_eq!(
        tokens,
        vec![
            CanonicalKind::Identifier("add".into()),
            CanonicalKind::CallOpen,
            CanonicalKind::Number(1.0),
            CanonicalKind::Comma,
            CanonicalKind::Number(2.0),
            CanonicalKind::CallClose,
            CanonicalKind::Eof,
        ]
    );
}

#[test]
fn lex_strings() {
    let tokens = lex("\"hello\" 'world'");
    assert_eq!(
        tokens,
        vec![
            CanonicalKind::String("hello".into()),
            CanonicalKind::String("world".into()),
            CanonicalKind::Eof,
        ]
    );
}

#[test]
fn lex_booleans_and_null() {
    let tokens = lex("true false null");
    assert_eq!(
        tokens,
        vec![
            CanonicalKind::Bool(true),
            CanonicalKind::Bool(false),
            CanonicalKind::Null,
            CanonicalKind::Eof,
        ]
    );
}

#[test]
fn lex_comments_filtered() {
    let tokens = lex("// comment\n42 /* block */");
    assert_eq!(
        tokens,
        vec![CanonicalKind::Number(42.0), CanonicalKind::Eof,]
    );
}

#[test]
fn lex_negative_numbers() {
    let tokens = lex("-3 -1.5");
    assert_eq!(
        tokens,
        vec![
            CanonicalKind::Number(-3.0),
            CanonicalKind::Number(-1.5),
            CanonicalKind::Eof,
        ]
    );
}

#[test]
fn lex_namespace_identifiers() {
    let tokens = lex("math.sqrt");
    assert_eq!(
        tokens,
        vec![
            CanonicalKind::Identifier("math.sqrt".into()),
            CanonicalKind::Eof,
        ]
    );
}

#[test]
fn lex_handle_and_as_keywords() {
    let tokens = lex("handle foo as bar");
    assert_eq!(
        tokens,
        vec![
            CanonicalKind::Handle,
            CanonicalKind::Identifier("foo".into()),
            CanonicalKind::As,
            CanonicalKind::Identifier("bar".into()),
            CanonicalKind::Eof,
        ]
    );
}

#[test]
fn lex_iter_streams() {
    let lexer = CanonicalLexer::default();
    let tokens: Vec<_> = lexer.lex_iter("add(1)").collect();
    assert_eq!(tokens.len(), 5);
    assert!(tokens.iter().all(|t| t.is_ok()));
}

#[test]
fn lex_unterminated_string_errors() {
    let result = CanonicalLexer::default().lex("\"oops");
    assert!(result.is_err());
}

#[test]
fn lex_unexpected_char_errors() {
    let result = CanonicalLexer::default().lex("@@@");
    assert!(result.is_err());
}

#[test]
fn custom_config_with_extra_keyword() {
    let config = canonical_config().with_keyword("await", CanonicalKind::Handle);

    let lexer = CanonicalLexer::with_config(config);
    let tokens = lexer.lex("await(foo)").unwrap();
    assert_eq!(tokens[0].kind, CanonicalKind::Handle);
}

#[test]
fn custom_config_without_string_rule() {
    let config = canonical_config().without_rule_by_name("string");
    let lexer = CanonicalLexer::with_config(config);
    let result = lexer.lex("\"hello\"");
    assert!(result.is_err());
}
