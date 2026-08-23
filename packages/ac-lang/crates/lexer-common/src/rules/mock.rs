//! [`RuleMock`] — helpers for creating test rules with trait impls.

use ac_lexer::{LexError, LexRule, LexerConfig, Span, Token};
use crate::tokens::MockKind;

/// Helpers for creating test rules and rule-related test fixtures.
pub struct RuleMock;

/// A stateful rule that matches `!` and produces a [`MockKind::Null`] token.
#[derive(Clone)]
pub struct BangRule;

/// A simple rule that matches `#` and produces a [`MockKind::Custom`] token.
#[derive(Clone)]
pub struct HashRule;

/// A simple rule that matches `@` and produces a [`MockKind::Custom`] token.
#[derive(Clone)]
pub struct AtRule;

impl RuleMock {
    /// Creates a rule that matches `!` and produces a [`MockKind::Null`] token.
    pub fn bang() -> BangRule {
        BangRule
    }

    /// Creates a rule that matches `#` and produces a [`MockKind::Custom`] token.
    pub fn hash() -> HashRule {
        HashRule
    }

    /// Creates a rule that matches `@` and produces a [`MockKind::Custom`] token.
    pub fn at() -> AtRule {
        AtRule
    }
}

impl<E: LexError> LexRule<MockKind, E> for BangRule {
    fn try_lex(
        &self,
        chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
        _config: &LexerConfig<MockKind, E>,
    ) -> Option<Result<Token<MockKind>, E>> {
        match chars.peek() {
            Some(&(_, '!')) => {
                let (pos, _) = chars.next().unwrap();
                Some(Ok(Token::new(MockKind::Null, Span::at(pos))))
            }
            _ => None}
    }

    fn name(&self) -> &'static str {
        "bang"
    }

    fn clone_box(&self) -> Box<dyn LexRule<MockKind, E>> {
        Box::new(self.clone())
    }
}

impl<E: LexError> LexRule<MockKind, E> for HashRule {
    fn try_lex(
        &self,
        chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
        _config: &LexerConfig<MockKind, E>,
    ) -> Option<Result<Token<MockKind>, E>> {
        match chars.peek() {
            Some(&(_, '#')) => {
                let (start, _) = chars.next().unwrap();
                Some(Ok(Token::new(
                    MockKind::Custom("hash".into()),
                    Span::at(start),
                )))
            }
            _ => None}
    }

    fn name(&self) -> &'static str {
        "hash"
    }

    fn clone_box(&self) -> Box<dyn LexRule<MockKind, E>> {
        Box::new(self.clone())
    }
}

impl<E: LexError> LexRule<MockKind, E> for AtRule {
    fn try_lex(
        &self,
        chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
        _config: &LexerConfig<MockKind, E>,
    ) -> Option<Result<Token<MockKind>, E>> {
        match chars.peek() {
            Some(&(_, '@')) => {
                let (start, _) = chars.next().unwrap();
                Some(Ok(Token::new(
                    MockKind::Custom("at".into()),
                    Span::at(start),
                )))
            }
            _ => None}
    }

    fn name(&self) -> &'static str {
        "at"
    }

    fn clone_box(&self) -> Box<dyn LexRule<MockKind, E>> {
        Box::new(self.clone())
    }
}
