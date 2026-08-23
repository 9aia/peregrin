use ac_runtime::NonFunctionValue;
use ac_runtime_core::Module;
use ac_vm::{CanonicalContext, Context, Error};

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
fn join_with_non_string_elements_errors() {
    let json_arr = NonFunctionValue::new(serde_json::json!([1, 2, 3]));
    let result = call_err("join", &[json_arr, s(",")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
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
fn char_at_out_of_bounds_errors() {
    let result = call_err("char_at", &[s("hi"), n(10.0)]);
    assert!(matches!(result, Err(Error::PrimitiveError(_))));
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
fn char_at_at_last_char() {
    assert_eq!(call("char_at", &[s("hi"), n(1.0)]).as_str(), Some("i"));
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

// ── split_whitespace ────────────────────────────────────────────────

#[test]
fn split_whitespace_basic() {
    let result = call("split_whitespace", &[s("  hello   world  ")]);
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 2);
    assert_eq!(array[0].as_str(), Some("hello"));
    assert_eq!(array[1].as_str(), Some("world"));
}

#[test]
fn split_whitespace_tabs_and_newlines() {
    let result = call("split_whitespace", &[s("a\tb\nc")]);
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array[0].as_str(), Some("a"));
    assert_eq!(array[1].as_str(), Some("b"));
    assert_eq!(array[2].as_str(), Some("c"));
}

#[test]
fn split_whitespace_empty_string() {
    let result = call("split_whitespace", &[s("")]);
    assert_eq!(result.as_array().unwrap().len(), 0);
}

#[test]
fn split_whitespace_all_whitespace() {
    let result = call("split_whitespace", &[s("   \t\n  ")]);
    assert_eq!(result.as_array().unwrap().len(), 0);
}

#[test]
fn split_whitespace_no_args_errors() {
    let result = call_err("split_whitespace", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn split_whitespace_non_string_errors() {
    let result = call_err("split_whitespace", &[n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── truncate ────────────────────────────────────────────────────────

#[test]
fn truncate_with_ellipsis() {
    assert_eq!(
        call("truncate", &[s("hello world"), n(8.0)]).as_str(),
        Some("hello...")
    );
}

#[test]
fn truncate_custom_suffix() {
    assert_eq!(
        call("truncate", &[s("hello world"), n(7.0), s("!")]).as_str(),
        Some("hello !")
    );
}

#[test]
fn truncate_no_truncation_needed() {
    assert_eq!(
        call("truncate", &[s("hi"), n(10.0)]).as_str(),
        Some("hi")
    );
}

#[test]
fn truncate_exact_length() {
    assert_eq!(
        call("truncate", &[s("hello"), n(5.0)]).as_str(),
        Some("hello")
    );
}

#[test]
fn truncate_suffix_equal_to_max() {
    assert_eq!(
        call("truncate", &[s("hello world"), n(3.0), s("...")]).as_str(),
        Some("...")
    );
}

#[test]
fn truncate_zero_max() {
    assert_eq!(
        call("truncate", &[s("hello"), n(0.0)]).as_str(),
        Some("")
    );
}

#[test]
fn truncate_negative_errors() {
    let result = call_err("truncate", &[s("hello"), n(-1.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn truncate_no_args_errors() {
    let result = call_err("truncate", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn truncate_first_arg_non_string_errors() {
    let result = call_err("truncate", &[n(42.0), n(5.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn truncate_second_arg_non_number_errors() {
    let result = call_err("truncate", &[s("hello"), s("x")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn truncate_unicode() {
    assert_eq!(
        call("truncate", &[s("héllo wörld"), n(8.0)]).as_str(),
        Some("héllo...")
    );
}

// ── slugify ─────────────────────────────────────────────────────────

#[test]
fn slugify_basic() {
    assert_eq!(call("slugify", &[s("Hello World")]).as_str(), Some("hello-world"));
}

#[test]
fn slugify_special_chars() {
    assert_eq!(call("slugify", &[s("Hello, World!")]).as_str(), Some("hello-world"));
}

#[test]
fn slugify_multiple_spaces() {
    assert_eq!(call("slugify", &[s("hello   world")]).as_str(), Some("hello-world"));
}

#[test]
fn slugify_underscores() {
    assert_eq!(call("slugify", &[s("hello_world")]).as_str(), Some("hello-world"));
}

#[test]
fn slugify_leading_trailing_dashes() {
    assert_eq!(call("slugify", &[s("---hello---")]).as_str(), Some("hello"));
}

#[test]
fn slugify_empty_string() {
    assert_eq!(call("slugify", &[s("")]).as_str(), Some(""));
}

#[test]
fn slugify_only_special_chars() {
    assert_eq!(call("slugify", &[s("!@#$%")]).as_str(), Some(""));
}

#[test]
fn slugify_unicode() {
    assert_eq!(call("slugify", &[s("café au lait")]).as_str(), Some("café-au-lait"));
}

#[test]
fn slugify_no_args_errors() {
    let result = call_err("slugify", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn slugify_non_string_errors() {
    let result = call_err("slugify", &[n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── escape_html ─────────────────────────────────────────────────────

#[test]
fn escape_html_basic() {
    assert_eq!(
        call("escape_html", &[s("<div>\"hello\" & 'world' / </div>")]).as_str(),
        Some("&lt;div&gt;&quot;hello&quot; &amp; &#x27;world&#x27; &#x2F; &lt;&#x2F;div&gt;")
    );
}

#[test]
fn escape_html_no_special_chars() {
    assert_eq!(
        call("escape_html", &[s("hello world")]).as_str(),
        Some("hello world")
    );
}

#[test]
fn escape_html_empty_string() {
    assert_eq!(call("escape_html", &[s("")]).as_str(), Some(""));
}

#[test]
fn escape_html_no_args_errors() {
    let result = call_err("escape_html", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn escape_html_non_string_errors() {
    let result = call_err("escape_html", &[n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── unescape_html ───────────────────────────────────────────────────

#[test]
fn unescape_html_basic() {
    assert_eq!(
        call("unescape_html", &[s("&lt;div&gt;&quot;hello&quot; &amp; &#x27;world&#x27; &#x2F; &lt;/div&gt;")]).as_str(),
        Some("<div>\"hello\" & 'world' / </div>")
    );
}

#[test]
fn unescape_html_numeric_entities() {
    assert_eq!(
        call("unescape_html", &[s("&#39;&#47;")]).as_str(),
        Some("'/")
    );
}

#[test]
fn unescape_html_no_entities() {
    assert_eq!(
        call("unescape_html", &[s("hello world")]).as_str(),
        Some("hello world")
    );
}

#[test]
fn unescape_html_empty_string() {
    assert_eq!(call("unescape_html", &[s("")]).as_str(), Some(""));
}

#[test]
fn unescape_html_no_args_errors() {
    let result = call_err("unescape_html", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── escape_url ──────────────────────────────────────────────────────

#[test]
fn escape_url_basic() {
    assert_eq!(
        call("escape_url", &[s("hello world")]).as_str(),
        Some("hello%20world")
    );
}

#[test]
fn escape_url_unreserved_kept() {
    assert_eq!(
        call("escape_url", &[s("a-b_c.d~e")]).as_str(),
        Some("a-b_c.d~e")
    );
}

#[test]
fn escape_url_special_chars() {
    assert_eq!(
        call("escape_url", &[s("a&b=c")]).as_str(),
        Some("a%26b%3Dc")
    );
}

#[test]
fn escape_url_empty_string() {
    assert_eq!(call("escape_url", &[s("")]).as_str(), Some(""));
}

#[test]
fn escape_url_no_args_errors() {
    let result = call_err("escape_url", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn escape_url_non_string_errors() {
    let result = call_err("escape_url", &[n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── unescape_url ────────────────────────────────────────────────────

#[test]
fn unescape_url_basic() {
    assert_eq!(
        call("unescape_url", &[s("hello%20world")]).as_str(),
        Some("hello world")
    );
}

#[test]
fn unescape_url_roundtrip() {
    let encoded = call("escape_url", &[s("a&b=c/d?e")]);
    let decoded = call("unescape_url", &[encoded]);
    assert_eq!(decoded.as_str(), Some("a&b=c/d?e"));
}

#[test]
fn unescape_url_no_encoding() {
    assert_eq!(
        call("unescape_url", &[s("hello")]).as_str(),
        Some("hello")
    );
}

#[test]
fn unescape_url_empty_string() {
    assert_eq!(call("unescape_url", &[s("")]).as_str(), Some(""));
}

#[test]
fn unescape_url_no_args_errors() {
    let result = call_err("unescape_url", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── word_wrap ───────────────────────────────────────────────────────

#[test]
fn word_wrap_basic() {
    assert_eq!(
        call("word_wrap", &[s("hello world foo bar"), n(10.0)]).as_str(),
        Some("hello\nworld foo\nbar")
    );
}

#[test]
fn word_wrap_short_width() {
    assert_eq!(
        call("word_wrap", &[s("a b c d"), n(3.0)]).as_str(),
        Some("a b\nc d")
    );
}

#[test]
fn word_wrap_single_word() {
    assert_eq!(
        call("word_wrap", &[s("hello"), n(3.0)]).as_str(),
        Some("hello")
    );
}

#[test]
fn word_wrap_multiline() {
    assert_eq!(
        call("word_wrap", &[s("a b\nc d"), n(3.0)]).as_str(),
        Some("a b\nc d")
    );
}

#[test]
fn word_wrap_empty_string() {
    assert_eq!(call("word_wrap", &[s(""), n(10.0)]).as_str(), Some(""));
}

#[test]
fn word_wrap_empty_lines() {
    assert_eq!(
        call("word_wrap", &[s("a\n\nb"), n(10.0)]).as_str(),
        Some("a\n\nb")
    );
}

#[test]
fn word_wrap_negative_width_errors() {
    let result = call_err("word_wrap", &[s("hello"), n(-1.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn word_wrap_zero_width_errors() {
    let result = call_err("word_wrap", &[s("hello"), n(0.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn word_wrap_no_args_errors() {
    let result = call_err("word_wrap", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn word_wrap_first_arg_non_string_errors() {
    let result = call_err("word_wrap", &[n(42.0), n(10.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn word_wrap_second_arg_non_number_errors() {
    let result = call_err("word_wrap", &[s("hello"), s("x")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}
