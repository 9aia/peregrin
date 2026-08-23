//! [`DelimiterRule`] — matches single-character delimiters from config.

use ac_lexer::LexRule;
use ac_lexer::{LexError, LexerConfig, Span, Token, TokenKind};

/// Matches single-character delimiters from the config's delimiter set.
#[derive(Clone, Default)]
pub struct DelimiterRule;

impl<K, E> LexRule<K, E> for DelimiterRule
where
    K: TokenKind,
    E: LexError,
{
    fn try_lex(
        &self,
        chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
        config: &LexerConfig<K, E>,
    ) -> Option<Result<Token<K>, E>> {
        match chars.peek() {
            Some(&(pos, ch)) => {
                if let Some(kind) = config.lookup_delimiter(ch) {
                    chars.next();
                    Some(Ok(Token::new(kind.clone(), Span::at(pos))))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn name(&self) -> &'static str {
        "delimiter"
    }

    fn clone_box(&self) -> Box<dyn LexRule<K, E>> {
        Box::new(self.clone())
    }
}
