//! The [`Lexer`] trait — converts expression text into a token stream.

use crate::{LexError, Token, TokenKind};

/// A lexer that converts expression text into a token stream.
///
/// The batch method [`lex`](Lexer::lex) is the primary interface and is
/// object-safe (usable via `&dyn Lexer`). The streaming method
/// [`lex_iter`](Lexer::lex_iter) returns tokens one at a time via
/// `Box<dyn Iterator>` — it has a default implementation that collects
/// from `lex`, but concrete implementations can override it with a true
/// streaming iterator.
///
/// The lexer does not build structure — structural decisions (nesting,
/// precedence, argument grouping) are the parser's job.
///
/// Dialect-specific behavior (which characters open calls, what
/// keywords exist, literal syntax) is configured by the implementing
/// type, typically using a [`LexerConfig`](crate::LexerConfig) preset
/// from a dialect crate.
pub trait Lexer<K: TokenKind, E: LexError>: Send + Sync {
    /// Lex `input` into a flat list of tokens.
    ///
    /// Returns `Err(E)` on unrecoverable input (e.g. unterminated
    /// string literal). Implementations may also emit error tokens
    /// for recoverable errors, allowing the parser to attempt
    /// recovery with diagnostic context.
    fn lex(&self, input: &str) -> Result<Vec<Token<K>>, E>;

    /// Lex `input` into a streaming iterator of tokens.
    ///
    /// Each call to `next` yields one `Result<Token<K>, E>`. The
    /// iterator always emits a final EOF token before exhausting.
    /// On unrecoverable input (e.g. unterminated string), an `Err` is
    /// yielded and the iterator stops.
    ///
    /// The default implementation calls [`lex`](Self::lex) and wraps the
    /// result in an iterator — not truly streaming. Concrete
    /// implementations should override this for real incremental
    /// tokenization.
    fn lex_iter<'a>(
        &'a self,
        input: &'a str,
    ) -> Box<dyn Iterator<Item = Result<Token<K>, E>> + 'a> {
        match self.lex(input) {
            Ok(tokens) => Box::new(tokens.into_iter().map(Ok)),
            Err(e) => Box::new(std::iter::once(Err(e)))}
    }
}
