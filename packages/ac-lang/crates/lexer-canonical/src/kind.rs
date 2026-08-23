//! Canonical token kind and factory.

use ac_lexer::{CommentFactory, CommentTokenKind, KindFactory, TokenKind};
use std::fmt;

/// The canonical AC expression token kinds.
///
/// This is the dialect-specific enum implementing [`TokenKind`] for the
/// canonical AC syntax. Other dialects can define their own enums and
/// reuse the generic lexing infrastructure from `ac-lexer`.
#[derive(Debug, Clone, PartialEq)]
pub enum CanonicalKind {
    /// An identifier — function name, variable name, or namespace path
    /// (e.g. `add`, `math.sqrt`, `x`).
    Identifier(String),

    /// A string literal (already unescaped, without quotes).
    String(String),

    /// A number literal.
    Number(f64),

    /// A boolean literal.
    Bool(bool),

    /// The null literal.
    Null,

    /// The `handle` keyword.
    Handle,

    /// The `as` keyword (used in handler clauses).
    As,

    /// Call-open delimiter — `(` in canonical.
    CallOpen,

    /// Call-close delimiter — `)` in canonical.
    CallClose,

    /// Block-open delimiter — `{` in canonical.
    BlockOpen,

    /// Block-close delimiter — `}` in canonical.
    BlockClose,

    /// Bracket-open delimiter — `[` in canonical.
    BracketOpen,

    /// Bracket-close delimiter — `]` in canonical.
    BracketClose,

    /// Argument separator — `,` in canonical.
    Comma,

    /// Clause separator — `:` in handler clauses.
    Colon,

    /// A comment — `// line` or `/* block */`. Filtered out of the
    /// token stream by the canonical lexer, but available for tooling
    /// (LSP, formatters) that wants to preserve comments.
    Comment(String),

    /// End of input.
    Eof}

impl fmt::Display for CanonicalKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CanonicalKind::Identifier(s) => write!(f, "identifier({s})"),
            CanonicalKind::String(s) => write!(f, "string({s:?})"),
            CanonicalKind::Number(n) => write!(f, "number({n})"),
            CanonicalKind::Bool(b) => write!(f, "bool({b})"),
            CanonicalKind::Null => write!(f, "null"),
            CanonicalKind::Handle => write!(f, "handle"),
            CanonicalKind::As => write!(f, "as"),
            CanonicalKind::CallOpen => write!(f, "("),
            CanonicalKind::CallClose => write!(f, ")"),
            CanonicalKind::BlockOpen => write!(f, "{{"),
            CanonicalKind::BlockClose => write!(f, "}}"),
            CanonicalKind::BracketOpen => write!(f, "["),
            CanonicalKind::BracketClose => write!(f, "]"),
            CanonicalKind::Comma => write!(f, ","),
            CanonicalKind::Colon => write!(f, ":"),
            CanonicalKind::Comment(s) => write!(f, "comment({s:?})"),
            CanonicalKind::Eof => write!(f, "eof")}
    }
}

impl TokenKind for CanonicalKind {
    type Factory = CanonicalFactory;

    fn is_eof(&self) -> bool {
        matches!(self, CanonicalKind::Eof)
    }

    fn is_error(&self) -> bool {
        false
    }
}

impl CommentTokenKind for CanonicalKind {
    fn is_comment(&self) -> bool {
        matches!(self, CanonicalKind::Comment(_))
    }
}

/// Factory for constructing [`CanonicalKind`] tokens from raw lexed content.
#[derive(Clone, Default)]
pub struct CanonicalFactory;

impl KindFactory<CanonicalKind> for CanonicalFactory {
    fn string_kind(&self, value: String) -> CanonicalKind {
        CanonicalKind::String(value)
    }

    fn number_kind(&self, value: f64) -> CanonicalKind {
        CanonicalKind::Number(value)
    }

    fn ident_kind(&self, text: String) -> CanonicalKind {
        CanonicalKind::Identifier(text)
    }

    fn eof_kind(&self) -> CanonicalKind {
        CanonicalKind::Eof
    }
}

impl CommentFactory<CanonicalKind> for CanonicalFactory {
    fn comment_kind(&self, text: String) -> CanonicalKind {
        CanonicalKind::Comment(text)
    }
}
