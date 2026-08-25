//! [`StringRule`] — matches string literals.

use handl_lexer::LexRule;
use handl_lexer::{DefaultLexError, LexError, LexerConfig, StringFactory, Token, TokenKind};

/// Matches string literals using the factory's [`StringFactory::is_string_start`]
/// and [`StringFactory::lex_string`].
#[derive(Clone, Default)]
pub struct StringRule;

impl<K, E> LexRule<K, E> for StringRule
where
    K: TokenKind,
    K::Factory: StringFactory<K>,
    E: LexError + From<DefaultLexError>,
{
    fn try_lex(
        &self,
        chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
        config: &LexerConfig<K, E>,
    ) -> Option<Result<Token<K>, E>> {
        match chars.peek() {
            Some(&(_, ch)) if config.factory().is_string_start(ch) => Some(
                config
                    .factory()
                    .lex_string(chars)
                    .map(|(value, span)| Token::new(config.factory().string_kind(value), span))
                    .map_err(E::from),
            ),
            _ => None,
        }
    }

    fn name(&self) -> &'static str {
        "string"
    }

    fn clone_box(&self) -> Box<dyn LexRule<K, E>> {
        Box::new(self.clone())
    }
}
