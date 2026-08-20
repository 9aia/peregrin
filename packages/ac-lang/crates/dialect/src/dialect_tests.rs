use ac_runtime::RuntimeValue;

use super::dialect::Dialect;
use super::schema::{Schema, SchemaVocabulary, ValidationError};

/// A minimal schema vocabulary for testing.
struct StubVocabulary;

impl SchemaVocabulary<RuntimeValue> for StubVocabulary {
    fn id(&self) -> &'static str {
        "stub"
    }

    fn compile(
        &self,
        _raw: &serde_json::Value,
    ) -> Result<Box<dyn Schema<RuntimeValue>>, ValidationError> {
        Err(ValidationError {
            message: "not implemented".to_string(),
            path: String::new(),
            location: None,
        })
    }
}

/// A minimal dialect implementation for testing default methods.
struct StubDialect;

impl Dialect for StubDialect {
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
        let mut in_string: Option<char> = None;
        for (i, c) in s.char_indices() {
            match in_string {
                Some(quote) => {
                    if c == quote {
                        in_string = None;
                    }
                }
                None => {
                    if c == '"' || c == '\'' {
                        in_string = Some(c);
                    } else if c == '(' {
                        return Some(i);
                    }
                }
            }
        }
        None
    }

    fn split_args(&self, s: &str) -> Vec<String> {
        let mut args = Vec::new();
        let mut current = String::new();
        let mut depth = 0i32;
        let mut in_string: Option<char> = None;

        for c in s.chars() {
            match in_string {
                Some(quote) => {
                    current.push(c);
                    if c == quote {
                        in_string = None;
                    }
                }
                None => {
                    if c == '"' || c == '\'' {
                        in_string = Some(c);
                        current.push(c);
                    } else if c == '(' {
                        depth += 1;
                        current.push(c);
                    } else if c == ')' {
                        depth -= 1;
                        current.push(c);
                    } else if c == ',' && depth == 0 {
                        args.push(current.trim().to_string());
                        current.clear();
                    } else {
                        current.push(c);
                    }
                }
            }
        }

        let trimmed = current.trim();
        if !trimmed.is_empty() || !args.is_empty() {
            args.push(trimmed.to_string());
        }
        args
    }

    fn parse_string_literal(&self, s: &str) -> Option<String> {
        let trimmed = s.trim();
        if trimmed.len() >= 2 && (trimmed.starts_with('"') && trimmed.ends_with('"')) {
            Some(trimmed[1..trimmed.len() - 1].to_string())
        } else if trimmed.len() >= 2 && (trimmed.starts_with('\'') && trimmed.ends_with('\'')) {
            Some(trimmed[1..trimmed.len() - 1].to_string())
        } else {
            None
        }
    }

    fn parse_number_literal(&self, s: &str) -> Option<f64> {
        s.trim().parse().ok()
    }

    fn schema_vocabulary(&self) -> &dyn SchemaVocabulary<RuntimeValue> {
        &StubVocabulary
    }
}

#[test]
fn find_call_open_simple() {
    let d = StubDialect;
    assert_eq!(d.find_call_open("add(1, 2)"), Some(3));
}

#[test]
fn find_call_open_none() {
    let d = StubDialect;
    assert_eq!(d.find_call_open("doc.name"), None);
}

#[test]
fn find_call_open_inside_string() {
    let d = StubDialect;
    assert_eq!(d.find_call_open(r#"concat("not (here)", x)"#), Some(6));
}

#[test]
fn find_call_open_single_quoted_string() {
    let d = StubDialect;
    assert_eq!(d.find_call_open("concat('no (paren)', x)"), Some(6));
}

#[test]
fn split_args_simple() {
    let d = StubDialect;
    assert_eq!(d.split_args("1, 2, 3"), vec!["1", "2", "3"]);
}

#[test]
fn split_args_nested_call() {
    let d = StubDialect;
    assert_eq!(d.split_args("add(1, 2), 3"), vec!["add(1, 2)", "3"]);
}

#[test]
fn split_args_string_with_comma() {
    let d = StubDialect;
    assert_eq!(d.split_args("\"a, b\", 3"), vec!["\"a, b\"", "3"]);
}

#[test]
fn split_args_single_quoted_with_comma() {
    let d = StubDialect;
    assert_eq!(d.split_args("'a, b', 3"), vec!["'a, b'", "3"]);
}

#[test]
fn parse_string_literal_double_quoted() {
    let d = StubDialect;
    assert_eq!(d.parse_string_literal("\"hello\""), Some("hello".to_string()));
}

#[test]
fn parse_string_literal_single_quoted() {
    let d = StubDialect;
    assert_eq!(d.parse_string_literal("'hello'"), Some("hello".to_string()));
}

#[test]
fn parse_string_literal_not_a_string() {
    let d = StubDialect;
    assert_eq!(d.parse_string_literal("hello"), None);
}

#[test]
fn parse_number_literal_integer() {
    let d = StubDialect;
    assert_eq!(d.parse_number_literal("42"), Some(42.0));
}

#[test]
fn parse_number_literal_float() {
    let d = StubDialect;
    assert_eq!(d.parse_number_literal("1.5"), Some(1.5));
}

#[test]
fn parse_number_literal_negative() {
    let d = StubDialect;
    assert_eq!(d.parse_number_literal("-3"), Some(-3.0));
}

#[test]
fn parse_bool_literal_true() {
    let d = StubDialect;
    assert_eq!(d.parse_bool_literal("true"), Some(true));
}

#[test]
fn parse_bool_literal_false() {
    let d = StubDialect;
    assert_eq!(d.parse_bool_literal("false"), Some(false));
}

#[test]
fn parse_bool_literal_invalid() {
    let d = StubDialect;
    assert_eq!(d.parse_bool_literal("yes"), None);
}

#[test]
fn is_null_literal_true() {
    let d = StubDialect;
    assert!(d.is_null_literal("null"));
}

#[test]
fn is_null_literal_false() {
    let d = StubDialect;
    assert!(!d.is_null_literal("none"));
}

#[test]
fn default_key_names() {
    let d = StubDialect;
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
fn reserved_field_keys_default() {
    let d = StubDialect;
    let keys = d.reserved_field_keys();
    assert_eq!(keys, vec!["expr", "arg", "merge", "inheritable", "mutable"]);
}

#[test]
fn top_level_reserved_keys_default() {
    let d = StubDialect;
    let keys = d.top_level_reserved_keys();
    assert_eq!(keys, vec!["fields", "functions"]);
}
