use super::BlockCommentRule;
use crate::empty_config as cfg;
use crate::tokens::MockKind;
use ac_lexer::{DefaultLexError, LexRule, Span};

const RULE: BlockCommentRule = BlockCommentRule;

#[test]
fn block_comment_rule_matches() {
    let config = cfg();
    let mut chars = "/* hello */ rest".char_indices().peekable();
    let result = BlockCommentRule.try_lex(&mut chars, &config);
    let token = result.unwrap().unwrap();
    assert_eq!(token.kind, MockKind::Comment(" hello ".into()));
    assert_eq!(token.span, Span::new(0, 11));
}

#[test]
fn block_comment_rule_nested_stars() {
    let config = cfg();
    let mut chars = "/* a * b */ rest".char_indices().peekable();
    let result = BlockCommentRule.try_lex(&mut chars, &config);
    let token = result.unwrap().unwrap();
    assert_eq!(token.kind, MockKind::Comment(" a * b ".into()));
}

#[test]
fn block_comment_rule_unterminated_errors() {
    let config = cfg();
    let mut chars = "/* oops".char_indices().peekable();
    let result = BlockCommentRule.try_lex(&mut chars, &config);
    assert!(result.is_some());
    assert!(result.unwrap().is_err());
}

#[test]
fn block_comment_rule_no_match_single_slash() {
    let config = cfg();
    let mut chars = "/ hello".char_indices().peekable();
    let result = BlockCommentRule.try_lex(&mut chars, &config);
    assert!(result.is_none());
}

#[test]
fn block_comment_rule_no_match_line_comment() {
    let config = cfg();
    let mut chars = "// hello".char_indices().peekable();
    let result = BlockCommentRule.try_lex(&mut chars, &config);
    assert!(result.is_none());
}

#[test]
fn block_comment_rule_no_match_letter() {
    let config = cfg();
    let mut chars = "hello".char_indices().peekable();
    let result = BlockCommentRule.try_lex(&mut chars, &config);
    assert!(result.is_none());
}

#[test]
fn block_comment_rule_name() {
    let rule: &dyn LexRule<MockKind, DefaultLexError> = &RULE;
    assert_eq!(rule.name(), "block_comment");
}

#[test]
fn block_comment_rule_clone_box() {
    let cloned: Box<dyn LexRule<MockKind, DefaultLexError>> = RULE.clone_box();
    assert_eq!(cloned.name(), "block_comment");
}
