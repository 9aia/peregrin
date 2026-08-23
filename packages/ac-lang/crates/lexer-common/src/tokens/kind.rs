//! [`MockKind`] — a comprehensive test token kind enum.

use ac_lexer::{CommentTokenKind, TokenKind};
use std::fmt;

/// A mock token kind covering all common lexer token categories.
#[derive(Debug, Clone, PartialEq)]
pub enum MockKind {
    Ident(String),
    Keyword(String),
    Num(f64),
    Str(String),
    Bool(bool),
    Null,
    Comma,
    Colon,
    Comment(String),
    Delim(char),
    Operator(String),
    Custom(String),
    Eof,
}

impl fmt::Display for MockKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MockKind::Ident(s) => write!(f, "ident({s})"),
            MockKind::Keyword(s) => write!(f, "keyword({s})"),
            MockKind::Num(n) => write!(f, "num({n})"),
            MockKind::Str(s) => write!(f, "str({s:?})"),
            MockKind::Bool(b) => write!(f, "bool({b})"),
            MockKind::Null => write!(f, "null"),
            MockKind::Comma => write!(f, ","),
            MockKind::Colon => write!(f, ":"),
            MockKind::Comment(s) => write!(f, "comment({s:?})"),
            MockKind::Delim(c) => write!(f, "delim({c})"),
            MockKind::Operator(s) => write!(f, "operator({s})"),
            MockKind::Custom(s) => write!(f, "custom({s})"),
            MockKind::Eof => write!(f, "eof"),
        }
    }
}

impl TokenKind for MockKind {
    type Factory = super::MockFactory;

    fn is_eof(&self) -> bool {
        matches!(self, MockKind::Eof)
    }

    fn is_error(&self) -> bool {
        false
    }
}

impl CommentTokenKind for MockKind {
    fn is_comment(&self) -> bool {
        matches!(self, MockKind::Comment(_))
    }
}
