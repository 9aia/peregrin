use ac_lexer::{DefaultLexError, LexRule, Span};
use crate::tokens::MockKind;
use crate::empty_config as cfg;
use super::LineCommentRule;

const RULE: LineCommentRule = LineCommentRule;

#[test]
fn line_comment_rule_matches() {
    let config = cfg();
    let mut chars = "// hello\nrest".char_indices().peekable();
    let result = LineCommentRule.try_lex(&mut chars, &config);
    let token = result.unwrap().unwrap();
    assert_eq!(token.kind, MockKind::Comment(" hello".into()));
    assert_eq!(token.span, Span::new(0, 8));
}

#[test]
fn line_comment_rule_matches_no_newline() {
    let config = cfg();
    let mut chars = "// hello".char_indices().peekable();
    let result = LineCommentRule.try_lex(&mut chars, &config);
    let token = result.unwrap().unwrap();
    assert_eq!(token.kind, MockKind::Comment(" hello".into()));
}

#[test]
fn line_comment_rule_no_match_single_slash() {
    let config = cfg();
    let mut chars = "/ hello".char_indices().peekable();
    let result = LineCommentRule.try_lex(&mut chars, &config);
    assert!(result.is_none());
}

#[test]
fn line_comment_rule_no_match_block_comment() {
    let config = cfg();
    let mut chars = "/* hello */".char_indices().peekable();
    let result = LineCommentRule.try_lex(&mut chars, &config);
    assert!(result.is_none());
}

#[test]
fn line_comment_rule_no_match_letter() {
    let config = cfg();
    let mut chars = "hello".char_indices().peekable();
    let result = LineCommentRule.try_lex(&mut chars, &config);
    assert!(result.is_none());
}

#[test]
fn line_comment_rule_name() {
    let rule: &dyn LexRule<MockKind, DefaultLexError> = &RULE;
    assert_eq!(rule.name(), "line_comment");
}

#[test]
fn line_comment_rule_clone_box() {
    let cloned: Box<dyn LexRule<MockKind, DefaultLexError>> = RULE.clone_box();
    assert_eq!(cloned.name(), "line_comment");
}
