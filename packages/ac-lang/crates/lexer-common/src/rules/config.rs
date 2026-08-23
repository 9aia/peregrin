//! Config builders for tests.

use ac_lexer::{DefaultLexError, LexerConfig};
use crate::tokens::{MockFactory, MockKind};
use super::{DelimiterRule, IdentRule, NumberRule, StringRule};

/// Returns an empty `LexerConfig<MockKind, DefaultLexError>` with just the
/// factory — no rules, no keywords, no delimiters. Use this when tests need
/// full control.
pub fn empty_config() -> LexerConfig<MockKind, DefaultLexError> {
    LexerConfig::new(MockFactory::new())
}

/// Returns a `LexerConfig<MockKind, DefaultLexError>` with all built-in rules
/// and a few common keywords/delimiters pre-configured.
pub fn test_config() -> LexerConfig<MockKind, DefaultLexError> {
    LexerConfig::new(MockFactory::new())
        .with_keyword("true", MockKind::Bool(true))
        .with_keyword("false", MockKind::Bool(false))
        .with_keyword("null", MockKind::Null)
        .with_delimiter('(', MockKind::Delim('('))
        .with_delimiter(')', MockKind::Delim(')'))
        .with_delimiter('{', MockKind::Delim('{'))
        .with_delimiter('}', MockKind::Delim('}'))
        .with_delimiter('[', MockKind::Delim('['))
        .with_delimiter(']', MockKind::Delim(']'))
        .with_delimiter(',', MockKind::Comma)
        .with_delimiter(':', MockKind::Colon)
        .with_rule(StringRule)
        .with_rule(NumberRule)
        .with_rule(DelimiterRule)
        .with_rule(IdentRule)
}
