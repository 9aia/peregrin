//! # ac-dialect
//!
//! Dialect abstraction for the AC language implementation.
//!
//! The AC language specification defines one canonical syntax. This crate
//! abstracts the syntax decisions, special key names, and schema vocabulary
//! behind a [`Dialect`] trait so the implementation can be structured around
//! a single source of truth.
//!
//! ## Architectural rule
//!
//! > **The language spec fixes syntax. The implementation abstracts it.**
//!
//! The spec says `${...}` for interpolation, `.` for paths, `fn()` for calls.
//! The `Dialect` trait lets the Rust implementation be parameterized without
//! changing the spec. The canonical implementation lives in `ac-dialect-canonical`.

use ac_schema::SchemaVocabulary;
use ac_runtime::RuntimeValue;

/// A dialect for parsing and evaluating AC expressions and documents.
///
/// The AC language specification defines one canonical syntax.
/// This trait abstracts the syntax decisions so the implementation
/// can be structured around a single canonical dialect, with the
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

    /// Check if `s` is a boolean literal and return its value.
    /// Canonical: `"true"` → `true`, `"false"` → `false`.
    fn parse_bool_literal(&self, s: &str) -> Option<bool> {
        match s.trim() {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        }
    }

    /// Check if `s` is the null literal.
    /// Canonical: `"null"`.
    fn is_null_literal(&self, s: &str) -> bool {
        s.trim() == "null"
    }

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

    /// All field-level reserved keys (used for stripping during typegen).
    fn reserved_field_keys(&self) -> Vec<&str> {
        vec![
            self.expr_key(),
            self.arg_key(),
            self.merge_key(),
            self.inheritable_key(),
            self.mutable_key(),
        ]
    }

    /// Reserved top-level keys that cannot be declared as field names.
    fn top_level_reserved_keys(&self) -> Vec<&str> {
        vec![self.fields_key(), self.functions_key()]
    }

    // --- Schema vocabulary ---

    /// The schema vocabulary used for field and function validation.
    ///
    /// The canonical dialect returns a JSON Schema draft 2020-12 vocabulary.
    fn schema_vocabulary(&self) -> &dyn SchemaVocabulary<RuntimeValue>;
}
