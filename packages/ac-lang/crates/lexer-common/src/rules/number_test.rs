use super::NumberRule;
use crate::empty_config as cfg;
use crate::tokens::MockKind;
use ac_lexer::{DefaultLexError, LexRule, Span};

const RULE: NumberRule = NumberRule;

#[test]
fn number_rule_matches_integer() {
    let config = cfg();
    let mut chars = "42 rest".char_indices().peekable();
    let result = NumberRule.try_lex(&mut chars, &config);
    let token = result.unwrap().unwrap();
    assert_eq!(token.kind, MockKind::Num(42.0));
    assert_eq!(token.span, Span::new(0, 2));
}

#[test]
fn number_rule_matches_float() {
    let config = cfg();
    let mut chars = "1.5 rest".char_indices().peekable();
    let result = NumberRule.try_lex(&mut chars, &config);
    let token = result.unwrap().unwrap();
    assert_eq!(token.kind, MockKind::Num(1.5));
}

#[test]
fn number_rule_matches_negative() {
    let config = cfg();
    let mut chars = "-3 rest".char_indices().peekable();
    let result = NumberRule.try_lex(&mut chars, &config);
    let token = result.unwrap().unwrap();
    assert_eq!(token.kind, MockKind::Num(-3.0));
}

#[test]
fn number_rule_dash_without_digit_no_match() {
    let config = cfg();
    let mut chars = "- rest".char_indices().peekable();
    let result = NumberRule.try_lex(&mut chars, &config);
    assert!(result.is_none());
}

#[test]
fn number_rule_no_match() {
    let config = cfg();
    let mut chars = "hello".char_indices().peekable();
    let result = NumberRule.try_lex(&mut chars, &config);
    assert!(result.is_none());
}

#[test]
fn number_rule_name() {
    let rule: &dyn LexRule<MockKind, DefaultLexError> = &RULE;
    assert_eq!(rule.name(), "number");
}

#[test]
fn number_rule_clone_box() {
    let cloned: Box<dyn LexRule<MockKind, DefaultLexError>> = RULE.clone_box();
    assert_eq!(cloned.name(), "number");
}