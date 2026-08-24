use super::KeywordRule;
use crate::tokens::{MockFactory, MockKind};
use ac_lexer::{DefaultLexError, LexRule, LexerConfig, Span};

const RULE: KeywordRule = KeywordRule;

#[test]
fn keyword_rule_matches_factory_keyword() {
    let factory = MockFactory::new().with_keyword_kind(|t| match t.as_str() {
        "await" => Some(MockKind::Keyword(t)),
        _ => None,
    });
    let config: LexerConfig<MockKind, DefaultLexError> = LexerConfig::new(factory);
    let mut chars = "await rest".char_indices().peekable();
    let token = RULE.try_lex(&mut chars, &config).unwrap().unwrap();
    assert_eq!(token.kind, MockKind::Keyword("await".into()));
    assert_eq!(token.span, Span::new(0, 5));
}

#[test]
fn keyword_rule_no_match_non_keyword() {
    let factory = MockFactory::new().with_keyword_kind(|_| None);
    let config: LexerConfig<MockKind, DefaultLexError> = LexerConfig::new(factory);
    let mut chars = "foo".char_indices().peekable();
    assert!(RULE.try_lex(&mut chars, &config).is_none());
}

#[test]
fn keyword_rule_no_match_digit() {
    let config: LexerConfig<MockKind, DefaultLexError> = LexerConfig::new(MockFactory::new());
    let mut chars = "42".char_indices().peekable();
    assert!(RULE.try_lex(&mut chars, &config).is_none());
}

#[test]
fn keyword_rule_name() {
    let rule: &dyn LexRule<MockKind, DefaultLexError> = &RULE;
    assert_eq!(rule.name(), "keyword");
}

#[test]
fn keyword_rule_clone_box() {
    let cloned: Box<dyn LexRule<MockKind, DefaultLexError>> = RULE.clone_box();
    assert_eq!(cloned.name(), "keyword");
}