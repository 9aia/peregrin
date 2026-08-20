//! # ac-dialect-canonical
//!
//! The canonical AC language dialect implementation.
//!
//! Implements the [`Dialect`] trait with the syntax defined in the AC
//! Language Specification:
//! - Interpolation: `${...}`
//! - Path separator: `.`
//! - Function calls: `fn(args)`
//! - String literals: `"..."` or `'...'`
//! - Number literals: `42`, `1.5`, `-3`
//! - Boolean literals: `true`, `false`
//! - Null literal: `null`
//! - Schema vocabulary: JSON Schema draft 2020-12
//!
//! This is the only shipped dialect. See [DIALECT.md](../../docs/DIALECT.md)
//! for the full dialect specification.

use ac_dialect::Dialect;
use ac_runtime::RuntimeValue;
use ac_schema::SchemaVocabulary;

/// The canonical AC language dialect.
///
/// Implements the syntax defined in the AC Language Specification.
/// See the crate-level documentation for the full list of canonical values.
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
    fn parse_bool_literal() {
        let d = AcDialect;
        assert_eq!(d.parse_bool_literal("true"), Some(true));
        assert_eq!(d.parse_bool_literal("false"), Some(false));
        assert_eq!(d.parse_bool_literal("yes"), None);
        assert_eq!(d.parse_bool_literal("null"), None);
    }

    #[test]
    fn is_null_literal() {
        let d = AcDialect;
        assert!(d.is_null_literal("null"));
        assert!(!d.is_null_literal("none"));
        assert!(!d.is_null_literal("true"));
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
