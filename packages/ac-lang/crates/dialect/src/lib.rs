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

use ac_schema::SchemaVocabulary;
use ac_runtime::RuntimeValue;

/// A syntax dialect for parsing and evaluating AC expressions.
///
/// The AC language specification defines one canonical syntax.
/// This trait abstracts the syntax decisions so the implementation
/// can be structured around a single [`AcDialect`], with the
/// architecture allowing others if needed.
///
/// In addition to expression syntax, the dialect defines the
/// AC-specific key names used in documents (`fields`, `functions`,
/// `expr`, `arg`, `merge`, `inheritable`, `mutable`, `params`,
/// `return`, `body`) and the schema vocabulary used for validation.
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

    // --- AC-specific key names ---

    /// The top-level key for field declarations. Canonical: `"fields"`.
    fn fields_key(&self) -> &str {
        "fields"
    }

    /// The top-level key for function declarations. Canonical: `"functions"`.
    fn functions_key(&self) -> &str {
        "functions"
    }

    /// The field-level key for computed field expressions. Canonical: `"expr"`.
    fn expr_key(&self) -> &str {
        "expr"
    }

    /// The field-level key for parameterized computed field type. Canonical: `"arg"`.
    fn arg_key(&self) -> &str {
        "arg"
    }

    /// The field-level key for merge field flag. Canonical: `"merge"`.
    fn merge_key(&self) -> &str {
        "merge"
    }

    /// The field-level key for scoped inheritance flag. Canonical: `"inheritable"`.
    fn inheritable_key(&self) -> &str {
        "inheritable"
    }

    /// The field-level key for immutability flag. Canonical: `"mutable"`.
    fn mutable_key(&self) -> &str {
        "mutable"
    }

    /// The function-level key for parameters. Canonical: `"params"`.
    fn params_key(&self) -> &str {
        "params"
    }

    /// The function-level key for return schema. Canonical: `"return"`.
    fn return_key(&self) -> &str {
        "return"
    }

    /// The function-level key for user-defined function body. Canonical: `"body"`.
    fn body_key(&self) -> &str {
        "body"
    }

    /// All AC-specific field-level keys (used for stripping during typegen).
    fn ac_field_keys(&self) -> Vec<&str> {
        vec![
            self.expr_key(),
            self.arg_key(),
            self.merge_key(),
            self.inheritable_key(),
            self.mutable_key(),
        ]
    }

    /// Reserved top-level keys that cannot be declared as field names.
    fn reserved_keys(&self) -> Vec<&str> {
        vec![self.fields_key(), self.functions_key()]
    }

    // --- Schema vocabulary ---

    /// The schema vocabulary used for field and function validation.
    ///
    /// The canonical dialect returns a JSON Schema draft 2020-12 vocabulary.
    fn schema_vocabulary(&self) -> &dyn SchemaVocabulary<RuntimeValue>;
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
/// - Schema vocabulary: JSON Schema draft 2020-12
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

    fn schema_vocabulary(&self) -> &dyn SchemaVocabulary<RuntimeValue> {
        &ac_schema_json::JSON_SCHEMA_VOCABULARY
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

    #[test]
    fn key_names() {
        let d = AcDialect;
        assert_eq!(d.fields_key(), "fields");
        assert_eq!(d.functions_key(), "functions");
        assert_eq!(d.expr_key(), "expr");
        assert_eq!(d.arg_key(), "arg");
        assert_eq!(d.merge_key(), "merge");
        assert_eq!(d.inheritable_key(), "inheritable");
        assert_eq!(d.mutable_key(), "mutable");
        assert_eq!(d.params_key(), "params");
        assert_eq!(d.return_key(), "return");
        assert_eq!(d.body_key(), "body");
    }

    #[test]
    fn reserved_keys() {
        let d = AcDialect;
        let reserved = d.reserved_keys();
        assert!(reserved.contains(&"fields"));
        assert!(reserved.contains(&"functions"));
    }

    #[test]
    fn ac_field_keys() {
        let d = AcDialect;
        let keys = d.ac_field_keys();
        assert!(keys.contains(&"expr"));
        assert!(keys.contains(&"arg"));
        assert!(keys.contains(&"merge"));
        assert!(keys.contains(&"inheritable"));
        assert!(keys.contains(&"mutable"));
    }

    #[test]
    fn schema_vocabulary_id() {
        let d = AcDialect;
        assert_eq!(d.schema_vocabulary().id(), "json-schema-draft-2020-12");
    }
}
