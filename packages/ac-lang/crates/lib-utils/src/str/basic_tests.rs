use ac_runtime_values::NonFunctionValue;

use ac_runtime_values::{CanonicalContext, Error};

use ac_runtime_modules::Module;

use super::StrModule;

fn call(name: &str, args: &[NonFunctionValue]) -> NonFunctionValue {
    let func = StrModule.function(name).expect("function exists");
    let mut ctx = CanonicalContext::new();
    func.call(args, &mut ctx).expect("call succeeds")
}

fn call_err(name: &str, args: &[NonFunctionValue]) -> Result<NonFunctionValue, Error> {
    let func = StrModule.function(name).expect("function exists");
    let mut ctx = CanonicalContext::new();
    func.call(args, &mut ctx)
}

fn s(v: &str) -> NonFunctionValue {
    NonFunctionValue::new(v.to_string())
}

fn n(v: f64) -> NonFunctionValue {
    NonFunctionValue::new(serde_json::json!(v))
}

#[test]
fn module_name() {
    assert_eq!(StrModule.name(), "str");
}

// ── upper ───────────────────────────────────────────────────────────

#[test]
fn upper() {
    assert_eq!(call("upper", &[s("hello")]).as_str(), Some("HELLO"));
}

#[test]
fn upper_empty_string() {
    assert_eq!(call("upper", &[s("")]).as_str(), Some(""));
}

#[test]
fn upper_unicode() {
    assert_eq!(call("upper", &[s("héllo")]).as_str(), Some("HÉLLO"));
}

// ── lower ───────────────────────────────────────────────────────────

#[test]
fn lower() {
    assert_eq!(call("lower", &[s("HELLO")]).as_str(), Some("hello"));
}

#[test]
fn lower_no_args_errors() {
    let result = call_err("lower", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn lower_non_string_errors() {
    let result = call_err("lower", &[n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn lower_empty_string() {
    assert_eq!(call("lower", &[s("")]).as_str(), Some(""));
}

#[test]
fn lower_unicode() {
    assert_eq!(call("lower", &[s("HÉLLO")]).as_str(), Some("héllo"));
}

// ── trim ────────────────────────────────────────────────────────────

#[test]
fn trim() {
    assert_eq!(call("trim", &[s("  hi  ")]).as_str(), Some("hi"));
}

#[test]
fn trim_no_args_errors() {
    let result = call_err("trim", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn trim_non_string_errors() {
    let result = call_err("trim", &[n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn trim_empty_string() {
    assert_eq!(call("trim", &[s("")]).as_str(), Some(""));
}

#[test]
fn trim_no_whitespace() {
    assert_eq!(call("trim", &[s("hello")]).as_str(), Some("hello"));
}

#[test]
fn trim_all_whitespace() {
    assert_eq!(call("trim", &[s("   ")]).as_str(), Some(""));
}

// ── trim_start ──────────────────────────────────────────────────────

#[test]
fn trim_start() {
    assert_eq!(call("trim_start", &[s("  hi  ")]).as_str(), Some("hi  "));
}

#[test]
fn trim_start_no_args_errors() {
    let result = call_err("trim_start", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn trim_start_non_string_errors() {
    let result = call_err("trim_start", &[n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn trim_start_empty_string() {
    assert_eq!(call("trim_start", &[s("")]).as_str(), Some(""));
}

// ── trim_end ────────────────────────────────────────────────────────

#[test]
fn trim_end() {
    assert_eq!(call("trim_end", &[s("  hi  ")]).as_str(), Some("  hi"));
}

#[test]
fn trim_end_no_args_errors() {
    let result = call_err("trim_end", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn trim_end_non_string_errors() {
    let result = call_err("trim_end", &[n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn trim_end_empty_string() {
    assert_eq!(call("trim_end", &[s("")]).as_str(), Some(""));
}

// ── trim_matches ────────────────────────────────────────────────────

#[test]
fn trim_matches_basic() {
    assert_eq!(
        call("trim_matches", &[s("xxhelloxx"), s("x")]).as_str(),
        Some("hello")
    );
}

#[test]
fn trim_matches_multiple_chars() {
    assert_eq!(
        call("trim_matches", &[s("xyhelloxy"), s("xy")]).as_str(),
        Some("hello")
    );
}

#[test]
fn trim_matches_no_match() {
    assert_eq!(
        call("trim_matches", &[s("hello"), s("x")]).as_str(),
        Some("hello")
    );
}

#[test]
fn trim_matches_empty_string() {
    assert_eq!(call("trim_matches", &[s(""), s("x")]).as_str(), Some(""));
}

#[test]
fn trim_matches_empty_chars() {
    assert_eq!(
        call("trim_matches", &[s("hello"), s("")]).as_str(),
        Some("hello")
    );
}

#[test]
fn trim_matches_only_match_chars() {
    assert_eq!(call("trim_matches", &[s("xxx"), s("x")]).as_str(), Some(""));
}

#[test]
fn trim_matches_no_args_errors() {
    let result = call_err("trim_matches", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn trim_matches_first_arg_non_string_errors() {
    let result = call_err("trim_matches", &[n(42.0), s("x")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn trim_matches_second_arg_non_string_errors() {
    let result = call_err("trim_matches", &[s("hello"), n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── reverse ─────────────────────────────────────────────────────────

#[test]
fn reverse() {
    assert_eq!(call("reverse", &[s("hello")]).as_str(), Some("olleh"));
}

#[test]
fn reverse_no_args_errors() {
    let result = call_err("reverse", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn reverse_non_string_errors() {
    let result = call_err("reverse", &[n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn reverse_empty_string() {
    assert_eq!(call("reverse", &[s("")]).as_str(), Some(""));
}

#[test]
fn reverse_unicode() {
    assert_eq!(call("reverse", &[s("héllo")]).as_str(), Some("olléh"));
}

// ── length ──────────────────────────────────────────────────────────

#[test]
fn length_ascii() {
    assert_eq!(call("length", &[s("hello")]).as_number(), Some(5.0));
}

#[test]
fn length_empty() {
    assert_eq!(call("length", &[s("")]).as_number(), Some(0.0));
}

#[test]
fn length_unicode() {
    assert_eq!(call("length", &[s("héllo")]).as_number(), Some(5.0));
}

#[test]
fn length_no_args_errors() {
    let result = call_err("length", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn length_non_string_errors() {
    let result = call_err("length", &[n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn length_empty_string() {
    assert_eq!(call("length", &[s("")]).as_number(), Some(0.0));
}

#[test]
fn length_unicode_edge() {
    assert_eq!(call("length", &[s("café")]).as_number(), Some(4.0));
}

// ── concat ──────────────────────────────────────────────────────────

#[test]
fn concat_two() {
    assert_eq!(
        call("concat", &[s("hello"), s(" world")]).as_str(),
        Some("hello world")
    );
}

#[test]
fn concat_three() {
    assert_eq!(
        call("concat", &[s("a"), s("b"), s("c")]).as_str(),
        Some("abc")
    );
}

#[test]
fn concat_one_arg_errors() {
    let result = call_err("concat", &[s("a")]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn concat_no_args_errors() {
    let result = call_err("concat", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn concat_single_arg_errors() {
    let result = call_err("concat", &[s("a")]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn concat_non_string_arg_errors() {
    let result = call_err("concat", &[s("a"), n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn concat_empty_strings() {
    assert_eq!(call("concat", &[s(""), s("")]).as_str(), Some(""));
}

#[test]
fn concat_three_strings() {
    assert_eq!(
        call("concat", &[s("a"), s("b"), s("c")]).as_str(),
        Some("abc")
    );
}

// ── is_empty ────────────────────────────────────────────────────────

#[test]
fn is_empty_true() {
    assert_eq!(call("is_empty", &[s("")]).as_bool(), Some(true));
}

#[test]
fn is_empty_false() {
    assert_eq!(call("is_empty", &[s("hello")]).as_bool(), Some(false));
}

#[test]
fn is_empty_whitespace_is_not_empty() {
    assert_eq!(call("is_empty", &[s(" ")]).as_bool(), Some(false));
}

#[test]
fn is_empty_no_args_errors() {
    let result = call_err("is_empty", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn is_empty_non_string_errors() {
    let result = call_err("is_empty", &[n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── equals ──────────────────────────────────────────────────────────

#[test]
fn equals_true() {
    assert_eq!(
        call("equals", &[s("hello"), s("hello")]).as_bool(),
        Some(true)
    );
}

#[test]
fn equals_false() {
    assert_eq!(
        call("equals", &[s("hello"), s("world")]).as_bool(),
        Some(false)
    );
}

#[test]
fn equals_both_empty() {
    assert_eq!(call("equals", &[s(""), s("")]).as_bool(), Some(true));
}

#[test]
fn equals_case_sensitive() {
    assert_eq!(
        call("equals", &[s("Hello"), s("hello")]).as_bool(),
        Some(false)
    );
}

#[test]
fn equals_no_args_errors() {
    let result = call_err("equals", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn equals_one_arg_errors() {
    let result = call_err("equals", &[s("hello")]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn equals_first_arg_non_string_errors() {
    let result = call_err("equals", &[n(42.0), s("hello")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn equals_second_arg_non_string_errors() {
    let result = call_err("equals", &[s("hello"), n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── bytes ───────────────────────────────────────────────────────────

#[test]
fn bytes_ascii() {
    assert_eq!(call("bytes", &[s("hello")]).as_number(), Some(5.0));
}

#[test]
fn bytes_empty() {
    assert_eq!(call("bytes", &[s("")]).as_number(), Some(0.0));
}

#[test]
fn bytes_unicode() {
    assert_eq!(call("bytes", &[s("café")]).as_number(), Some(5.0));
}

#[test]
fn bytes_no_args_errors() {
    let result = call_err("bytes", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn bytes_non_string_errors() {
    let result = call_err("bytes", &[n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── is_alpha ────────────────────────────────────────────────────────

#[test]
fn is_alpha_true() {
    assert_eq!(call("is_alpha", &[s("hello")]).as_bool(), Some(true));
}

#[test]
fn is_alpha_false() {
    assert_eq!(call("is_alpha", &[s("hello123")]).as_bool(), Some(false));
}

#[test]
fn is_alpha_empty_false() {
    assert_eq!(call("is_alpha", &[s("")]).as_bool(), Some(false));
}

#[test]
fn is_alpha_unicode() {
    assert_eq!(call("is_alpha", &[s("café")]).as_bool(), Some(true));
}

#[test]
fn is_alpha_no_args_errors() {
    let result = call_err("is_alpha", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn is_alpha_non_string_errors() {
    let result = call_err("is_alpha", &[n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── is_numeric ──────────────────────────────────────────────────────

#[test]
fn is_numeric_true() {
    assert_eq!(call("is_numeric", &[s("123")]).as_bool(), Some(true));
}

#[test]
fn is_numeric_false() {
    assert_eq!(call("is_numeric", &[s("123abc")]).as_bool(), Some(false));
}

#[test]
fn is_numeric_empty_false() {
    assert_eq!(call("is_numeric", &[s("")]).as_bool(), Some(false));
}

#[test]
fn is_numeric_unicode_digits() {
    assert_eq!(call("is_numeric", &[s("১২৩")]).as_bool(), Some(true));
}

#[test]
fn is_numeric_no_args_errors() {
    let result = call_err("is_numeric", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn is_numeric_non_string_errors() {
    let result = call_err("is_numeric", &[n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── is_alphanumeric ─────────────────────────────────────────────────

#[test]
fn is_alphanumeric_true() {
    assert_eq!(
        call("is_alphanumeric", &[s("hello123")]).as_bool(),
        Some(true)
    );
}

#[test]
fn is_alphanumeric_false() {
    assert_eq!(
        call("is_alphanumeric", &[s("hello 123")]).as_bool(),
        Some(false)
    );
}

#[test]
fn is_alphanumeric_empty_false() {
    assert_eq!(call("is_alphanumeric", &[s("")]).as_bool(), Some(false));
}

#[test]
fn is_alphanumeric_unicode() {
    assert_eq!(
        call("is_alphanumeric", &[s("café123")]).as_bool(),
        Some(true)
    );
}

#[test]
fn is_alphanumeric_no_args_errors() {
    let result = call_err("is_alphanumeric", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn is_alphanumeric_non_string_errors() {
    let result = call_err("is_alphanumeric", &[n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── shared error tests ──────────────────────────────────────────────

#[test]
fn unknown_function_returns_none() {
    assert!(StrModule.function("nope").is_none());
}

#[test]
fn type_mismatch_on_non_string() {
    let result = call_err("upper", &[n(42.0)]);
    assert_eq!(
        result,
        Err(Error::TypeMismatch {
            expected: "string",
            got: "non-string"
        })
    );
}

#[test]
fn argument_count_on_missing_args() {
    let result = call_err("upper", &[]);
    assert_eq!(
        result,
        Err(Error::ArgumentCount {
            expected: 1,
            got: 0
        })
    );
}
