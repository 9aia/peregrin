use ac_lexer::{DefaultLexError, LexRule, Span};
use crate::tokens::MockKind;
use crate::empty_config as cfg;
use super::StringRule;

const RULE: StringRule = StringRule;

#[test]
fn string_rule_matches_double_quote() {
    let config = cfg();
    let mut chars = "\"hello\" rest".char_indices().peekable();
    let result = StringRule.try_lex(&mut chars, &config);
    let token = result.unwrap().unwrap();
    assert_eq!(token.kind, MockKind::Str("hello".into()));
    assert_eq!(token.span, Span::new(0, 7));
}

#[test]
fn string_rule_matches_single_quote() {
    let config = cfg();
    let mut chars = "'world' rest".char_indices().peekable();
    let result = StringRule.try_lex(&mut chars, &config);
    let token = result.unwrap().unwrap();
    assert_eq!(token.kind, MockKind::Str("world".into()));
}

#[test]
fn string_rule_no_match() {
    let config = cfg();
    let mut chars = "hello".char_indices().peekable();
    let result = StringRule.try_lex(&mut chars, &config);
    assert!(result.is_none());
}

#[test]
fn string_rule_unterminated_errors() {
    let config = cfg();
    let mut chars = "\"oops".char_indices().peekable();
    let result = StringRule.try_lex(&mut chars, &config);
    assert!(result.is_some());
    assert!(result.unwrap().is_err());
}

#[test]
fn string_rule_name() {
    let rule: &dyn LexRule<MockKind, DefaultLexError> = &RULE;
    assert_eq!(rule.name(), "string");
}

#[test]
fn string_rule_clone_box() {
    let cloned: Box<dyn LexRule<MockKind, DefaultLexError>> = RULE.clone_box();
    assert_eq!(cloned.name(), "string");
}
