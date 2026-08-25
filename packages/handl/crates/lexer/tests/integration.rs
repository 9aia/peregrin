//! Integration tests for the generic lexer infrastructure.
//!
//! These tests exercise the public API of `handl-lexer` from an external
//! perspective, defining a custom `TokenKind` implementation and verifying
//! that the generic config, rules, and lexer trait work end-to-end.

use handl_lexer::{
    CommentFactory, CommentTokenKind, DefaultLexError, FnRule, IdentFactory, KindFactory, LexError,
    LexRule, Lexer, LexerConfig, NumberFactory, Span, StringFactory, Token, TokenKind,
};
use std::fmt;

// --- Test dialect ---

#[derive(Debug, Clone, PartialEq)]
enum IntegKind {
    Ident(String),
    Num(f64),
    Str(String),
    Comment(String),
    Eof,
}

impl fmt::Display for IntegKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IntegKind::Ident(s) => write!(f, "ident({s})"),
            IntegKind::Num(n) => write!(f, "num({n})"),
            IntegKind::Str(s) => write!(f, "str({s:?})"),
            IntegKind::Comment(s) => write!(f, "comment({s:?})"),
            IntegKind::Eof => write!(f, "eof"),
        }
    }
}

impl TokenKind for IntegKind {
    type Factory = IntegFactory;
    fn is_eof(&self) -> bool {
        matches!(self, IntegKind::Eof)
    }
    fn is_error(&self) -> bool {
        false
    }
}

impl CommentTokenKind for IntegKind {
    fn is_comment(&self) -> bool {
        matches!(self, IntegKind::Comment(_))
    }
}

#[derive(Clone, Default)]
struct IntegFactory;

impl KindFactory<IntegKind> for IntegFactory {
    fn string_kind(&self, v: String) -> IntegKind {
        IntegKind::Str(v)
    }
    fn number_kind(&self, v: f64) -> IntegKind {
        IntegKind::Num(v)
    }
    fn ident_kind(&self, t: String) -> IntegKind {
        IntegKind::Ident(t)
    }
    fn eof_kind(&self) -> IntegKind {
        IntegKind::Eof
    }
}

impl CommentFactory<IntegKind> for IntegFactory {
    fn comment_kind(&self, t: String) -> IntegKind {
        IntegKind::Comment(t)
    }
}

// --- Local rules (since built-in rules live in handl-lexer-common) ---

use handl_lexer::primitives::{is_ident_start, lex_ident, lex_number, lex_string};

#[derive(Clone, Default)]
struct StringRule;

impl<K, E> LexRule<K, E> for StringRule
where
    K: TokenKind,
    K::Factory: StringFactory<K>,
    E: LexError + From<DefaultLexError>,
{
    fn try_lex(
        &self,
        chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
        config: &LexerConfig<K, E>,
    ) -> Option<Result<Token<K>, E>> {
        match chars.peek() {
            Some(&(_, '"' | '\'')) => Some(
                lex_string(chars)
                    .map(|(value, span)| Token::new(config.factory().string_kind(value), span))
                    .map_err(E::from),
            ),
            _ => None,
        }
    }
    fn name(&self) -> &'static str {
        "string"
    }
    fn clone_box(&self) -> Box<dyn LexRule<K, E>> {
        Box::new(self.clone())
    }
}

#[derive(Clone, Default)]
struct NumberRule;

impl<K, E> LexRule<K, E> for NumberRule
where
    K: TokenKind,
    K::Factory: NumberFactory<K>,
    E: LexError + From<DefaultLexError>,
{
    fn try_lex(
        &self,
        chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
        config: &LexerConfig<K, E>,
    ) -> Option<Result<Token<K>, E>> {
        match chars.peek() {
            Some(&(_, '0'..='9')) => Some(
                lex_number(chars)
                    .map(|(value, span)| Token::new(config.factory().number_kind(value), span))
                    .map_err(E::from),
            ),
            Some(&(_, '-')) => {
                let mut peek = chars.clone();
                peek.next();
                if matches!(peek.peek(), Some((_, c)) if c.is_ascii_digit()) {
                    Some(
                        lex_number(chars)
                            .map(|(value, span)| {
                                Token::new(config.factory().number_kind(value), span)
                            })
                            .map_err(E::from),
                    )
                } else {
                    None
                }
            }
            _ => None,
        }
    }
    fn name(&self) -> &'static str {
        "number"
    }
    fn clone_box(&self) -> Box<dyn LexRule<K, E>> {
        Box::new(self.clone())
    }
}

#[derive(Clone, Default)]
struct IdentRule;

impl<K, E> LexRule<K, E> for IdentRule
where
    K: TokenKind,
    K::Factory: IdentFactory<K>,
    E: LexError + From<DefaultLexError>,
{
    fn try_lex(
        &self,
        chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
        config: &LexerConfig<K, E>,
    ) -> Option<Result<Token<K>, E>> {
        match chars.peek() {
            Some(&(_, ch)) if is_ident_start(ch) => Some(
                lex_ident(chars)
                    .map(|(text, span)| {
                        let kind = config
                            .lookup_keyword(&text)
                            .cloned()
                            .unwrap_or_else(|| config.factory().ident_kind(text));
                        Token::new(kind, span)
                    })
                    .map_err(E::from),
            ),
            _ => None,
        }
    }
    fn name(&self) -> &'static str {
        "ident"
    }
    fn clone_box(&self) -> Box<dyn LexRule<K, E>> {
        Box::new(self.clone())
    }
}

// --- A simple lexer using the generic infrastructure ---

struct SimpleLexer {
    config: LexerConfig<IntegKind, DefaultLexError>,
}

impl Default for SimpleLexer {
    fn default() -> Self {
        SimpleLexer {
            config: LexerConfig::new(IntegFactory)
                .with_rule(StringRule)
                .with_rule(NumberRule)
                .with_rule(IdentRule),
        }
    }
}

impl Lexer<IntegKind, DefaultLexError> for SimpleLexer {
    fn lex(&self, input: &str) -> Result<Vec<Token<IntegKind>>, DefaultLexError> {
        let mut tokens = Vec::new();
        let mut chars = input.char_indices().peekable();

        while let Some(&(pos, ch)) = chars.peek() {
            if ch.is_whitespace() {
                chars.next();
                continue;
            }

            let mut matched = false;
            for rule in self.config.rules() {
                if let Some(result) = rule.try_lex(&mut chars, &self.config) {
                    let token = result?;
                    if !token.kind.is_comment() {
                        tokens.push(token);
                    }
                    matched = true;
                    break;
                }
            }

            if !matched {
                chars.next();
                return Err(DefaultLexError::UnexpectedChar {
                    span: Span::at(pos),
                    char: ch,
                });
            }
        }

        tokens.push(Token::new(
            self.config.factory().eof_kind(),
            Span::at(input.len()),
        ));
        Ok(tokens)
    }
}

// --- Tests ---

#[test]
fn lex_basic_tokens() {
    let lexer = SimpleLexer::default();
    let tokens = lexer.lex("foo 42 \"hello\"").unwrap();
    assert_eq!(tokens.len(), 4);
    assert_eq!(tokens[0].kind, IntegKind::Ident("foo".into()));
    assert_eq!(tokens[1].kind, IntegKind::Num(42.0));
    assert_eq!(tokens[2].kind, IntegKind::Str("hello".into()));
    assert_eq!(tokens[3].kind, IntegKind::Eof);
}

#[test]
fn lex_unterminated_string_errors() {
    let lexer = SimpleLexer::default();
    let result = lexer.lex("\"oops");
    assert!(result.is_err());
}

#[test]
fn lex_unexpected_char_errors() {
    let lexer = SimpleLexer::default();
    let result = lexer.lex("@@@");
    assert!(result.is_err());
}

#[test]
fn lex_iter_streams_tokens() {
    let lexer = SimpleLexer::default();
    let tokens: Vec<_> = lexer.lex_iter("foo 42").collect();
    assert_eq!(tokens.len(), 3);
    assert!(tokens.iter().all(|t| t.is_ok()));
}

#[test]
fn custom_fn_rule_works() {
    fn bang_rule(
        chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
        _config: &LexerConfig<IntegKind, DefaultLexError>,
    ) -> Option<Result<Token<IntegKind>, DefaultLexError>> {
        match chars.peek() {
            Some(&(_, '!')) => {
                let (pos, _) = chars.next().unwrap();
                Some(Ok(Token::new(IntegKind::Ident("!".into()), Span::at(pos))))
            }
            _ => None,
        }
    }

    let config = LexerConfig::new(IntegFactory)
        .with_rule(FnRule::new("bang", bang_rule))
        .with_rule(StringRule)
        .with_rule(NumberRule)
        .with_rule(IdentRule);

    let lexer = SimpleLexer { config };
    let tokens = lexer.lex("! 42").unwrap();
    assert_eq!(tokens[0].kind, IntegKind::Ident("!".into()));
    assert_eq!(tokens[1].kind, IntegKind::Num(42.0));
}

#[test]
fn config_merge_combines_rules() {
    let base: LexerConfig<IntegKind, DefaultLexError> = LexerConfig::new(IntegFactory)
        .with_rule(StringRule)
        .with_rule(NumberRule);
    let ext = LexerConfig::new(IntegFactory).with_rule(IdentRule);

    let merged = base.merge(ext);
    assert_eq!(merged.rules().len(), 3);
    assert_eq!(merged.rules()[0].name(), "string");
    assert_eq!(merged.rules()[1].name(), "number");
    assert_eq!(merged.rules()[2].name(), "ident");
}

#[test]
fn config_merge_dedups_by_name() {
    let base: LexerConfig<IntegKind, DefaultLexError> =
        LexerConfig::new(IntegFactory).with_rule(StringRule);
    let ext = LexerConfig::new(IntegFactory).with_rule(StringRule);

    let merged = base.merge(ext);
    assert_eq!(merged.rules().len(), 1);
}

#[test]
fn without_rule_by_name_disables_feature() {
    let config = LexerConfig::new(IntegFactory)
        .with_rule(StringRule)
        .with_rule(NumberRule)
        .with_rule(IdentRule)
        .without_rule_by_name("string");

    let lexer = SimpleLexer { config };
    let result = lexer.lex("\"hello\"");
    assert!(result.is_err());
}
