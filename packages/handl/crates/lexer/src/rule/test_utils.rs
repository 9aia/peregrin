//! Shared test utilities for handl-lexer internal tests.
//!
//! Defines a minimal local mock kind + factory (independent of handl-lexer-common)
//! to avoid circular dev-dependency.

use crate::{
    CommentFactory, CommentTokenKind, DefaultLexError, KindFactory, LexError, LexRule, Lexer,
    LexerConfig, Span, Token, TokenKind,
};
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum TestKind {
    Ident(String),
    Num(f64),
    Str(String),
    Bool(bool),
    Null,
    Comma,
    Colon,
    Comment(String),
    Delim(char),
    Custom(String),
    Eof,
}

impl fmt::Display for TestKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TestKind::Ident(s) => write!(f, "ident({s})"),
            TestKind::Num(n) => write!(f, "num({n})"),
            TestKind::Str(s) => write!(f, "str({s:?})"),
            TestKind::Bool(b) => write!(f, "bool({b})"),
            TestKind::Null => write!(f, "null"),
            TestKind::Comma => write!(f, ","),
            TestKind::Colon => write!(f, ":"),
            TestKind::Comment(s) => write!(f, "comment({s:?})"),
            TestKind::Delim(c) => write!(f, "delim({c})"),
            TestKind::Custom(s) => write!(f, "custom({s})"),
            TestKind::Eof => write!(f, "eof"),
        }
    }
}

impl TokenKind for TestKind {
    type Factory = TestFactory;
    fn is_eof(&self) -> bool {
        matches!(self, TestKind::Eof)
    }
    fn is_error(&self) -> bool {
        false
    }
}

impl CommentTokenKind for TestKind {
    fn is_comment(&self) -> bool {
        matches!(self, TestKind::Comment(_))
    }
}

#[derive(Clone, Default)]
pub struct TestFactory;

impl KindFactory<TestKind> for TestFactory {
    fn string_kind(&self, v: String) -> TestKind {
        TestKind::Str(v)
    }
    fn number_kind(&self, v: f64) -> TestKind {
        TestKind::Num(v)
    }
    fn ident_kind(&self, t: String) -> TestKind {
        TestKind::Ident(t)
    }
    fn eof_kind(&self) -> TestKind {
        TestKind::Eof
    }
}

impl CommentFactory<TestKind> for TestFactory {
    fn comment_kind(&self, t: String) -> TestKind {
        TestKind::Comment(t)
    }
}

pub fn cfg() -> LexerConfig<TestKind, DefaultLexError> {
    LexerConfig::new(TestFactory)
}

// --- Local mock rules for config_test ---

#[derive(Clone)]
pub struct BangRule;

impl<E: LexError> LexRule<TestKind, E> for BangRule {
    fn try_lex(
        &self,
        chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
        _config: &LexerConfig<TestKind, E>,
    ) -> Option<Result<Token<TestKind>, E>> {
        match chars.peek() {
            Some(&(_, '!')) => {
                let (pos, _) = chars.next().unwrap();
                Some(Ok(Token::new(TestKind::Null, Span::at(pos))))
            }
            _ => None,
        }
    }

    fn name(&self) -> &'static str {
        "bang"
    }

    fn clone_box(&self) -> Box<dyn LexRule<TestKind, E>> {
        Box::new(self.clone())
    }
}

// --- Local mock lexer ---

pub struct MockLexer<E: LexError = DefaultLexError> {
    tokens: Result<Vec<Token<TestKind>>, E>,
}

impl<E: LexError> MockLexer<E> {
    pub fn ok(tokens: Vec<Token<TestKind>>) -> Self {
        MockLexer { tokens: Ok(tokens) }
    }

    pub fn err(e: E) -> Self {
        MockLexer { tokens: Err(e) }
    }
}

impl<E: LexError> Lexer<TestKind, E> for MockLexer<E> {
    fn lex(&self, _input: &str) -> Result<Vec<Token<TestKind>>, E> {
        self.tokens.clone()
    }
}
