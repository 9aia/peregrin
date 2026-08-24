use ac_lexer::Span;
use thiserror::Error;

/// An extensible parser error.
///
/// Dialects can implement this trait with their own error type and pass it
/// as `E` to `Parser`, `Grammar`, and `Rule`. The common engine calls the
/// constructor methods when it encounters unexpected tokens or EOF.
pub trait ParseError: std::error::Error + std::fmt::Debug + Clone + Send + Sync + 'static {
    fn unexpected_eof<S: Into<String>>(span: Span, expected: S) -> Self;
    fn unexpected_token<S: Into<String>>(span: Span, expected: S) -> Self;
    fn message<S: Into<String>>(span: Span, message: S) -> Self;
}

#[derive(Debug, Clone, Error)]
pub enum DefaultParseError {
    #[error("unexpected end of input at {span}: expected {expected}")]
    UnexpectedEof { span: Span, expected: String },

    #[error("unexpected token at {span}: expected {expected}")]
    UnexpectedToken { span: Span, expected: String },

    #[error("parse error at {span}: {message}")]
    Message { span: Span, message: String },
}

impl ParseError for DefaultParseError {
    fn unexpected_eof<S: Into<String>>(span: Span, expected: S) -> Self {
        Self::UnexpectedEof {
            span,
            expected: expected.into(),
        }
    }

    fn unexpected_token<S: Into<String>>(span: Span, expected: S) -> Self {
        Self::UnexpectedToken {
            span,
            expected: expected.into(),
        }
    }

    fn message<S: Into<String>>(span: Span, message: S) -> Self {
        Self::Message {
            span,
            message: message.into(),
        }
    }
}