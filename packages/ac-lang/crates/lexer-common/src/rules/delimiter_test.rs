use super::DelimiterRule;
use crate::empty_config as cfg;
use crate::tokens::MockKind;
use ac_lexer::{DefaultLexError, LexRule, Span};

const RULE: DelimiterRule = DelimiterRule;

#[test]
fn delimiter_rule_matches_configured() {
    let config = cfg().with_delimiter('(', MockKind::Delim('('));
    let mut chars = "( rest".char_indices().peekable();
    let result = DelimiterRule.try_lex(&mut chars, &config);
    let token = result.unwrap().unwrap();
    assert_eq!(token.kind, MockKind::Delim('('));
    assert_eq!(token.span, Span::at(0));
}

#[test]
fn delimiter_rule_no_match_unconfigured() {
    let config = cfg();
    let mut chars = "( rest".char_indices().peekable();
    let result = DelimiterRule.try_lex(&mut chars, &config);
    assert!(result.is_none());
}

#[test]
fn delimiter_rule_no_match_letter() {
    let config = cfg().with_delimiter('(', MockKind::Delim('('));
    let mut chars = "hello".char_indices().peekable();
    let result = DelimiterRule.try_lex(&mut chars, &config);
    assert!(result.is_none());
}

#[test]
fn delimiter_rule_name() {
    let rule: &dyn LexRule<MockKind, DefaultLexError> = &RULE;
    assert_eq!(rule.name(), "delimiter");
}

#[test]
fn delimiter_rule_clone_box() {
    let cloned: Box<dyn LexRule<MockKind, DefaultLexError>> = RULE.clone_box();
    assert_eq!(cloned.name(), "delimiter");
}
