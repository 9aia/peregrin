use ac_runtime::NonFunctionValue;
use ac_runtime_core::Module;
use ac_vm::{Context, Error};

use super::StrModule;

fn call(name: &str, args: &[NonFunctionValue]) -> NonFunctionValue {
    let func = StrModule.function(name).expect("function exists");
    let mut ctx = Context::new();
    func.call(args, &mut ctx).expect("call succeeds")
}

fn call_err(name: &str, args: &[NonFunctionValue]) -> Result<NonFunctionValue, Error> {
    let func = StrModule.function(name).expect("function exists");
    let mut ctx = Context::new();
    func.call(args, &mut ctx)
}

fn s(v: &str) -> NonFunctionValue {
    NonFunctionValue::new(v.to_string())
}

fn n(v: f64) -> NonFunctionValue {
    NonFunctionValue::new(serde_json::json!(v))
}

// ── contains ────────────────────────────────────────────────────────

#[test]
fn contains_true() {
    assert_eq!(call("contains", &[s("hello world"), s("world")]).as_bool(), Some(true));
}

#[test]
fn contains_false() {
    assert_eq!(call("contains", &[s("hello world"), s("xyz")]).as_bool(), Some(false));
}

#[test]
fn contains_no_args_errors() {
    let result = call_err("contains", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn contains_one_arg_errors() {
    let result = call_err("contains", &[s("hello")]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn contains_first_arg_non_string_errors() {
    let result = call_err("contains", &[n(42.0), s("ell")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn contains_second_arg_non_string_errors() {
    let result = call_err("contains", &[s("hello"), n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn contains_empty_pattern() {
    assert_eq!(call("contains", &[s("hello"), s("")]).as_bool(), Some(true));
}

// ── starts_with ─────────────────────────────────────────────────────

#[test]
fn starts_with_true() {
    assert_eq!(call("starts_with", &[s("hello"), s("he")]).as_bool(), Some(true));
}

#[test]
fn starts_with_false() {
    assert_eq!(call("starts_with", &[s("hello"), s("lo")]).as_bool(), Some(false));
}

#[test]
fn starts_with_no_args_errors() {
    let result = call_err("starts_with", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn starts_with_first_arg_non_string_errors() {
    let result = call_err("starts_with", &[n(42.0), s("he")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn starts_with_second_arg_non_string_errors() {
    let result = call_err("starts_with", &[s("hello"), n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── ends_with ───────────────────────────────────────────────────────

#[test]
fn ends_with_true() {
    assert_eq!(call("ends_with", &[s("hello"), s("lo")]).as_bool(), Some(true));
}

#[test]
fn ends_with_false() {
    assert_eq!(call("ends_with", &[s("hello"), s("he")]).as_bool(), Some(false));
}

#[test]
fn ends_with_no_args_errors() {
    let result = call_err("ends_with", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn ends_with_first_arg_non_string_errors() {
    let result = call_err("ends_with", &[n(42.0), s("lo")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn ends_with_second_arg_non_string_errors() {
    let result = call_err("ends_with", &[s("hello"), n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── count ───────────────────────────────────────────────────────────

#[test]
fn count() {
    assert_eq!(call("count", &[s("foo foo foo"), s("foo")]).as_number(), Some(3.0));
}

#[test]
fn count_empty_pattern() {
    assert_eq!(call("count", &[s("hello"), s("")]).as_number(), Some(0.0));
}

#[test]
fn count_no_args_errors() {
    let result = call_err("count", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn count_first_arg_non_string_errors() {
    let result = call_err("count", &[n(42.0), s("l")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn count_second_arg_non_string_errors() {
    let result = call_err("count", &[s("hello"), n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn count_empty_pattern_returns_zero() {
    assert_eq!(call("count", &[s("hello"), s("")]).as_number(), Some(0.0));
}

#[test]
fn count_no_match() {
    assert_eq!(call("count", &[s("hello"), s("z")]).as_number(), Some(0.0));
}

// ── find ────────────────────────────────────────────────────────────

#[test]
fn find_found() {
    assert_eq!(call("find", &[s("hello world"), s("world")]).as_number(), Some(6.0));
}

#[test]
fn find_not_found() {
    assert_eq!(call("find", &[s("hello"), s("xyz")]).as_number(), Some(-1.0));
}

#[test]
fn find_no_args_errors() {
    let result = call_err("find", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn find_first_arg_non_string_errors() {
    let result = call_err("find", &[n(42.0), s("l")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn find_second_arg_non_string_errors() {
    let result = call_err("find", &[s("hello"), n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn find_no_match_returns_minus_one() {
    assert_eq!(call("find", &[s("hello"), s("z")]).as_number(), Some(-1.0));
}

#[test]
fn find_empty_pattern() {
    assert_eq!(call("find", &[s("hello"), s("")]).as_number(), Some(0.0));
}

#[test]
fn find_unicode() {
    assert_eq!(call("find", &[s("café world"), s("world")]).as_number(), Some(5.0));
}

#[test]
fn find_unicode_returns_char_offset() {
    let result = call("find", &[s("héllo"), s("lo")]);
    assert_eq!(result.as_number(), Some(3.0));
}

// ── rfind ───────────────────────────────────────────────────────────

#[test]
fn rfind_found() {
    assert_eq!(call("rfind", &[s("hello world hello"), s("hello")]).as_number(), Some(12.0));
}

#[test]
fn rfind_single_match() {
    assert_eq!(call("rfind", &[s("hello"), s("ell")]).as_number(), Some(1.0));
}

#[test]
fn rfind_not_found() {
    assert_eq!(call("rfind", &[s("hello"), s("xyz")]).as_number(), Some(-1.0));
}

#[test]
fn rfind_empty_pattern() {
    assert_eq!(call("rfind", &[s("hello"), s("")]).as_number(), Some(5.0));
}

#[test]
fn rfind_unicode() {
    assert_eq!(call("rfind", &[s("café café"), s("café")]).as_number(), Some(5.0));
}

#[test]
fn rfind_no_args_errors() {
    let result = call_err("rfind", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn rfind_first_arg_non_string_errors() {
    let result = call_err("rfind", &[n(42.0), s("l")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn rfind_second_arg_non_string_errors() {
    let result = call_err("rfind", &[s("hello"), n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}
