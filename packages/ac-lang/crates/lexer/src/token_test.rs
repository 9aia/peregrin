use crate::{Span, Token, TokenKind};
use crate::rule::test_utils::TestKind;

#[test]
fn token_new() {
    let t = Token::new(TestKind::Null, Span::new(0, 3));
    assert_eq!(t.kind, TestKind::Null);
    assert_eq!(t.span, Span::new(0, 3));
}

#[test]
fn token_kind_is_eof() {
    assert!(TestKind::Eof.is_eof());
    assert!(!TestKind::Null.is_eof());
}

#[test]
fn token_kind_display() {
    assert_eq!(TestKind::Null.to_string(), "null");
    assert_eq!(TestKind::Eof.to_string(), "eof");
}
