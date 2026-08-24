use super::IdentRule;
use crate::empty_config as cfg;
use crate::tokens::MockKind;
use ac_lexer::{DefaultLexError, LexRule, Span};

const RULE: IdentRule = IdentRule;

#[test]
fn ident_rule_matches_identifier() {
    let config = cfg();
    let mut chars = "foo bar".char_indices().peekable();
    let result = IdentRule.try_lex(&mut chars, &config);
    let token = result.unwrap().unwrap();
    assert_eq!(token.kind, MockKind::Ident("foo".into()));
    assert_eq!(token.span, Span::new(0, 3));
}

#[test]
fn ident_rule_matches_namespace_path() {
    let config = cfg();
    let mut chars = "math.sqrt rest".char_indices().peekable();
    let result = IdentRule.try_lex(&mut chars, &config);
    let token = result.unwrap().unwrap();
    assert_eq!(token.kind, MockKind::Ident("math.sqrt".into()));
}

#[test]
fn ident_rule_keyword_remapped() {
    let config = cfg().with_keyword("true", MockKind::Delim('t'));
    let mut chars = "true rest".char_indices().peekable();
    let result = IdentRule.try_lex(&mut chars, &config);
    let token = result.unwrap().unwrap();
    assert_eq!(token.kind, MockKind::Delim('t'));
}

#[test]
fn ident_rule_no_match_digit() {
    let config = cfg();
    let mut chars = "42".char_indices().peekable();
    let result = IdentRule.try_lex(&mut chars, &config);
    assert!(result.is_none());
}

#[test]
fn ident_rule_no_match_symbol() {
    let config = cfg();
    let mut chars = "@@".char_indices().peekable();
    let result = IdentRule.try_lex(&mut chars, &config);
    assert!(result.is_none());
}

#[test]
fn ident_rule_name() {
    let rule: &dyn LexRule<MockKind, DefaultLexError> = &RULE;
    assert_eq!(rule.name(), "ident");
}

#[test]
fn ident_rule_clone_box() {
    let cloned: Box<dyn LexRule<MockKind, DefaultLexError>> = RULE.clone_box();
    assert_eq!(cloned.name(), "ident");
}