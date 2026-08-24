//! Canonical token kind and factory.

use ac_lexer::{
    CommentFactory, CommentTokenKind, EofFactory, IdentFactory, NumberFactory, StringFactory,
    TokenKind,
};
use std::fmt;

/// The canonical AC expression token kinds.
///
/// This is the dialect-specific enum implementing [`TokenKind`] for the
/// canonical AC syntax. Other dialects can define their own enums and
/// reuse the generic lexing infrastructure from `ac-lexer`.
#[derive(Debug, Clone, PartialEq)]
pub enum CanonicalKind {
    /// An identifier — function name or variable name
    /// (e.g. `add`, `x`). Dots are separate `Dot` tokens.
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

    /// The `is` keyword (interrogative mood).
    Is,

    /// The `must` keyword (normative mood).
    Must,

    /// The `may` keyword (potential mood).
    May,

    /// The `not` keyword (negative mood or logical negation).
    Not,

    /// The `when` keyword (conditional mood).
    When,

    /// The `else` keyword (conditional mood fallback).
    Else,

    /// The `match` keyword (pattern matching expression).
    Match,

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

    /// Arrow — `->` (first-class function body introducer).
    Arrow,

    /// Pipe forward — `|>` (left-to-right value threading).
    Pipe,

    /// Pipe backward — `<|` (right-to-left value threading).
    BackwardPipe,

    /// Flow / forward composition — `>>>` (arrow combinator).
    Flow,

    /// Composition / backward composition — `<<<` (arrow combinator).
    Compose,

    /// Fanout — `&&&` (arrow combinator).
    Fanout,

    /// Parallel / fanin — `||||` (arrow combinator).
    Parallel,

    /// Ternary / optional — `?`.
    Question,

    /// Range — `..`.
    Range,

    /// Spread — `...`.
    Spread,

    /// Less-than — `<` (generic parameter list opener).
    LessThan,

    /// Greater-than — `>` (generic parameter list closer).
    GreaterThan,

    /// Equal — `=` (assignment).
    Equal,

    /// Dot — `.` (field access).
    Dot,

    /// A comment — `// line` or `/* block */`. Filtered out of the
    /// token stream by the canonical lexer, but available for tooling
    /// (LSP, formatters) that wants to preserve comments.
    Comment(String),

    /// End of input.
    Eof,
}

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
            CanonicalKind::Is => write!(f, "is"),
            CanonicalKind::Must => write!(f, "must"),
            CanonicalKind::May => write!(f, "may"),
            CanonicalKind::Not => write!(f, "not"),
            CanonicalKind::When => write!(f, "when"),
            CanonicalKind::Else => write!(f, "else"),
            CanonicalKind::Match => write!(f, "match"),
            CanonicalKind::CallOpen => write!(f, "("),
            CanonicalKind::CallClose => write!(f, ")"),
            CanonicalKind::BlockOpen => write!(f, "{{"),
            CanonicalKind::BlockClose => write!(f, "}}"),
            CanonicalKind::BracketOpen => write!(f, "["),
            CanonicalKind::BracketClose => write!(f, "]"),
            CanonicalKind::Comma => write!(f, ","),
            CanonicalKind::Colon => write!(f, ":"),
            CanonicalKind::Arrow => write!(f, "->"),
            CanonicalKind::Pipe => write!(f, "|>"),
            CanonicalKind::BackwardPipe => write!(f, "<|"),
            CanonicalKind::Flow => write!(f, ">>>"),
            CanonicalKind::Compose => write!(f, "<<<"),
            CanonicalKind::Fanout => write!(f, "&&&"),
            CanonicalKind::Parallel => write!(f, "||||"),
            CanonicalKind::Question => write!(f, "?"),
            CanonicalKind::Range => write!(f, ".."),
            CanonicalKind::Spread => write!(f, "..."),
            CanonicalKind::LessThan => write!(f, "<"),
            CanonicalKind::GreaterThan => write!(f, ">"),
            CanonicalKind::Equal => write!(f, "="),
            CanonicalKind::Dot => write!(f, "."),
            CanonicalKind::Comment(s) => write!(f, "comment({s:?})"),
            CanonicalKind::Eof => write!(f, "eof"),
        }
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

impl EofFactory<CanonicalKind> for CanonicalFactory {
    fn eof_kind(&self) -> CanonicalKind {
        CanonicalKind::Eof
    }
}

impl StringFactory<CanonicalKind> for CanonicalFactory {
    fn string_kind(&self, value: String) -> CanonicalKind {
        CanonicalKind::String(value)
    }
}

impl NumberFactory<CanonicalKind> for CanonicalFactory {
    fn number_kind(&self, value: f64) -> CanonicalKind {
        CanonicalKind::Number(value)
    }
}

impl IdentFactory<CanonicalKind> for CanonicalFactory {
    fn ident_kind(&self, text: String) -> CanonicalKind {
        CanonicalKind::Identifier(text)
    }

    fn is_ident_start(&self, ch: char) -> bool {
        ch.is_alphabetic() || ch == '_'
    }

    fn is_ident_continue(&self, ch: char) -> bool {
        ch.is_alphanumeric() || ch == '_'
    }
}

impl CommentFactory<CanonicalKind> for CanonicalFactory {
    fn comment_kind(&self, text: String) -> CanonicalKind {
        CanonicalKind::Comment(text)
    }
}
