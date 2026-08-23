//! [`LineCommentRule`] — matches line comments.

use ac_lexer::{CommentFactory, CommentTokenKind, LexError, LexerConfig, Token};
use ac_lexer::LexRule;

/// Matches line comments using the factory's [`CommentFactory::is_line_comment_start`]
/// and [`CommentFactory::lex_line_comment`].
#[derive(Clone, Default)]
pub struct LineCommentRule;

impl<K, E> LexRule<K, E> for LineCommentRule
where
    K: CommentTokenKind,
    K::Factory: CommentFactory<K>,
    E: LexError,
{
    fn try_lex(
        &self,
        chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
        config: &LexerConfig<K, E>,
    ) -> Option<Result<Token<K>, E>> {
        if !config.factory().is_line_comment_start(chars) {
            return None;
        }

        let (text, span) = config.factory().lex_line_comment(chars);
        Some(Ok(Token::new(
            config.factory().comment_kind(text),
            span,
        )))
    }

    fn name(&self) -> &'static str {
        "line_comment"
    }

    fn clone_box(&self) -> Box<dyn LexRule<K, E>> {
        Box::new(self.clone())
    }
}
