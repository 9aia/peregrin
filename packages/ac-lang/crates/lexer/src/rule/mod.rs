//! Lexing rules — composable, type-safe building blocks for tokenization.
//!
//! Each rule implements [`LexRule`] and tries to produce a token from the
//! current position in the input. Rules are tried in order by [`LexerConfig`];
//! the first that returns `Some` wins.
//!
//! Built-in rules (`StringRule`, `NumberRule`, etc.) live in `ac-lexer-common`.
//! Custom rules can be defined as structs with state, or wrapped from function
//! pointers using [`FnRule`].

use crate::{LexError, LexerConfig, Token, TokenKind};

mod fn_rule;

#[cfg(test)]
mod fn_rule_test;
#[cfg(test)]
pub(crate) mod test_utils;

pub use fn_rule::FnRule;

/// A lexing rule — inspects the current position and optionally produces a token.
///
/// Returns `None` if the rule does not match (try the next rule),
/// `Some(Ok(token))` on success, or `Some(Err(_))` on a matched-but-failed
/// lex (e.g. unterminated string).
///
/// # Implementing a custom rule
///
/// ```
/// use ac_lexer::{DefaultLexError, EofFactory, LexRule, LexerConfig, Span, Token, TokenKind};
/// use std::fmt;
///
/// #[derive(Debug, Clone, PartialEq)]
/// enum MyKind { Hash(String), Eof }
///
/// impl fmt::Display for MyKind {
///     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
///         match self { MyKind::Hash(s) => write!(f, "hash({s})"), MyKind::Eof => write!(f, "eof") }
///     }
/// }
///
/// #[derive(Clone)]
/// struct MyFactory;
/// impl EofFactory<MyKind> for MyFactory {
///     fn eof_kind(&self) -> MyKind { MyKind::Eof }
/// }
///
/// impl TokenKind for MyKind {
///     type Factory = MyFactory;
///     fn is_eof(&self) -> bool { matches!(self, MyKind::Eof) }
///     fn is_error(&self) -> bool { false }
/// }
///
/// // A stateful custom rule.
/// #[derive(Clone)]
/// struct HashRule;
///
/// impl LexRule<MyKind, DefaultLexError> for HashRule {
///     fn try_lex(
///         &self,
///         chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
///         _config: &LexerConfig<MyKind, DefaultLexError>,
///     ) -> Option<Result<Token<MyKind>, DefaultLexError>> {
///         match chars.peek() {
///             Some(&(_, '#')) => {
///                 let (start, _) = chars.next().unwrap();
///                 Some(Ok(Token::new(MyKind::Hash("hash".into()), Span::at(start))))
///             }
///             _ => None,
///         }
///     }
///
///     fn name(&self) -> &'static str { "hash" }
///
///     fn clone_box(&self) -> Box<dyn LexRule<MyKind, DefaultLexError>> {
///         Box::new(self.clone())
///     }
/// }
/// ```
pub trait LexRule<K: TokenKind, E: LexError>: Send + Sync + 'static {
    /// Try to lex a token from the current position.
    ///
    /// - `None` — this rule does not match; try the next rule.
    /// - `Some(Ok(token))` — success; the token is consumed.
    /// - `Some(Err(_))` — matched but failed (e.g. unterminated string).
    fn try_lex(
        &self,
        chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
        config: &LexerConfig<K, E>,
    ) -> Option<Result<Token<K>, E>>;

    /// A name for deduplication and debugging.
    ///
    /// Two rules with the same name are considered duplicates by
    /// [`LexerConfig::merge`] and [`LexerConfig::without_rule`].
    fn name(&self) -> &'static str;

    /// Clone into a boxed trait object.
    fn clone_box(&self) -> Box<dyn LexRule<K, E>>;
}
