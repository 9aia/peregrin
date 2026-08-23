//! [`IdentRule`] — matches identifiers and keywords.

use ac_lexer::{DefaultLexError, IdentFactory, LexError, LexerConfig, Token, TokenKind};
use ac_lexer::LexRule;

/// Matches identifiers and keywords (identifiers in the keyword table
/// are remapped to their token kind).
#[derive(Clone, Default)]
pub struct IdentRule;

impl<K, E> LexRule<K, E> for IdentRule
where
    K: TokenKind,
    K::Factory: IdentFactory<K>,
    E: LexError + From<DefaultLexError>,
{
    fn try_lex(
        &self,
        chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
        config: &LexerConfig<K, E>,
    ) -> Option<Result<Token<K>, E>> {
        match chars.peek() {
            Some(&(_, ch)) if config.factory().is_ident_start(ch) => {
                Some(
                    config.factory().lex_ident(chars)
                        .map(|(text, span)| {
                            let kind = config
                                .lookup_keyword(&text)
                                .cloned()
                                .unwrap_or_else(|| config.factory().ident_kind(text));
                            Token::new(kind, span)
                        })
                        .map_err(E::from)
                )
            }
            _ => None,
        }
    }

    fn name(&self) -> &'static str {
        "ident"
    }

    fn clone_box(&self) -> Box<dyn LexRule<K, E>> {
        Box::new(self.clone())
    }
}
