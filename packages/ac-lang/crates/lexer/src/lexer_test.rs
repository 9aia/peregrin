use crate::rule::test_utils::{MockLexer, TestKind};
use crate::{DefaultLexError, Lexer, Span, Token};

// --- lex (batch) ---

#[test]
fn mock_lex_returns_tokens() {
    let lexer = MockLexer::<DefaultLexError>::ok(vec![
        Token::new(TestKind::Num(42.0), Span::new(0, 2)),
        Token::new(TestKind::Eof, Span::new(2, 2)),
    ]);

    let tokens = lexer.lex("ignored").unwrap();
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0].kind, TestKind::Num(42.0));
    assert_eq!(tokens[1].kind, TestKind::Eof);
}

#[test]
fn mock_lex_returns_error() {
    let lexer = MockLexer::<DefaultLexError>::err(DefaultLexError::UnexpectedChar {
        span: Span::at(0),
        char: '@',
    });

    let result = lexer.lex("ignored");
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        DefaultLexError::UnexpectedChar {
            span: Span::at(0),
            char: '@'
        }
    );
}

// --- lex_iter (default impl) ---

#[test]
fn default_lex_iter_yields_tokens() {
    let lexer = MockLexer::<DefaultLexError>::ok(vec![
        Token::new(TestKind::Ident("foo".into()), Span::new(0, 3)),
        Token::new(TestKind::Eof, Span::new(3, 3)),
    ]);

    let tokens: Vec<_> = lexer.lex_iter("ignored").collect();
    assert_eq!(tokens.len(), 2);
    assert!(tokens[0].is_ok());
    assert_eq!(
        tokens[0].as_ref().unwrap().kind,
        TestKind::Ident("foo".into())
    );
    assert_eq!(tokens[1].as_ref().unwrap().kind, TestKind::Eof);
}

#[test]
fn default_lex_iter_yields_error_then_stops() {
    let lexer = MockLexer::<DefaultLexError>::err(DefaultLexError::UnexpectedChar {
        span: Span::at(5),
        char: '!',
    });

    let mut iter = lexer.lex_iter("ignored");
    let first = iter.next();
    assert!(first.unwrap().is_err());
    assert!(iter.next().is_none());
}

#[test]
fn default_lex_iter_empty_token_list() {
    let lexer = MockLexer::<DefaultLexError>::ok(vec![]);

    let tokens: Vec<_> = lexer.lex_iter("ignored").collect();
    assert!(tokens.is_empty());
}

// --- dyn Lexer dispatch ---

#[test]
fn dyn_lex_works() {
    let lexer: Box<dyn Lexer<TestKind, DefaultLexError>> =
        Box::new(MockLexer::<DefaultLexError>::ok(vec![
            Token::new(TestKind::Bool(true), Span::new(0, 4)),
            Token::new(TestKind::Eof, Span::new(4, 4)),
        ]));

    let tokens = lexer.lex("ignored").unwrap();
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0].kind, TestKind::Bool(true));
}

#[test]
fn dyn_lex_iter_works() {
    let lexer: Box<dyn Lexer<TestKind, DefaultLexError>> =
        Box::new(MockLexer::<DefaultLexError>::ok(vec![
            Token::new(TestKind::Null, Span::new(0, 4)),
            Token::new(TestKind::Eof, Span::new(4, 4)),
        ]));

    let tokens: Vec<_> = lexer.lex_iter("ignored").collect();
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0].as_ref().unwrap().kind, TestKind::Null);
}

#[test]
fn dyn_lex_error_works() {
    let lexer: Box<dyn Lexer<TestKind, DefaultLexError>> = Box::new(
        MockLexer::<DefaultLexError>::err(DefaultLexError::InvalidNumber {
            span: Span::new(0, 3),
        }),
    );

    let result = lexer.lex("ignored");
    assert!(result.is_err());
}

// --- Send + Sync ---

#[test]
fn lexer_is_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<MockLexer>();
}
