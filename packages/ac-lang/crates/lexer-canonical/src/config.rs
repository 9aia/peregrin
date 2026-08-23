//! Canonical lexer configuration preset.

use ac_lexer::{DefaultLexError, LexerConfig};
use ac_lexer_common::{
    BlockCommentRule, DelimiterRule, IdentRule, LineCommentRule, NumberRule,
    StringRule};

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
        .with_rule(StringRule)
        .with_rule(NumberRule)
        .with_rule(LineCommentRule)
        .with_rule(BlockCommentRule)
        .with_rule(DelimiterRule)
        .with_rule(IdentRule)
}
