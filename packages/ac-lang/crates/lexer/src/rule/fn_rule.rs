//! [`FnRule`] — a rule backed by a function pointer.

use crate::{LexError, LexerConfig, Token, TokenKind};
use crate::rule::LexRule;

/// A rule backed by a function pointer.
///
/// Use this when you need a simple rule without defining a struct.
/// For stateful or composable rules, implement [`LexRule`] directly.
///
/// ```
/// use ac_lexer::{DefaultLexError, EofFactory, FnRule, LexerConfig, TokenKind, LexError, Span, Token};
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
/// fn hash_rule(
///     chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
///     _config: &LexerConfig<MyKind, DefaultLexError>,
/// ) -> Option<Result<Token<MyKind>, DefaultLexError>> {
///     match chars.peek() {
///         Some(&(_, '#')) => {
///             let (start, _) = chars.next().unwrap();
///             Some(Ok(Token::new(MyKind::Hash("hash".into()), Span::at(start))))
///         }
///         _ => None,
///     }
/// }
///
/// let config = LexerConfig::new(MyFactory)
///     .with_rule(FnRule::new("hash", hash_rule));
/// ```
pub struct FnRule<K: TokenKind, E: LexError> {
    name: &'static str,
    func: fn(
        &mut std::iter::Peekable<std::str::CharIndices<'_>>,
        &LexerConfig<K, E>,
    ) -> Option<Result<Token<K>, E>>}

impl<K: TokenKind, E: LexError> FnRule<K, E> {
    /// Creates a named function-pointer rule.
    pub fn new(
        name: &'static str,
        func: fn(
            &mut std::iter::Peekable<std::str::CharIndices<'_>>,
            &LexerConfig<K, E>,
        ) -> Option<Result<Token<K>, E>>,
    ) -> Self {
        FnRule { name, func }
    }
}

impl<K: TokenKind, E: LexError> Clone for FnRule<K, E> {
    fn clone(&self) -> Self {
        FnRule {
            name: self.name,
            func: self.func}
    }
}

impl<K: TokenKind, E: LexError> LexRule<K, E> for FnRule<K, E> {
    fn try_lex(
        &self,
        chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
        config: &LexerConfig<K, E>,
    ) -> Option<Result<Token<K>, E>> {
        (self.func)(chars, config)
    }

    fn name(&self) -> &'static str {
        self.name
    }

    fn clone_box(&self) -> Box<dyn LexRule<K, E>> {
        Box::new(self.clone())
    }
}
