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
        s.find('(')
    }

    fn split_args(&self, s: &str) -> Vec<String> {
        s.split(',').map(|a| a.trim().to_string()).collect()
    }

    fn parse_string_literal(&self, s: &str) -> Option<String> {
        let trimmed = s.trim();
        if trimmed.len() >= 2 && trimmed.starts_with('"') && trimmed.ends_with('"') {
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
