use ac_lexer::Span;
use thiserror::Error;

/// A parsing error — the AST could not be constructed.
///
/// Carries a span and message for diagnostic reporting. The span
/// may be zero-length for errors at a specific position (e.g.
/// unexpected end of input).
#[derive(Debug, Clone, PartialEq, Error)]
pub enum ParseError {
    /// An unexpected token was encountered.
    #[error("unexpected token at {span}: {expected}")]
    UnexpectedToken { span: Span, expected: String },

    /// An unexpected end of input.
    #[error("unexpected end of input at {span}: {expected}")]
    UnexpectedEof { span: Span, expected: String },

    /// A lexing error occurred during `parse_text`.
    #[error("lexing error: {message}")]
    LexError { message: String },
}