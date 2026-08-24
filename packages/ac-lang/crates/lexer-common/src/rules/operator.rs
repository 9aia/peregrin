//! [`OperatorRule`] — matches operators.

use ac_lexer::LexRule;
use ac_lexer::{LexError, LexerConfig, OperatorFactory, Token, TokenKind};

/// Matches operators using the factory's [`OperatorFactory::lex_operator`].
#[derive(Clone, Default)]
pub struct OperatorRule;

impl<K, E> LexRule<K, E> for OperatorRule
where
    K: TokenKind,
    K::Factory: OperatorFactory<K>,
    E: LexError,
{
    fn try_lex(
        &self,
        chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
        config: &LexerConfig<K, E>,
    ) -> Option<Result<Token<K>, E>> {
        config
            .factory()
            .lex_operator(chars)
            .map(|(kind, span)| Ok(Token::new(kind, span)))
    }

    fn name(&self) -> &'static str {
        "operator"
    }

    fn clone_box(&self) -> Box<dyn LexRule<K, E>> {
        Box::new(self.clone())
    }
}
