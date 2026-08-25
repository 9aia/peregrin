use crate::{DefaultLexError, Span};
use std::fmt;

/// A token kind — the type of a lexed token.
///
/// Each dialect defines its own enum implementing this trait. This allows
/// compile-time type safety and pattern matching on token kinds without
/// a closed central enum.
///
/// Only `EofFactory` is required; the other category factories are opt-in
/// and are brought into scope by the rules that need them.
///
/// # Implementing
///
/// ```
/// use handl_lexer::{TokenKind, EofFactory};
/// use std::fmt;
///
/// #[derive(Debug, Clone, PartialEq)]
/// enum MyKind {
///     Identifier(String),
///     Number(f64),
///     Eof,
/// }
///
/// impl fmt::Display for MyKind {
///     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
///         match self {
///             MyKind::Identifier(s) => write!(f, "identifier({s})"),
///             MyKind::Number(n) => write!(f, "number({n})"),
///             MyKind::Eof => write!(f, "eof"),
///         }
///     }
/// }
///
/// #[derive(Clone)]
/// struct MyFactory;
///
/// impl EofFactory<MyKind> for MyFactory {
///     fn eof_kind(&self) -> MyKind { MyKind::Eof }
/// }
///
/// impl TokenKind for MyKind {
///     type Factory = MyFactory;
///     fn is_eof(&self) -> bool { matches!(self, MyKind::Eof) }
///     fn is_error(&self) -> bool { false }
/// }
/// ```
pub trait TokenKind: Clone + PartialEq + fmt::Debug + fmt::Display + Send + Sync + 'static {
    /// The factory type that constructs this token kind from raw lexed content.
    ///
    /// Only the end-of-input factory is required; rules requiring other
    /// factories add those bounds on `K::Factory` themselves.
    type Factory: EofFactory<Self>;

    /// Returns `true` if this is the end-of-input sentinel token.
    fn is_eof(&self) -> bool;

    /// Returns `true` if this is a recoverable error token.
    fn is_error(&self) -> bool;
}

/// A token kind that can represent comments.
///
/// Dialects with comment tokens implement this trait. Rules that lex comments
/// (e.g. `LineCommentRule`) require the token kind to implement this.
pub trait CommentTokenKind: TokenKind {
    /// Returns `true` if this is a comment token (to be filtered by the lexer).
    fn is_comment(&self) -> bool;
}

/// Factory for constructing end-of-input token kinds.
pub trait EofFactory<K: TokenKind>: Clone + Send + Sync + 'static {
    /// Construct the end-of-input sentinel token kind.
    fn eof_kind(&self) -> K;
}

/// Factory for constructing and lexing string-literal token kinds.
pub trait StringFactory<K: TokenKind>: Clone + Send + Sync + 'static {
    /// Construct a string-literal token kind from the unescaped value.
    fn string_kind(&self, value: String) -> K;

    /// Returns `true` if `ch` may open a string literal.
    ///
    /// The default implementation recognizes `"` and `'`. Dialects can
    /// override this to support different quote characters.
    fn is_string_start(&self, ch: char) -> bool {
        crate::primitives::is_string_start(ch)
    }

    /// Lex a string literal from the current position.
    ///
    /// The default implementation uses the canonical `"..."` / `'...'` syntax
    /// with backslash escapes. Dialects can override this to support different
    /// quote characters or escape rules.
    fn lex_string(
        &self,
        chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
    ) -> Result<(String, Span), DefaultLexError> {
        crate::primitives::lex_string(chars)
    }
}

/// Factory for constructing and lexing number-literal token kinds.
pub trait NumberFactory<K: TokenKind>: Clone + Send + Sync + 'static {
    /// Construct a number-literal token kind from the parsed value.
    fn number_kind(&self, value: f64) -> K;

    /// Returns `true` if `ch` may start a number literal.
    ///
    /// The default implementation recognizes ASCII digits and a leading `-`.
    fn is_number_start(&self, ch: char) -> bool {
        crate::primitives::is_number_start(ch)
    }

    /// Lex a number literal from the current position.
    ///
    /// The default implementation accepts integers, decimals, and a leading `-`
    /// sign. Dialects can override this to support different numeric literal
    /// syntax.
    fn lex_number(
        &self,
        chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
    ) -> Result<(f64, Span), DefaultLexError> {
        crate::primitives::lex_number(chars)
    }
}

/// Factory for constructing and lexing identifier token kinds.
pub trait IdentFactory<K: TokenKind>: Clone + Send + Sync + 'static {
    /// Construct an identifier token kind from the raw text.
    fn ident_kind(&self, text: String) -> K;

    /// Returns `true` if `ch` may start an identifier.
    ///
    /// The default implementation allows alphabetic characters, `_`, and `.`.
    fn is_ident_start(&self, ch: char) -> bool {
        crate::primitives::is_ident_start(ch)
    }

    /// Returns `true` if `ch` may follow the first character of an identifier.
    ///
    /// The default implementation allows alphanumeric characters, `_`, and `.`.
    fn is_ident_continue(&self, ch: char) -> bool {
        crate::primitives::is_ident_continue(ch)
    }

    /// Lex an identifier from the current position.
    ///
    /// The default implementation consumes characters matching
    /// [`Self::is_ident_continue`]. Dialects can override this for different
    /// identifier syntax (e.g. namespace separators, unicode identifiers).
    fn lex_ident(
        &self,
        chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
    ) -> Result<(String, Span), DefaultLexError> {
        let (start, _) = chars.peek().copied().unwrap();
        let mut text = String::new();
        let mut end = start;

        while let Some(&(pos, ch)) = chars.peek() {
            if !self.is_ident_continue(ch) {
                break;
            }
            text.push(ch);
            end = pos + ch.len_utf8();
            chars.next();
        }

        Ok((text, Span::new(start, end)))
    }
}

/// Factory for constructing keyword token kinds.
pub trait KeywordFactory<K: TokenKind>: Clone + Send + Sync + 'static {
    /// Construct a keyword token kind from the keyword text, if recognized.
    fn keyword_kind(&self, _text: &str) -> Option<K> {
        None
    }
}

/// Factory for constructing and lexing operator token kinds.
pub trait OperatorFactory<K: TokenKind>: Clone + Send + Sync + 'static {
    /// Construct an operator token kind from the operator text, if recognized.
    fn operator_kind(&self, _text: &str) -> Option<K> {
        None
    }

    /// The set of characters that may begin an operator.
    ///
    /// The default implementation matches the canonical single-character set.
    fn operator_chars(&self) -> &'static str {
        crate::primitives::DEFAULT_OPERATOR_CHARS
    }

    /// Lex an operator from the current position.
    ///
    /// The default implementation consumes a single operator character and
    /// returns the corresponding token kind, but only if [`Self::operator_kind`]
    /// recognizes it. Dialects can override this to support multi-character
    /// operators (`>=`, `!=`, etc.) while reusing the same rule.
    fn lex_operator(
        &self,
        chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
    ) -> Option<(K, Span)> {
        let &(pos, ch) = chars.peek()?;
        if !self.operator_chars().contains(ch) {
            return None;
        }
        let text = ch.to_string();
        let kind = self.operator_kind(&text)?;
        chars.next();
        Some((kind, Span::at(pos)))
    }
}

/// A convenience trait for factories that implement all common token-kind
/// constructors. Rules can instead require the smaller factory traits they
/// actually need, so dialects only implement the categories they support.
pub trait KindFactory<K: TokenKind>: Clone + Send + Sync + 'static {
    /// Construct a string-literal token kind from the unescaped value.
    fn string_kind(&self, value: String) -> K;

    /// Construct a number-literal token kind from the parsed value.
    fn number_kind(&self, value: f64) -> K;

    /// Construct an identifier token kind from the raw text.
    fn ident_kind(&self, text: String) -> K;

    /// Construct the end-of-input sentinel token kind.
    fn eof_kind(&self) -> K;

    /// Construct a keyword token kind from the keyword text, if recognized.
    fn keyword_kind(&self, _text: &str) -> Option<K> {
        None
    }

    /// Construct an operator token kind from the operator text, if recognized.
    fn operator_kind(&self, _text: &str) -> Option<K> {
        None
    }
}

impl<T, K> EofFactory<K> for T
where
    T: KindFactory<K>,
    K: TokenKind,
{
    fn eof_kind(&self) -> K {
        KindFactory::eof_kind(self)
    }
}

impl<T, K> StringFactory<K> for T
where
    T: KindFactory<K>,
    K: TokenKind,
{
    fn string_kind(&self, value: String) -> K {
        KindFactory::string_kind(self, value)
    }
}

impl<T, K> NumberFactory<K> for T
where
    T: KindFactory<K>,
    K: TokenKind,
{
    fn number_kind(&self, value: f64) -> K {
        KindFactory::number_kind(self, value)
    }
}

impl<T, K> IdentFactory<K> for T
where
    T: KindFactory<K>,
    K: TokenKind,
{
    fn ident_kind(&self, text: String) -> K {
        KindFactory::ident_kind(self, text)
    }
}

impl<T, K> KeywordFactory<K> for T
where
    T: KindFactory<K>,
    K: TokenKind,
{
    fn keyword_kind(&self, text: &str) -> Option<K> {
        KindFactory::keyword_kind(self, text)
    }
}

impl<T, K> OperatorFactory<K> for T
where
    T: KindFactory<K>,
    K: TokenKind,
{
    fn operator_kind(&self, text: &str) -> Option<K> {
        KindFactory::operator_kind(self, text)
    }
}

/// A factory for constructing and lexing comment token kinds.
pub trait CommentFactory<K: CommentTokenKind>: Clone + Send + Sync + 'static {
    /// Construct a comment token kind from the comment text.
    fn comment_kind(&self, text: String) -> K;

    /// Returns `true` if the iterator is positioned at the start of a line
    /// comment. The default implementation recognizes `//`.
    fn is_line_comment_start(
        &self,
        chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
    ) -> bool {
        crate::primitives::is_line_comment_start(chars)
    }

    /// Lex a line comment from the current position.
    ///
    /// The default implementation assumes `//` has already been consumed by the
    /// caller and reads to end of line. Dialects can override this to support
    /// different line-comment delimiters.
    fn lex_line_comment(
        &self,
        chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
    ) -> (String, Span) {
        crate::primitives::lex_line_comment(chars)
    }

    /// Returns `true` if the iterator is positioned at the start of a block
    /// comment. The default implementation recognizes `/*`.
    fn is_block_comment_start(
        &self,
        chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
    ) -> bool {
        crate::primitives::is_block_comment_start(chars)
    }

    /// Lex a block comment from the current position.
    ///
    /// The default implementation assumes `/*` has already been consumed by the
    /// caller and reads to the matching `*/`. Dialects can override this to
    /// support different block-comment delimiters or nesting rules.
    fn lex_block_comment(
        &self,
        chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
    ) -> Result<(String, Span), DefaultLexError> {
        crate::primitives::lex_block_comment(chars)
    }
}

/// A lexed token — its kind and source span.
#[derive(Debug, Clone, PartialEq)]
pub struct Token<K: TokenKind> {
    /// What kind of token this is.
    pub kind: K,

    /// Where this token appears in the source text.
    pub span: Span,
}

impl<K: TokenKind> Token<K> {
    /// Creates a token at the given span.
    pub fn new(kind: K, span: Span) -> Self {
        Token { kind, span }
    }
}
