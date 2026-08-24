use super::OperatorRule;
use crate::tokens::{MockFactory, MockKind};
use ac_lexer::{DefaultLexError, LexRule, LexerConfig, Span};

const RULE: OperatorRule = OperatorRule;

#[test]
fn operator_rule_matches_factory_operator() {
    let factory = MockFactory::new().with_operator_kind(|t| match t.as_str() {
        "+" => Some(MockKind::Operator(t)),
        _ => None,
    });
    let config: LexerConfig<MockKind, DefaultLexError> = LexerConfig::new(factory);
    let mut chars = "+ rest".char_indices().peekable();
    let token = RULE.try_lex(&mut chars, &config).unwrap().unwrap();
    assert_eq!(token.kind, MockKind::Operator("+".into()));
    assert_eq!(token.span, Span::at(0));
}

#[test]
fn operator_rule_no_match_unconfigured() {
    let config: LexerConfig<MockKind, DefaultLexError> = LexerConfig::new(MockFactory::new());
    let mut chars = "+".char_indices().peekable();
    assert!(RULE.try_lex(&mut chars, &config).is_none());
}

#[test]
fn operator_rule_no_match_letter() {
    let factory = MockFactory::new().with_operator_kind(|_| None);
    let config: LexerConfig<MockKind, DefaultLexError> = LexerConfig::new(factory);
    let mut chars = "hello".char_indices().peekable();
    assert!(RULE.try_lex(&mut chars, &config).is_none());
}

#[test]
fn operator_rule_name() {
    let rule: &dyn LexRule<MockKind, DefaultLexError> = &RULE;
    assert_eq!(rule.name(), "operator");
}

#[test]
fn operator_rule_clone_box() {
    let cloned: Box<dyn LexRule<MockKind, DefaultLexError>> = RULE.clone_box();
    assert_eq!(cloned.name(), "operator");
}