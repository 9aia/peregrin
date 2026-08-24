//! [`MockLexer`] — a mock [`Lexer`] that returns predefined results.

use super::MockKind;
use ac_lexer::{DefaultLexError, LexError, Lexer, Token};

/// A mock lexer that returns a predefined token list or error.
pub struct MockLexer<E: LexError = DefaultLexError> {
    tokens: Result<Vec<Token<MockKind>>, E>,
}

impl<E: LexError> MockLexer<E> {
    pub fn ok(tokens: Vec<Token<MockKind>>) -> Self {
        MockLexer { tokens: Ok(tokens) }
    }

    pub fn err(e: E) -> Self {
        MockLexer { tokens: Err(e) }
    }
}

impl<E: LexError> Lexer<MockKind, E> for MockLexer<E> {
    fn lex(&self, _input: &str) -> Result<Vec<Token<MockKind>>, E> {
        self.tokens.clone()
    }
}