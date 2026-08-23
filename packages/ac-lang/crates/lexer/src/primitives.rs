//! Low-level tokenization primitives.
//!
//! Each function consumes characters from a `CharIndices` peekable
//! iterator and produces a raw value + span. These are reusable building
//! blocks for writing [`crate::LexRule`] functions — the rule functions
//! use a [`crate::KindFactory`] to convert raw values to dialect-specific
//! token kinds.

use crate::{DefaultLexError, Span};

pub fn is_ident_start(ch: char) -> bool {
    ch.is_alphabetic() || ch == '_' || ch == '.'
}

/// Returns `true` if `ch` may appear after the first character of an identifier.
pub fn is_ident_continue(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_' || ch == '.'
}

/// Lexes a `"..."` or `'...'` string literal with backslash escapes.
/// Returns the unescaped value and its span.
pub fn lex_string(
    chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
) -> Result<(String, Span), DefaultLexError> {
    let (start, quote) = chars.next().unwrap();
    let mut value = String::new();
    let mut end = start + 1;

    loop {
        match chars.next() {
            None => {
                return Err(DefaultLexError::UnterminatedString {
                    span: Span::new(start, end),
                });
            }
            Some((pos, ch)) if ch == quote => {
                end = pos + 1;
                break;
            }
            Some((_, '\\')) => {
                match chars.next() {
                    Some((_, 'n')) => value.push('\n'),
                    Some((_, 't')) => value.push('\t'),
                    Some((_, 'r')) => value.push('\r'),
                    Some((_, '\\')) => value.push('\\'),
                    Some((_, '"')) => value.push('"'),
                    Some((_, '\'')) => value.push('\''),
                    Some((pos, ch)) => {
                        value.push(ch);
                        let _ = pos;
                    }
                    None => {
                        return Err(DefaultLexError::UnterminatedString {
                            span: Span::new(start, end),
                        });
                    }
                }
            }
            Some((_, ch)) => {
                value.push(ch);
            }
        }
    }

    Ok((value, Span::new(start, end)))
}

/// Lexes a number literal (integer or decimal, optionally negative).
/// Returns the parsed f64 value and its span.
pub fn lex_number(
    chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
) -> Result<(f64, Span), DefaultLexError> {
    let (start, _) = chars.peek().copied().unwrap();
    let mut text = String::new();
    let mut end = start;

    if chars.peek().map(|(_, c)| *c) == Some('-') {
        let (pos, ch) = chars.next().unwrap();
        text.push(ch);
        end = pos + 1;
    }

    while let Some(&(pos, ch)) = chars.peek() {
        if ch.is_ascii_digit() || ch == '.' {
            text.push(ch);
            end = pos + ch.len_utf8();
            chars.next();
        } else {
            break;
        }
    }

    let value: f64 = text.parse().map_err(|_| DefaultLexError::InvalidNumber {
        span: Span::new(start, end),
    })?;

    Ok((value, Span::new(start, end)))
}

pub fn is_string_start(ch: char) -> bool {
    ch == '"' || ch == '\''
}

pub fn is_number_start(ch: char) -> bool {
    ch.is_ascii_digit() || ch == '-'
}

/// Returns `true` if the iterator is positioned at the start of a line comment.
pub fn is_line_comment_start(chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>) -> bool {
    let mut peek = chars.clone();
    matches!(peek.next(), Some((_, '/'))) && matches!(peek.peek(), Some((_, '/')))
}

/// Lexes a `// line comment` (assumes `is_line_comment_start` is true).
/// Returns the comment text and its span.
pub fn lex_line_comment(
    chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
) -> (String, Span) {
    let (start, _) = chars.peek().copied().unwrap();
    chars.next();
    chars.next();
    let mut text = String::new();
    let mut end = start + 2;

    while let Some(&(pos, ch)) = chars.peek() {
        if ch == '\n' {
            end = pos;
            break;
        }
        text.push(ch);
        end = pos + ch.len_utf8();
        chars.next();
    }

    (text, Span::new(start, end))
}

/// Returns `true` if the iterator is positioned at the start of a block comment.
pub fn is_block_comment_start(chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>) -> bool {
    let mut peek = chars.clone();
    matches!(peek.next(), Some((_, '/'))) && matches!(peek.peek(), Some((_, '*')))
}

/// Lexes a `/* block comment */` (assumes `is_block_comment_start` is true).
/// Returns the comment text and its span, or an error if unterminated.
pub fn lex_block_comment(
    chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
) -> Result<(String, Span), DefaultLexError> {
    let (start, _) = chars.peek().copied().unwrap();
    chars.next();
    chars.next();
    let mut text = String::new();
    let mut end = start + 2;

    loop {
        match chars.next() {
            None => {
                return Err(DefaultLexError::UnterminatedString {
                    span: Span::new(start, end),
                });
            }
            Some((pos, '*')) => {
                end = pos + 1;
                match chars.peek() {
                    Some(&(_, '/')) => {
                        chars.next();
                        end += 1;
                        break;
                    }
                    _ => text.push('*'),
                }
            }
            Some((pos, ch)) => {
                text.push(ch);
                end = pos + ch.len_utf8();
            }
        }
    }

    Ok((text, Span::new(start, end)))
}

/// Canonical single-character operator set.
pub const DEFAULT_OPERATOR_CHARS: &str = "+-*/=<>&|%!^~?";

/// Lexes an identifier (may include `.` for namespace paths).
/// Returns the raw identifier text and its span.
pub fn lex_ident(
    chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
) -> Result<(String, Span), DefaultLexError> {
    let (start, _) = chars.peek().copied().unwrap();
    let mut text = String::new();
    let mut end = start;

    while let Some(&(pos, ch)) = chars.peek() {
        if ch.is_alphanumeric() || ch == '_' || ch == '.' {
            text.push(ch);
            end = pos + ch.len_utf8();
            chars.next();
        } else {
            break;
        }
    }

    Ok((text, Span::new(start, end)))
}
