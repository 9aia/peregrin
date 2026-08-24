//! Canonical lexer configuration preset.

use ac_lexer::{DefaultLexError, LexRule, LexerConfig, Span, Token};
use ac_lexer_common::{
    BlockCommentRule, DelimiterRule, IdentRule, LineCommentRule, NumberRule, StringRule,
};

use crate::kind::{CanonicalFactory, CanonicalKind};

/// Returns the canonical AC expression syntax configuration.
///
/// This is the preset used by [`CanonicalLexer::default`](crate::CanonicalLexer::default).
/// Custom dialects can start from this and modify it:
///
/// ```
/// use ac_lexer_canonical::{canonical_config, CanonicalKind};
///
/// let config = canonical_config()
///     .with_keyword("await", CanonicalKind::Handle)
///     .with_delimiter(';', CanonicalKind::Comma);
/// ```
pub fn canonical_config() -> LexerConfig<CanonicalKind, DefaultLexError> {
    LexerConfig::new(CanonicalFactory)
        .with_keyword("true", CanonicalKind::Bool(true))
        .with_keyword("false", CanonicalKind::Bool(false))
        .with_keyword("null", CanonicalKind::Null)
        .with_keyword("handle", CanonicalKind::Handle)
        .with_keyword("as", CanonicalKind::As)
        .with_delimiter('(', CanonicalKind::CallOpen)
        .with_delimiter(')', CanonicalKind::CallClose)
        .with_delimiter('{', CanonicalKind::BlockOpen)
        .with_delimiter('}', CanonicalKind::BlockClose)
        .with_delimiter('[', CanonicalKind::BracketOpen)
        .with_delimiter(']', CanonicalKind::BracketClose)
        .with_delimiter(',', CanonicalKind::Comma)
        .with_delimiter(':', CanonicalKind::Colon)
        .with_delimiter('<', CanonicalKind::LessThan)
        .with_delimiter('>', CanonicalKind::GreaterThan)
        .with_delimiter('=', CanonicalKind::Equal)
        .with_delimiter('.', CanonicalKind::Dot)
        .insert_rule(0, ArrowRule)
        .with_rule(StringRule)
        .with_rule(NumberRule)
        .with_rule(LineCommentRule)
        .with_rule(BlockCommentRule)
        .with_rule(DelimiterRule)
        .with_rule(IdentRule)
}

/// Matches the two-character arrow token `->` before the number rule
/// has a chance to consume the leading `-`.
#[derive(Clone, Default)]
struct ArrowRule;

impl LexRule<CanonicalKind, DefaultLexError> for ArrowRule {
    fn try_lex(
        &self,
        chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
        _config: &LexerConfig<CanonicalKind, DefaultLexError>,
    ) -> Option<Result<Token<CanonicalKind>, DefaultLexError>> {
        let &(_, ch) = chars.peek()?;
        if ch != '-' {
            return None;
        }

        let mut peek = chars.clone();
        peek.next();
        if !matches!(peek.peek(), Some(&(_, '>'))) {
            return None;
        }

        let (start, _) = chars.next().unwrap();
        let (end, _) = chars.next().unwrap();
        Some(Ok(Token::new(
            CanonicalKind::Arrow,
            Span::new(start, end + 1),
        )))
    }

    fn name(&self) -> &'static str {
        "arrow"
    }

    fn clone_box(&self) -> Box<dyn LexRule<CanonicalKind, DefaultLexError>> {
        Box::new(self.clone())
    }
}