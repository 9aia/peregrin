//! Canonical lexer configuration preset.

use ac_lexer::{DefaultLexError, LexRule, LexerConfig, Span, Token};
use ac_lexer_common::{
    BlockCommentRule, DelimiterRule, IdentRule, LineCommentRule, NumberRule, StringRule,
};

use crate::kind::{CanonicalFactory, CanonicalKind};

#[derive(Clone, Default)]
struct OperatorRule;

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
        .with_keyword("is", CanonicalKind::Is)
        .with_keyword("must", CanonicalKind::Must)
        .with_keyword("may", CanonicalKind::May)
        .with_keyword("not", CanonicalKind::Not)
        .with_keyword("when", CanonicalKind::When)
        .with_keyword("else", CanonicalKind::Else)
        .with_keyword("match", CanonicalKind::Match)
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
        .with_delimiter('?', CanonicalKind::Question)
        .insert_rule(0, OperatorRule)
        .with_rule(StringRule)
        .with_rule(NumberRule)
        .with_rule(LineCommentRule)
        .with_rule(BlockCommentRule)
        .with_rule(DelimiterRule)
        .with_rule(IdentRule)
}

/// Matches multi-character operator tokens before they are split by
/// the delimiter, string, or number rules.
impl OperatorRule {
    fn two_or_three(chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>) -> Option<(usize, usize, &'static str)> {
        let &(start, first) = chars.peek()?;

        let mut clone = chars.clone();
        clone.next();
        let &(_, second) = clone.peek()?;

        // Three-character operators must come before their two-character prefixes.
        if matches!((first, second), ('.', '.') | ('>', '>') | ('<', '<') | ('|', '|') | ('&', '&')) {
            let mut peek = clone.clone();
            peek.next();
            if let Some(&(_, third)) = peek.peek() {
                let three = (first, second, third);
                match three {
                    ('.', '.', '.') => {
                        let end = peek.next().unwrap().0;
                        return Some((start, end + 1, "..."));
                    }
                    ('>', '>', '>') => {
                        let end = peek.next().unwrap().0;
                        return Some((start, end + 1, ">>>"));
                    }
                    ('<', '<', '<') => {
                        let end = peek.next().unwrap().0;
                        return Some((start, end + 1, "<<<"));
                    }
                    ('|', '|', '|') => {
                        // Match `||||` (four pipes) before `|||`.
                        let mut four = peek.clone();
                        four.next();
                        if matches!(four.peek(), Some(&(_, '|'))) {
                            let end = four.next().unwrap().0;
                            return Some((start, end + 1, "||||"));
                        }
                        let end = peek.next().unwrap().0;
                        return Some((start, end + 1, "|||"));
                    }
                    ('&', '&', '&') => {
                        let end = peek.next().unwrap().0;
                        return Some((start, end + 1, "&&&"));
                    }
                    _ => {}
                }
            }
        }

        let two = (first, second);
        match two {
            ('-', '>') => {
                let end = clone.next().unwrap().0;
                Some((start, end + 1, "->"))
            }
            ('|', '>') => {
                let end = clone.next().unwrap().0;
                Some((start, end + 1, "|>"))
            }
            ('<', '|') => {
                let end = clone.next().unwrap().0;
                Some((start, end + 1, "<|"))
            }
            ('.', '.') => {
                let end = clone.next().unwrap().0;
                Some((start, end + 1, ".."))
            }
            _ => None,
        }
    }
}

impl LexRule<CanonicalKind, DefaultLexError> for OperatorRule {
    fn try_lex(
        &self,
        chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
        _config: &LexerConfig<CanonicalKind, DefaultLexError>,
    ) -> Option<Result<Token<CanonicalKind>, DefaultLexError>> {
        let (start, end, text) = Self::two_or_three(chars)?;
        let kind = match text {
            "->" => CanonicalKind::Arrow,
            "|>" => CanonicalKind::Pipe,
            "<|" => CanonicalKind::BackwardPipe,
            ">>>" => CanonicalKind::Flow,
            "<<<" => CanonicalKind::Compose,
            "&&&" => CanonicalKind::Fanout,
            "||||" => CanonicalKind::Parallel,
            ".." => CanonicalKind::Range,
            "..." => CanonicalKind::Spread,
            _ => return None,
        };
        for _ in text.chars() {
            chars.next();
        }
        Some(Ok(Token::new(kind, Span::new(start, end + 1))))
    }

    fn name(&self) -> &'static str {
        "canonical_operator"
    }

    fn clone_box(&self) -> Box<dyn LexRule<CanonicalKind, DefaultLexError>> {
        Box::new(self.clone())
    }
}
