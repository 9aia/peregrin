//! # ac-dialect
//!
//! Syntax dialect abstraction for the AC language implementation.
//!
//! The AC language specification defines one canonical syntax. This crate
//! abstracts the three syntax decisions — interpolation markers, path
//! separator, and call syntax — behind a [`Dialect`] trait so the
//! implementation is structured around a single [`AcDialect`], with the
//! architecture allowing others if needed.
//!
//! ## Architectural rule
//!
//! > **The language spec fixes syntax. The implementation abstracts it.**
//!
//! The spec says `${...}` for interpolation, `.` for paths, `fn()` for calls.
//! The `Dialect` trait lets the Rust implementation be parameterized without
//! changing the spec. `AcDialect` is the only shipped implementation.

/// A syntax dialect for parsing and evaluating AC expressions.
///
/// The AC language specification defines one canonical syntax.
/// This trait abstracts the syntax decisions so the implementation
/// can be structured around a single [`AcDialect`], with the
/// architecture allowing others if needed.
pub trait Dialect: Send + Sync {
    /// The interpolation prefix — e.g. `"${"` for the canonical dialect.
    fn interpolation_prefix(&self) -> &str;

    /// The interpolation suffix — e.g. `"}"` for the canonical dialect.
    fn interpolation_suffix(&self) -> &str;

    /// The path separator — e.g. `'.'` for the canonical dialect.
    fn path_separator(&self) -> char;

    /// The call-open character — e.g. `'('` for the canonical dialect.
    fn call_open(&self) -> char;

    /// The call-close character — e.g. `')'` for the canonical dialect.
    fn call_close(&self) -> char;

    /// Detect whether `s` contains a function call opening (e.g. `(` in canonical).
    /// Returns the byte position of the call-open token if present and not
    /// inside a string literal.
    fn find_call_open(&self, s: &str) -> Option<usize>;

    /// Split function arguments by the argument separator (`,` in canonical),
    /// respecting nested call-open/close tokens and string literals.
    fn split_args(&self, s: &str) -> Vec<String>;

    /// Check if `s` is a string literal and return its inner content.
    fn parse_string_literal(&self, s: &str) -> Option<String>;

    /// Check if `s` is a number literal and return its value.
    fn parse_number_literal(&self, s: &str) -> Option<f64>;
}

/// The canonical AC language dialect.
///
/// Implements the syntax defined in the AC Language Specification:
/// - Interpolation: `${...}`
/// - Path separator: `.`
/// - Function calls: `fn(args)`
/// - String literals: `"..."` or `'...'`
/// - Number literals: `42`, `1.5`, `-3`
/// - Boolean literals: `true`, `false`
/// - Null literal: `null`
pub struct AcDialect;

impl Dialect for AcDialect {
    fn interpolation_prefix(&self) -> &str {
        "${"
    }

    fn interpolation_suffix(&self) -> &str {
        "}"
    }

    fn path_separator(&self) -> char {
        '.'
    }

    fn call_open(&self) -> char {
        '('
    }

    fn call_close(&self) -> char {
        ')'
    }

    fn find_call_open(&self, s: &str) -> Option<usize> {
        let mut in_string = false;
        let mut quote_char = '\0';
        for (i, ch) in s.char_indices() {
            if in_string {
                if ch == quote_char {
                    in_string = false;
                }
            } else if ch == '"' || ch == '\'' {
                in_string = true;
                quote_char = ch;
            } else if ch == '(' {
                return Some(i);
            }
        }
        None
    }

    fn split_args(&self, s: &str) -> Vec<String> {
        let mut result = Vec::new();
        let mut current = String::new();
        let mut depth = 0i32;
        let mut in_string = false;
        let mut quote_char = '\0';

        for ch in s.chars() {
            if in_string {
                current.push(ch);
                if ch == quote_char {
                    in_string = false;
                }
            } else if ch == '"' || ch == '\'' {
                in_string = true;
                quote_char = ch;
                current.push(ch);
            } else if ch == '(' {
                depth += 1;
                current.push(ch);
            } else if ch == ')' {
                depth -= 1;
                current.push(ch);
            } else if ch == ',' && depth == 0 {
                result.push(current.clone());
                current.clear();
            } else {
                current.push(ch);
            }
        }
        if !current.trim().is_empty() {
            result.push(current);
        }
        result
    }

    fn parse_string_literal(&self, s: &str) -> Option<String> {
        let s = s.trim();
        if (s.starts_with('"') && s.ends_with('"') && s.len() >= 2)
            || (s.starts_with('\'') && s.ends_with('\'') && s.len() >= 2)
        {
            Some(s[1..s.len() - 1].to_string())
        } else {
            None
        }
    }

    fn parse_number_literal(&self, s: &str) -> Option<f64> {
        s.trim().parse::<f64>().ok()
    }
}

/// The canonical AC dialect as a static singleton.
pub static AC_DIALECT: AcDialect = AcDialect;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interpolation_markers() {
        let d = AcDialect;
        assert_eq!(d.interpolation_prefix(), "${");
        assert_eq!(d.interpolation_suffix(), "}");
    }

    #[test]
    fn path_separator() {
        let d = AcDialect;
        assert_eq!(d.path_separator(), '.');
    }

    #[test]
    fn call_markers() {
        let d = AcDialect;
        assert_eq!(d.call_open(), '(');
        assert_eq!(d.call_close(), ')');
    }

    #[test]
    fn find_call_open_basic() {
        let d = AcDialect;
        assert_eq!(d.find_call_open("add(1, 2)"), Some(3));
        assert_eq!(d.find_call_open("doc.name"), None);
    }

    #[test]
    fn find_call_open_in_string() {
        let d = AcDialect;
        assert_eq!(d.find_call_open("\"hello(world\""), None);
    }

    #[test]
    fn split_args_basic() {
        let d = AcDialect;
        let args = d.split_args("1, 2, 3");
        assert_eq!(args, vec!["1", " 2", " 3"]);
    }

    #[test]
    fn split_args_nested() {
        let d = AcDialect;
        let args = d.split_args("add(1, 2), 3");
        assert_eq!(args.len(), 2);
    }

    #[test]
    fn parse_string_literal_double() {
        let d = AcDialect;
        assert_eq!(d.parse_string_literal("\"hello\""), Some("hello".to_string()));
    }

    #[test]
    fn parse_string_literal_single() {
        let d = AcDialect;
        assert_eq!(d.parse_string_literal("'hello'"), Some("hello".to_string()));
    }

    #[test]
    fn parse_number_literal() {
        let d = AcDialect;
        assert_eq!(d.parse_number_literal("42"), Some(42.0));
        assert_eq!(d.parse_number_literal("1.5"), Some(1.5));
        assert_eq!(d.parse_number_literal("-3"), Some(-3.0));
        assert_eq!(d.parse_number_literal("not a number"), None);
    }
}
