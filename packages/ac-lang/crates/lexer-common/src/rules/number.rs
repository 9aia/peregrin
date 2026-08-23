//! [`NumberRule`] — matches numbers including negative numbers.

use ac_lexer::{DefaultLexError, LexError, LexerConfig, NumberFactory, Token, TokenKind};
use ac_lexer::LexRule;

/// Matches number literals using the factory's [`NumberFactory::is_number_start`]
/// and [`NumberFactory::lex_number`].
#[derive(Clone, Default)]
pub struct NumberRule;

impl<K, E> LexRule<K, E> for NumberRule
where
    K: TokenKind,
    K::Factory: NumberFactory<K>,
    E: LexError + From<DefaultLexError>,
{
    fn try_lex(
        &self,
        chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
        config: &LexerConfig<K, E>,
    ) -> Option<Result<Token<K>, E>> {
        match chars.peek() {
            Some(&(_, ch)) if config.factory().is_number_start(ch) => {
                if ch == '-' {
                    let mut peek = chars.clone();
                    peek.next();
                    if !matches!(peek.peek(), Some((_, c)) if c.is_ascii_digit()) {
                        return None;
                    }
                }
                Some(
                    config.factory().lex_number(chars)
                        .map(|(value, span)| {
                            Token::new(config.factory().number_kind(value), span)
                        })
                        .map_err(E::from)
                )
            }
            _ => None,
        }
    }

    fn name(&self) -> &'static str {
        "number"
    }

    fn clone_box(&self) -> Box<dyn LexRule<K, E>> {
        Box::new(self.clone())
    }
}
