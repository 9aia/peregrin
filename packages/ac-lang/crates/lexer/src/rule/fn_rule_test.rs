use crate::{DefaultLexError, LexerConfig, Span, Token};
use crate::rule::{FnRule, LexRule};
use super::test_utils::{TestKind, cfg};

fn hash_rule(
    chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
    _config: &LexerConfig<TestKind, DefaultLexError>,
) -> Option<Result<Token<TestKind>, DefaultLexError>> {
    match chars.peek() {
        Some(&(_, '#')) => {
            let (start, _) = chars.next().unwrap();
            Some(Ok(Token::new(
                TestKind::Ident("hash".into()),
                Span::at(start),
            )))
        }
        _ => None}
}

#[test]
fn fn_rule_matches() {
    let config = cfg();
    let rule = FnRule::new("hash", hash_rule);
    let mut chars = "#hello".char_indices().peekable();
    let result = rule.try_lex(&mut chars, &config);
    assert!(result.is_some());
    let token = result.unwrap().unwrap();
    assert_eq!(token.kind, TestKind::Ident("hash".into()));
}

#[test]
fn fn_rule_no_match() {
    let config = cfg();
    let rule = FnRule::new("hash", hash_rule);
    let mut chars = "hello".char_indices().peekable();
    let result = rule.try_lex(&mut chars, &config);
    assert!(result.is_none());
}

#[test]
fn fn_rule_name() {
    let rule = FnRule::new("hash", hash_rule);
    assert_eq!(rule.name(), "hash");
}

#[test]
fn fn_rule_clone_box() {
    let rule = FnRule::new("hash", hash_rule);
    let cloned = rule.clone_box();
    assert_eq!(cloned.name(), "hash");
}

#[test]
fn fn_rule_clone() {
    let rule = FnRule::new("hash", hash_rule);
    let cloned = rule.clone();
    assert_eq!(cloned.name(), "hash");
}
