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

fn arr(items: &[&str]) -> NonFunctionValue {
    let values: Vec<serde_json::Value> = items
        .iter()
        .map(|item| serde_json::Value::String(item.to_string()))
        .collect();
    NonFunctionValue::new(serde_json::Value::Array(values))
}

// ── split ───────────────────────────────────────────────────────────

#[test]
fn split() {
    let result = call("split", &[s("a,b,c"), s(",")]);
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array[0].as_str(), Some("a"));
    assert_eq!(array[1].as_str(), Some("b"));
    assert_eq!(array[2].as_str(), Some("c"));
}

#[test]
fn split_no_args_errors() {
    let result = call_err("split", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn split_one_arg_errors() {
    let result = call_err("split", &[s("a,b,c")]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn split_first_arg_non_string_errors() {
    let result = call_err("split", &[n(42.0), s(",")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn split_second_arg_non_string_errors() {
    let result = call_err("split", &[s("a,b"), n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn split_empty_string() {
    let result = call("split", &[s(""), s(",")]);
    assert_eq!(result.as_array().unwrap().len(), 1);
}

#[test]
fn split_no_match() {
    let result = call("split", &[s("hello"), s(",")]);
    assert_eq!(result.as_array().unwrap().len(), 1);
    assert_eq!(result.as_array().unwrap()[0].as_str(), Some("hello"));
}

#[test]
fn split_empty_delimiter() {
    let result = call("split", &[s("abc"), s("")]);
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 5);
    assert_eq!(arr[0].as_str(), Some(""));
    assert_eq!(arr[4].as_str(), Some(""));
}

// ── join ────────────────────────────────────────────────────────────

#[test]
fn join() {
    let result = call("join", &[arr(&["a", "b", "c"]), s("-")]);
    assert_eq!(result.as_str(), Some("a-b-c"));
}

#[test]
fn join_no_args_errors() {
    let result = call_err("join", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn join_one_arg_errors() {
    let result = call_err("join", &[arr(&["a", "b"])]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn join_first_arg_non_array_errors() {
    let result = call_err("join", &[s("hello"), s(",")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn join_second_arg_non_string_errors() {
    let result = call_err("join", &[arr(&["a", "b"]), n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn join_empty_array() {
    assert_eq!(call("join", &[arr(&[]), s(",")]).as_str(), Some(""));
}

#[test]
fn join_with_non_string_elements() {
    let json_arr = NonFunctionValue::new(serde_json::json!([1, 2, 3]));
    assert_eq!(call("join", &[json_arr, s(",")]).as_str(), Some(""));
}

// ── replace ─────────────────────────────────────────────────────────

#[test]
fn replace() {
    let result = call("replace", &[s("foo bar foo"), s("foo"), s("baz")]);
    assert_eq!(result.as_str(), Some("baz bar baz"));
}

#[test]
fn replace_no_args_errors() {
    let result = call_err("replace", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn replace_two_args_errors() {
    let result = call_err("replace", &[s("hello"), s("l")]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn replace_first_arg_non_string_errors() {
    let result = call_err("replace", &[n(42.0), s("l"), s("L")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn replace_second_arg_non_string_errors() {
    let result = call_err("replace", &[s("hello"), n(42.0), s("L")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn replace_third_arg_non_string_errors() {
    let result = call_err("replace", &[s("hello"), s("l"), n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn replace_no_match() {
    assert_eq!(
        call("replace", &[s("hello"), s("z"), s("x")]).as_str(),
        Some("hello")
    );
}

#[test]
fn replace_empty_string() {
    assert_eq!(call("replace", &[s(""), s("a"), s("b")]).as_str(), Some(""));
}

// ── replace_first ───────────────────────────────────────────────────

#[test]
fn replace_first() {
    assert_eq!(
        call("replace_first", &[s("foo bar foo"), s("foo"), s("baz")]).as_str(),
        Some("baz bar foo")
    );
}

#[test]
fn replace_first_no_match() {
    assert_eq!(
        call("replace_first", &[s("hello"), s("xyz"), s("abc")]).as_str(),
        Some("hello")
    );
}

#[test]
fn replace_first_no_args_errors() {
    let result = call_err("replace_first", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn replace_first_two_args_errors() {
    let result = call_err("replace_first", &[s("hello"), s("l")]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn replace_first_first_arg_non_string_errors() {
    let result = call_err("replace_first", &[n(42.0), s("l"), s("L")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn replace_first_second_arg_non_string_errors() {
    let result = call_err("replace_first", &[s("hello"), n(42.0), s("L")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn replace_first_third_arg_non_string_errors() {
    let result = call_err("replace_first", &[s("hello"), s("l"), n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn replace_first_no_match_edge() {
    assert_eq!(
        call("replace_first", &[s("hello"), s("z"), s("x")]).as_str(),
        Some("hello")
    );
}

#[test]
fn replace_first_only_first() {
    assert_eq!(
        call("replace_first", &[s("hello"), s("l"), s("L")]).as_str(),
        Some("heLlo")
    );
}

// ── repeat ──────────────────────────────────────────────────────────

#[test]
fn repeat() {
    assert_eq!(call("repeat", &[s("ab"), n(3.0)]).as_str(), Some("ababab"));
}

#[test]
fn repeat_zero() {
    assert_eq!(call("repeat", &[s("ab"), n(0.0)]).as_str(), Some(""));
}

#[test]
fn repeat_negative_errors() {
    let result = call_err("repeat", &[s("ab"), n(-1.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn repeat_no_args_errors() {
    let result = call_err("repeat", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn repeat_first_arg_non_string_errors() {
    let result = call_err("repeat", &[n(42.0), n(3.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn repeat_second_arg_non_number_errors() {
    let result = call_err("repeat", &[s("ab"), s("x")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn repeat_zero_times() {
    assert_eq!(call("repeat", &[s("ab"), n(0.0)]).as_str(), Some(""));
}

#[test]
fn repeat_empty_string() {
    assert_eq!(call("repeat", &[s(""), n(5.0)]).as_str(), Some(""));
}

// regression: overflow guard
#[test]
fn repeat_infinity_errors() {
    let result = call_err("repeat", &[s("ab"), n(f64::INFINITY)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn repeat_excessive_count_errors() {
    let result = call_err("repeat", &[s("ab"), n(1_000_001.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn repeat_max_allowed() {
    let result = call("repeat", &[s(""), n(1_000_000.0)]);
    assert_eq!(result.as_str(), Some(""));
}

// ── char_at ─────────────────────────────────────────────────────────

#[test]
fn char_at() {
    assert_eq!(call("char_at", &[s("hello"), n(1.0)]).as_str(), Some("e"));
}

#[test]
fn char_at_out_of_bounds() {
    let result = call("char_at", &[s("hi"), n(10.0)]);
    assert!(result.as_str().is_none());
}

#[test]
fn char_at_negative_errors() {
    let result = call_err("char_at", &[s("hello"), n(-1.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn char_at_no_args_errors() {
    let result = call_err("char_at", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn char_at_first_arg_non_string_errors() {
    let result = call_err("char_at", &[n(42.0), n(0.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn char_at_second_arg_non_number_errors() {
    let result = call_err("char_at", &[s("hello"), s("x")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn char_at_out_of_bounds_returns_null() {
    let result = call("char_at", &[s("hi"), n(10.0)]);
    assert!(result.is_null());
}

#[test]
fn char_at_unicode() {
    assert_eq!(call("char_at", &[s("café"), n(3.0)]).as_str(), Some("é"));
}

// ── slice ───────────────────────────────────────────────────────────

#[test]
fn slice() {
    let result = call("slice", &[s("hello"), n(1.0), n(4.0)]);
    assert_eq!(result.as_str(), Some("ell"));
}

#[test]
fn slice_out_of_bounds_clamps() {
    let result = call("slice", &[s("hi"), n(0.0), n(10.0)]);
    assert_eq!(result.as_str(), Some("hi"));
}

#[test]
fn slice_unicode() {
    let result = call("slice", &[s("héllo"), n(1.0), n(4.0)]);
    assert_eq!(result.as_str(), Some("éll"));
}

#[test]
fn slice_negative_errors() {
    let result = call_err("slice", &[s("hello"), n(-1.0), n(3.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn slice_no_args_errors() {
    let result = call_err("slice", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn slice_two_args_errors() {
    let result = call_err("slice", &[s("hello"), n(0.0)]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn slice_first_arg_non_string_errors() {
    let result = call_err("slice", &[n(42.0), n(0.0), n(3.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn slice_negative_start_errors() {
    let result = call_err("slice", &[s("hello"), n(-1.0), n(3.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn slice_start_beyond_length() {
    assert_eq!(
        call("slice", &[s("hi"), n(10.0), n(15.0)]).as_str(),
        Some("")
    );
}

#[test]
fn slice_empty_string() {
    assert_eq!(call("slice", &[s(""), n(0.0), n(5.0)]).as_str(), Some(""));
}

#[test]
fn slice_unicode_edge() {
    assert_eq!(
        call("slice", &[s("café world"), n(0.0), n(4.0)]).as_str(),
        Some("café")
    );
}

// ── pad_left ────────────────────────────────────────────────────────

#[test]
fn pad_left() {
    assert_eq!(
        call("pad_left", &[s("hi"), n(5.0), s(" ")]).as_str(),
        Some("   hi")
    );
}

#[test]
fn pad_left_multi_char() {
    assert_eq!(
        call("pad_left", &[s("ab"), n(7.0), s("-+")]).as_str(),
        Some("-+-+-ab")
    );
}

#[test]
fn pad_left_already_long_enough() {
    assert_eq!(
        call("pad_left", &[s("hello"), n(3.0), s(" ")]).as_str(),
        Some("hello")
    );
}

#[test]
fn pad_left_negative_errors() {
    let result = call_err("pad_left", &[s("hi"), n(-1.0), s(" ")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn pad_left_unicode() {
    assert_eq!(
        call("pad_left", &[s("café"), n(6.0), s(" ")]).as_str(),
        Some("  café")
    );
}

#[test]
fn pad_left_no_args_errors() {
    let result = call_err("pad_left", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn pad_left_two_args_errors() {
    let result = call_err("pad_left", &[s("hi"), n(5.0)]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn pad_left_first_arg_non_string_errors() {
    let result = call_err("pad_left", &[n(42.0), n(5.0), s(" ")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn pad_left_second_arg_non_number_errors() {
    let result = call_err("pad_left", &[s("hi"), s("x"), s(" ")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn pad_left_third_arg_non_string_errors() {
    let result = call_err("pad_left", &[s("hi"), n(5.0), n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn pad_left_empty_pad_char() {
    assert_eq!(
        call("pad_left", &[s("hi"), n(5.0), s("")]).as_str(),
        Some("hi")
    );
}

#[test]
fn pad_left_already_long_enough_extra() {
    assert_eq!(
        call("pad_left", &[s("hello"), n(3.0), s(" ")]).as_str(),
        Some("hello")
    );
}

#[test]
fn pad_left_multi_char_pad() {
    assert_eq!(
        call("pad_left", &[s("hi"), n(6.0), s("ab")]).as_str(),
        Some("ababhi")
    );
}

// regression: overflow guard
#[test]
fn pad_left_infinity_errors() {
    let result = call_err("pad_left", &[s("hi"), n(f64::INFINITY), s(" ")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn pad_left_excessive_target_errors() {
    let result = call_err("pad_left", &[s("hi"), n(1_000_001.0), s(" ")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── pad_right ───────────────────────────────────────────────────────

#[test]
fn pad_right() {
    assert_eq!(
        call("pad_right", &[s("hi"), n(5.0), s(" ")]).as_str(),
        Some("hi   ")
    );
}

#[test]
fn pad_right_multi_char() {
    assert_eq!(
        call("pad_right", &[s("ab"), n(7.0), s("-+")]).as_str(),
        Some("ab-+-+-")
    );
}

#[test]
fn pad_right_already_long_enough() {
    assert_eq!(
        call("pad_right", &[s("hello"), n(3.0), s(" ")]).as_str(),
        Some("hello")
    );
}

#[test]
fn pad_right_negative_errors() {
    let result = call_err("pad_right", &[s("hi"), n(-1.0), s(" ")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn pad_right_no_args_errors() {
    let result = call_err("pad_right", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn pad_right_two_args_errors() {
    let result = call_err("pad_right", &[s("hi"), n(5.0)]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn pad_right_first_arg_non_string_errors() {
    let result = call_err("pad_right", &[n(42.0), n(5.0), s(" ")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn pad_right_second_arg_non_number_errors() {
    let result = call_err("pad_right", &[s("hi"), s("x"), s(" ")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn pad_right_third_arg_non_string_errors() {
    let result = call_err("pad_right", &[s("hi"), n(5.0), n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn pad_right_empty_pad_char() {
    assert_eq!(
        call("pad_right", &[s("hi"), n(5.0), s("")]).as_str(),
        Some("hi")
    );
}

#[test]
fn pad_right_already_long_enough_extra() {
    assert_eq!(
        call("pad_right", &[s("hello"), n(3.0), s(" ")]).as_str(),
        Some("hello")
    );
}

#[test]
fn pad_right_unicode() {
    assert_eq!(
        call("pad_right", &[s("café"), n(6.0), s(" ")]).as_str(),
        Some("café  ")
    );
}

// regression: overflow guard
#[test]
fn pad_right_infinity_errors() {
    let result = call_err("pad_right", &[s("hi"), n(f64::INFINITY), s(" ")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn pad_right_excessive_target_errors() {
    let result = call_err("pad_right", &[s("hi"), n(1_000_001.0), s(" ")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── strip_prefix ────────────────────────────────────────────────────

#[test]
fn strip_prefix_present() {
    assert_eq!(
        call("strip_prefix", &[s("hello world"), s("hello ")]).as_str(),
        Some("world")
    );
}

#[test]
fn strip_prefix_not_present() {
    assert_eq!(
        call("strip_prefix", &[s("hello"), s("world")]).as_str(),
        Some("hello")
    );
}

#[test]
fn strip_prefix_exact_match() {
    assert_eq!(
        call("strip_prefix", &[s("hello"), s("hello")]).as_str(),
        Some("")
    );
}

#[test]
fn strip_prefix_empty_prefix() {
    assert_eq!(
        call("strip_prefix", &[s("hello"), s("")]).as_str(),
        Some("hello")
    );
}

#[test]
fn strip_prefix_no_args_errors() {
    let result = call_err("strip_prefix", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn strip_prefix_first_arg_non_string_errors() {
    let result = call_err("strip_prefix", &[n(42.0), s("he")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn strip_prefix_second_arg_non_string_errors() {
    let result = call_err("strip_prefix", &[s("hello"), n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── strip_suffix ────────────────────────────────────────────────────

#[test]
fn strip_suffix_present() {
    assert_eq!(
        call("strip_suffix", &[s("hello world"), s(" world")]).as_str(),
        Some("hello")
    );
}

#[test]
fn strip_suffix_not_present() {
    assert_eq!(
        call("strip_suffix", &[s("hello"), s("world")]).as_str(),
        Some("hello")
    );
}

#[test]
fn strip_suffix_exact_match() {
    assert_eq!(
        call("strip_suffix", &[s("hello"), s("hello")]).as_str(),
        Some("")
    );
}

#[test]
fn strip_suffix_empty_suffix() {
    assert_eq!(
        call("strip_suffix", &[s("hello"), s("")]).as_str(),
        Some("hello")
    );
}

#[test]
fn strip_suffix_no_args_errors() {
    let result = call_err("strip_suffix", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn strip_suffix_first_arg_non_string_errors() {
    let result = call_err("strip_suffix", &[n(42.0), s("lo")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn strip_suffix_second_arg_non_string_errors() {
    let result = call_err("strip_suffix", &[s("hello"), n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── lines ───────────────────────────────────────────────────────────

#[test]
fn lines_basic() {
    let result = call("lines", &[s("a\nb\nc")]);
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array[0].as_str(), Some("a"));
    assert_eq!(array[1].as_str(), Some("b"));
    assert_eq!(array[2].as_str(), Some("c"));
}

#[test]
fn lines_crlf() {
    let result = call("lines", &[s("a\r\nb\r\nc")]);
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array[0].as_str(), Some("a"));
    assert_eq!(array[1].as_str(), Some("b"));
    assert_eq!(array[2].as_str(), Some("c"));
}

#[test]
fn lines_single_line() {
    let result = call("lines", &[s("hello")]);
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 1);
    assert_eq!(array[0].as_str(), Some("hello"));
}

#[test]
fn lines_empty_string() {
    let result = call("lines", &[s("")]);
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 1);
    assert_eq!(array[0].as_str(), Some(""));
}

#[test]
fn lines_trailing_newline() {
    let result = call("lines", &[s("a\nb\n")]);
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 2);
    assert_eq!(array[0].as_str(), Some("a"));
    assert_eq!(array[1].as_str(), Some("b"));
}

#[test]
fn lines_no_args_errors() {
    let result = call_err("lines", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn lines_non_string_errors() {
    let result = call_err("lines", &[n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── split_at ────────────────────────────────────────────────────────

#[test]
fn split_at_basic() {
    let result = call("split_at", &[s("hello"), n(2.0)]);
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 2);
    assert_eq!(array[0].as_str(), Some("he"));
    assert_eq!(array[1].as_str(), Some("llo"));
}

#[test]
fn split_at_zero() {
    let result = call("split_at", &[s("hello"), n(0.0)]);
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 2);
    assert_eq!(array[0].as_str(), Some(""));
    assert_eq!(array[1].as_str(), Some("hello"));
}

#[test]
fn split_at_end() {
    let result = call("split_at", &[s("hello"), n(5.0)]);
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 2);
    assert_eq!(array[0].as_str(), Some("hello"));
    assert_eq!(array[1].as_str(), Some(""));
}

#[test]
fn split_at_beyond_length_clamps() {
    let result = call("split_at", &[s("hi"), n(10.0)]);
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 2);
    assert_eq!(array[0].as_str(), Some("hi"));
    assert_eq!(array[1].as_str(), Some(""));
}

#[test]
fn split_at_unicode() {
    let result = call("split_at", &[s("café"), n(2.0)]);
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 2);
    assert_eq!(array[0].as_str(), Some("ca"));
    assert_eq!(array[1].as_str(), Some("fé"));
}

#[test]
fn split_at_no_args_errors() {
    let result = call_err("split_at", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn split_at_first_arg_non_string_errors() {
    let result = call_err("split_at", &[n(42.0), n(2.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn split_at_second_arg_non_number_errors() {
    let result = call_err("split_at", &[s("hello"), s("x")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn split_at_negative_errors() {
    let result = call_err("split_at", &[s("hello"), n(-1.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}
