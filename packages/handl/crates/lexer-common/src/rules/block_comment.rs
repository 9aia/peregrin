//! [`BlockCommentRule`] — matches block comments.

use handl_lexer::LexRule;
use handl_lexer::{CommentFactory, CommentTokenKind, DefaultLexError, LexError, LexerConfig, Token};

/// Matches block comments using the factory's [`CommentFactory::is_block_comment_start`]
/// and [`CommentFactory::lex_block_comment`].
#[derive(Clone, Default)]
pub struct BlockCommentRule;

impl<K, E> LexRule<K, E> for BlockCommentRule
where
    K: CommentTokenKind,
    K::Factory: CommentFactory<K>,
    E: LexError + From<DefaultLexError>,
{
    fn try_lex(
        &self,
        chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
        config: &LexerConfig<K, E>,
    ) -> Option<Result<Token<K>, E>> {
        if !config.factory().is_block_comment_start(chars) {
            return None;
        }

        Some(
            config
                .factory()
                .lex_block_comment(chars)
                .map(|(text, span)| Token::new(config.factory().comment_kind(text), span))
                .map_err(E::from),
        )
    }

    fn name(&self) -> &'static str {
        "block_comment"
    }

    fn clone_box(&self) -> Box<dyn LexRule<K, E>> {
        Box::new(self.clone())
    }
}
