//! [`KeywordRule`] — matches identifiers the factory recognizes as keywords.

use ac_lexer::LexRule;
use ac_lexer::{IdentFactory, KeywordFactory, LexError, LexerConfig, Token, TokenKind};

/// Matches identifiers that the factory's [`KeywordFactory::keyword_kind`] maps
/// to a keyword kind. Non-keyword identifiers are left for other rules.
#[derive(Clone, Default)]
pub struct KeywordRule;

impl<K, E> LexRule<K, E> for KeywordRule
where
    K: TokenKind,
    K::Factory: KeywordFactory<K> + IdentFactory<K>,
    E: LexError,
{
    fn try_lex(
        &self,
        chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
        config: &LexerConfig<K, E>,
    ) -> Option<Result<Token<K>, E>> {
        match chars.peek() {
            Some(&(_, ch)) if config.factory().is_ident_start(ch) => {
                let mut peek = chars.clone();
                let (text, span) = config.factory().lex_ident(&mut peek).ok()?;
                config.factory().keyword_kind(&text).map(|kind| {
                    *chars = peek;
                    Ok(Token::new(kind, span))
                })
            }
            _ => None,
        }
    }

    fn name(&self) -> &'static str {
        "keyword"
    }

    fn clone_box(&self) -> Box<dyn LexRule<K, E>> {
        Box::new(self.clone())
    }
}
