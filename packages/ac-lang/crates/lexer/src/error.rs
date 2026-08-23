use crate::Span;
use std::fmt;
use thiserror::Error;

/// Trait for unrecoverable lexing errors.
///
/// Dialects can use the built-in [`DefaultLexError`] or define their own
/// error type. Rule implementations that use the default primitives need
/// `E: From<DefaultLexError>` so the primitive errors can be converted.
pub trait LexError:
    Clone + fmt::Debug + fmt::Display + std::error::Error + Send + Sync + 'static
{
}

impl<T> LexError for T where
    T: Clone + fmt::Debug + fmt::Display + std::error::Error + Send + Sync + 'static
{
}

/// The default, concrete lexing error type.
///
/// Recoverable errors produce `TokenKind::Error` tokens instead.
/// This type is for errors where no meaningful token stream can
/// be produced (e.g. unterminated string literal).
#[derive(Debug, Clone, PartialEq, Error)]
pub enum DefaultLexError {
    /// An unterminated string literal starting at the given span.
    #[error("unterminated string literal at {span}")]
    UnterminatedString { span: Span },

    /// An invalid number literal at the given span.
    #[error("invalid number literal at {span}")]
    InvalidNumber { span: Span },

    /// An unexpected character at the given span.
    #[error("unexpected character at {span}: {char}")]
    UnexpectedChar { span: Span, char: char },
}
