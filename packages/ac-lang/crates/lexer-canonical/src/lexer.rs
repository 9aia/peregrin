//! Canonical lexer implementation and streaming iterator.

use crate::config::canonical_config;
use crate::kind::CanonicalKind;
use ac_lexer::{CommentTokenKind, DefaultLexError, EofFactory, Lexer, LexerConfig, Span, Token};

/// The canonical AC expression lexer.
///
/// Stateless — safe to reuse across inputs. Uses [`canonical_config()`]
/// (the canonical AC syntax) unless configured otherwise.
#[derive(Clone)]
pub struct CanonicalLexer {
    config: LexerConfig<CanonicalKind, DefaultLexError>,
}

impl Default for CanonicalLexer {
    fn default() -> Self {
        CanonicalLexer {
            config: canonical_config(),
        }
    }
}

impl CanonicalLexer {
    /// Creates a lexer with a custom configuration.
    pub fn with_config(config: LexerConfig<CanonicalKind, DefaultLexError>) -> Self {
        CanonicalLexer { config }
    }
}

impl Lexer<CanonicalKind, DefaultLexError> for CanonicalLexer {
    fn lex(&self, input: &str) -> Result<Vec<Token<CanonicalKind>>, DefaultLexError> {
        let mut tokens = Vec::with_capacity(input.len() / 4 + 1);
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

    fn lex_iter<'a>(
        &'a self,
        input: &'a str,
    ) -> Box<dyn Iterator<Item = Result<Token<CanonicalKind>, DefaultLexError>> + 'a> {
        Box::new(TokenIter {
            chars: input.char_indices().peekable(),
            config: &self.config,
            input_len: input.len(),
            done: false,
        })
    }
}

/// Streaming token iterator backed by the canonical lexing engine.
///
/// Yields `Result<Token<CanonicalKind>, DefaultLexError>` one at a time. Always
/// emits a final EOF token before returning `None`. On error, yields `Err` and
/// stops.
pub struct TokenIter<'a> {
    chars: std::iter::Peekable<std::str::CharIndices<'a>>,
    config: &'a LexerConfig<CanonicalKind, DefaultLexError>,
    input_len: usize,
    done: bool,
}

impl<'a> Iterator for TokenIter<'a> {
    type Item = Result<Token<CanonicalKind>, DefaultLexError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        loop {
            match self.chars.peek() {
                None => {
                    self.done = true;
                    return Some(Ok(Token::new(
                        self.config.factory().eof_kind(),
                        Span::at(self.input_len),
                    )));
                }
                Some(&(_, ch)) if ch.is_whitespace() => {
                    self.chars.next();
                }
                Some(&(pos, ch)) => {
                    let mut comment_skipped = false;
                    for rule in self.config.rules() {
                        if let Some(result) = rule.try_lex(&mut self.chars, self.config) {
                            match result {
                                Ok(t) if t.kind.is_comment() => {
                                    comment_skipped = true;
                                    break;
                                }
                                Ok(t) => return Some(Ok(t)),
                                Err(e) => {
                                    self.done = true;
                                    return Some(Err(e));
                                }
                            }
                        }
                    }

                    if comment_skipped {
                        continue;
                    }

                    self.chars.next();
                    self.done = true;
                    return Some(Err(DefaultLexError::UnexpectedChar {
                        span: Span::at(pos),
                        char: ch,
                    }));
                }
            }
        }
    }
}
