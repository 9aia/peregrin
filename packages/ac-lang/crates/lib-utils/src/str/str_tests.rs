use ac_runtime::NonFunctionValue;
use ac_runtime_core::Module;
use ac_vm::{Context, Error};

use super::str::StrModule;

fn call(module: &dyn Module, name: &str, args: &[NonFunctionValue]) -> NonFunctionValue {
    let func = module.function(name).expect("function exists");
    let mut ctx = Context::new();
    func.call(args, &mut ctx).expect("call succeeds")
}

fn call_err(module: &dyn Module, name: &str, args: &[NonFunctionValue]) -> Result<NonFunctionValue, Error> {
    let func = module.function(name).expect("function exists");
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

#[test]
fn module_name() {
    assert_eq!(StrModule.name(), "str");
}

#[test]
fn upper() {
    assert_eq!(
        call(&StrModule, "upper", &[s("hello")]).as_str(),
        Some("HELLO")
    );
}

#[test]
fn lower() {
    assert_eq!(
        call(&StrModule, "lower", &[s("HELLO")]).as_str(),
        Some("hello")
    );
}

#[test]
fn trim() {
    assert_eq!(
        call(&StrModule, "trim", &[s("  hi  ")]).as_str(),
        Some("hi")
    );
}

#[test]
fn split() {
    let result = call(&StrModule, "split", &[s("a,b,c"), s(",")]);
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array[0].as_str(), Some("a"));
    assert_eq!(array[1].as_str(), Some("b"));
    assert_eq!(array[2].as_str(), Some("c"));
}

#[test]
fn join() {
    let result = call(&StrModule, "join", &[arr(&["a", "b", "c"]), s("-")]);
    assert_eq!(result.as_str(), Some("a-b-c"));
}

#[test]
fn replace() {
    let result = call(
        &StrModule,
        "replace",
        &[s("foo bar foo"), s("foo"), s("baz")],
    );
    assert_eq!(result.as_str(), Some("baz bar baz"));
}

#[test]
fn contains_true() {
    assert_eq!(
        call(&StrModule, "contains", &[s("hello world"), s("world")]).as_bool(),
        Some(true)
    );
}

#[test]
fn contains_false() {
    assert_eq!(
        call(&StrModule, "contains", &[s("hello world"), s("xyz")]).as_bool(),
        Some(false)
    );
}

#[test]
fn starts_with_true() {
    assert_eq!(
        call(&StrModule, "starts_with", &[s("hello"), s("he")]).as_bool(),
        Some(true)
    );
}

#[test]
fn starts_with_false() {
    assert_eq!(
        call(&StrModule, "starts_with", &[s("hello"), s("lo")]).as_bool(),
        Some(false)
    );
}

#[test]
fn ends_with_true() {
    assert_eq!(
        call(&StrModule, "ends_with", &[s("hello"), s("lo")]).as_bool(),
        Some(true)
    );
}

#[test]
fn ends_with_false() {
    assert_eq!(
        call(&StrModule, "ends_with", &[s("hello"), s("he")]).as_bool(),
        Some(false)
    );
}

#[test]
fn slice() {
    let result = call(&StrModule, "slice", &[s("hello"), n(1.0), n(4.0)]);
    assert_eq!(result.as_str(), Some("ell"));
}

#[test]
fn slice_out_of_bounds_clamps() {
    let result = call(&StrModule, "slice", &[s("hi"), n(0.0), n(10.0)]);
    assert_eq!(result.as_str(), Some("hi"));
}

#[test]
fn reverse() {
    assert_eq!(
        call(&StrModule, "reverse", &[s("hello")]).as_str(),
        Some("olleh")
    );
}

#[test]
fn trim_start() {
    assert_eq!(
        call(&StrModule, "trim_start", &[s("  hi  ")]).as_str(),
        Some("hi  ")
    );
}

#[test]
fn trim_end() {
    assert_eq!(
        call(&StrModule, "trim_end", &[s("  hi  ")]).as_str(),
        Some("  hi")
    );
}

#[test]
fn repeat() {
    assert_eq!(
        call(&StrModule, "repeat", &[s("ab"), n(3.0)]).as_str(),
        Some("ababab")
    );
}

#[test]
fn repeat_zero() {
    assert_eq!(
        call(&StrModule, "repeat", &[s("ab"), n(0.0)]).as_str(),
        Some("")
    );
}

#[test]
fn find_found() {
    assert_eq!(
        call(&StrModule, "find", &[s("hello world"), s("world")]).as_number(),
        Some(6.0)
    );
}

#[test]
fn find_not_found() {
    assert_eq!(
        call(&StrModule, "find", &[s("hello"), s("xyz")]).as_number(),
        Some(-1.0)
    );
}

#[test]
fn count() {
    assert_eq!(
        call(&StrModule, "count", &[s("foo foo foo"), s("foo")]).as_number(),
        Some(3.0)
    );
}

#[test]
fn count_empty_pattern() {
    assert_eq!(
        call(&StrModule, "count", &[s("hello"), s("")]).as_number(),
        Some(0.0)
    );
}

#[test]
fn char_at() {
    assert_eq!(
        call(&StrModule, "char_at", &[s("hello"), n(1.0)]).as_str(),
        Some("e")
    );
}

#[test]
fn char_at_out_of_bounds() {
    let result = call(&StrModule, "char_at", &[s("hi"), n(10.0)]);
    assert!(result.as_str().is_none());
}

#[test]
fn pad_left() {
    assert_eq!(
        call(&StrModule, "pad_left", &[s("hi"), n(5.0), s(" ")]).as_str(),
        Some("   hi")
    );
}

#[test]
fn pad_left_multi_char() {
    assert_eq!(
        call(&StrModule, "pad_left", &[s("ab"), n(7.0), s("-+")]).as_str(),
        Some("-+-+-ab")
    );
}

#[test]
fn pad_left_already_long_enough() {
    assert_eq!(
        call(&StrModule, "pad_left", &[s("hello"), n(3.0), s(" ")]).as_str(),
        Some("hello")
    );
}

#[test]
fn pad_right() {
    assert_eq!(
        call(&StrModule, "pad_right", &[s("hi"), n(5.0), s(" ")]).as_str(),
        Some("hi   ")
    );
}

#[test]
fn pad_right_multi_char() {
    assert_eq!(
        call(&StrModule, "pad_right", &[s("ab"), n(7.0), s("-+")]).as_str(),
        Some("ab-+-+-")
    );
}

#[test]
fn pad_right_already_long_enough() {
    assert_eq!(
        call(&StrModule, "pad_right", &[s("hello"), n(3.0), s(" ")]).as_str(),
        Some("hello")
    );
}

#[test]
fn length_ascii() {
    assert_eq!(
        call(&StrModule, "length", &[s("hello")]).as_number(),
        Some(5.0)
    );
}

#[test]
fn length_empty() {
    assert_eq!(
        call(&StrModule, "length", &[s("")]).as_number(),
        Some(0.0)
    );
}

#[test]
fn length_unicode() {
    assert_eq!(
        call(&StrModule, "length", &[s("héllo")]).as_number(),
        Some(5.0)
    );
}

#[test]
fn concat_two() {
    assert_eq!(
        call(&StrModule, "concat", &[s("hello"), s(" world")]).as_str(),
        Some("hello world")
    );
}

#[test]
fn concat_three() {
    assert_eq!(
        call(&StrModule, "concat", &[s("a"), s("b"), s("c")]).as_str(),
        Some("abc")
    );
}

#[test]
fn concat_one_arg_errors() {
    let func = StrModule.function("concat").unwrap();
    let mut ctx = Context::new();
    let result = func.call(&[s("a")], &mut ctx);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn replace_first() {
    assert_eq!(
        call(
            &StrModule,
            "replace_first",
            &[s("foo bar foo"), s("foo"), s("baz")]
        )
        .as_str(),
        Some("baz bar foo")
    );
}

#[test]
fn replace_first_no_match() {
    assert_eq!(
        call(
            &StrModule,
            "replace_first",
            &[s("hello"), s("xyz"), s("abc")]
        )
        .as_str(),
        Some("hello")
    );
}

#[test]
fn slice_unicode() {
    let result = call(&StrModule, "slice", &[s("héllo"), n(1.0), n(4.0)]);
    assert_eq!(result.as_str(), Some("éll"));
}

#[test]
fn find_unicode_returns_char_offset() {
    let result = call(&StrModule, "find", &[s("héllo"), s("lo")]);
    assert_eq!(result.as_number(), Some(3.0));
}

#[test]
fn char_at_negative_errors() {
    let func = StrModule.function("char_at").unwrap();
    let mut ctx = Context::new();
    let result = func.call(&[s("hello"), n(-1.0)], &mut ctx);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn slice_negative_errors() {
    let func = StrModule.function("slice").unwrap();
    let mut ctx = Context::new();
    let result = func.call(&[s("hello"), n(-1.0), n(3.0)], &mut ctx);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn repeat_negative_errors() {
    let func = StrModule.function("repeat").unwrap();
    let mut ctx = Context::new();
    let result = func.call(&[s("ab"), n(-1.0)], &mut ctx);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn pad_left_negative_errors() {
    let func = StrModule.function("pad_left").unwrap();
    let mut ctx = Context::new();
    let result = func.call(&[s("hi"), n(-1.0), s(" ")], &mut ctx);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn pad_left_unicode() {
    assert_eq!(
        call(&StrModule, "pad_left", &[s("café"), n(6.0), s(" ")]).as_str(),
        Some("  café")
    );
}

#[test]
fn unknown_function_returns_none() {
    assert!(StrModule.function("nope").is_none());
}

#[test]
fn type_mismatch_on_non_string() {
    let func = StrModule.function("upper").unwrap();
    let mut ctx = Context::new();
    let result = func.call(&[n(42.0)], &mut ctx);
    assert_eq!(
        result,
        Err(Error::TypeMismatch {
            expected: "string",
            got: "non-string",
        })
    );
}

#[test]
fn argument_count_on_missing_args() {
    let func = StrModule.function("upper").unwrap();
    let mut ctx = Context::new();
    let result = func.call(&[], &mut ctx);
    assert_eq!(
        result,
        Err(Error::ArgumentCount {
            expected: 1,
            got: 0,
        })
    );
}

// ── additional edge-case tests ─────────────────────────────────────

// Unary functions: missing args and type mismatch for all

#[test]
fn lower_no_args_errors() {
    let result = call_err(&StrModule, "lower", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn lower_non_string_errors() {
    let result = call_err(&StrModule, "lower", &[n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn trim_no_args_errors() {
    let result = call_err(&StrModule, "trim", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn trim_non_string_errors() {
    let result = call_err(&StrModule, "trim", &[n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn trim_start_no_args_errors() {
    let result = call_err(&StrModule, "trim_start", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn trim_start_non_string_errors() {
    let result = call_err(&StrModule, "trim_start", &[n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn trim_end_no_args_errors() {
    let result = call_err(&StrModule, "trim_end", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn trim_end_non_string_errors() {
    let result = call_err(&StrModule, "trim_end", &[n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn reverse_no_args_errors() {
    let result = call_err(&StrModule, "reverse", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn reverse_non_string_errors() {
    let result = call_err(&StrModule, "reverse", &[n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn reverse_empty_string() {
    assert_eq!(call(&StrModule, "reverse", &[s("")]).as_str(), Some(""));
}

#[test]
fn reverse_unicode() {
    assert_eq!(call(&StrModule, "reverse", &[s("héllo")]).as_str(), Some("olléh"));
}

#[test]
fn length_no_args_errors() {
    let result = call_err(&StrModule, "length", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn length_non_string_errors() {
    let result = call_err(&StrModule, "length", &[n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn length_empty_string() {
    assert_eq!(call(&StrModule, "length", &[s("")]).as_number(), Some(0.0));
}

#[test]
fn length_unicode_edge() {
    assert_eq!(call(&StrModule, "length", &[s("café")]).as_number(), Some(4.0));
}

// Split: 2-arg function

#[test]
fn split_no_args_errors() {
    let result = call_err(&StrModule, "split", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn split_one_arg_errors() {
    let result = call_err(&StrModule, "split", &[s("a,b,c")]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn split_first_arg_non_string_errors() {
    let result = call_err(&StrModule, "split", &[n(42.0), s(",")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn split_second_arg_non_string_errors() {
    let result = call_err(&StrModule, "split", &[s("a,b"), n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn split_empty_string() {
    let result = call(&StrModule, "split", &[s(""), s(",")]);
    assert_eq!(result.as_array().unwrap().len(), 1);
}

#[test]
fn split_no_match() {
    let result = call(&StrModule, "split", &[s("hello"), s(",")]);
    assert_eq!(result.as_array().unwrap().len(), 1);
    assert_eq!(result.as_array().unwrap()[0].as_str(), Some("hello"));
}

#[test]
fn split_empty_delimiter() {
    let result = call(&StrModule, "split", &[s("abc"), s("")]);
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 5);
    assert_eq!(arr[0].as_str(), Some(""));
    assert_eq!(arr[4].as_str(), Some(""));
}

// Join: 2-arg function

#[test]
fn join_no_args_errors() {
    let result = call_err(&StrModule, "join", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn join_one_arg_errors() {
    let result = call_err(&StrModule, "join", &[arr(&["a", "b"])]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn join_first_arg_non_array_errors() {
    let result = call_err(&StrModule, "join", &[s("hello"), s(",")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn join_second_arg_non_string_errors() {
    let result = call_err(&StrModule, "join", &[arr(&["a", "b"]), n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn join_empty_array() {
    assert_eq!(call(&StrModule, "join", &[arr(&[]), s(",")]).as_str(), Some(""));
}

#[test]
fn join_with_non_string_elements() {
    let json_arr = NonFunctionValue::new(serde_json::json!([1, 2, 3]));
    assert_eq!(call(&StrModule, "join", &[json_arr, s(",")]).as_str(), Some(""));
}

// Replace: 3-arg function

#[test]
fn replace_no_args_errors() {
    let result = call_err(&StrModule, "replace", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn replace_two_args_errors() {
    let result = call_err(&StrModule, "replace", &[s("hello"), s("l")]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn replace_first_arg_non_string_errors() {
    let result = call_err(&StrModule, "replace", &[n(42.0), s("l"), s("L")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn replace_second_arg_non_string_errors() {
    let result = call_err(&StrModule, "replace", &[s("hello"), n(42.0), s("L")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn replace_third_arg_non_string_errors() {
    let result = call_err(&StrModule, "replace", &[s("hello"), s("l"), n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn replace_no_match() {
    assert_eq!(call(&StrModule, "replace", &[s("hello"), s("z"), s("x")]).as_str(), Some("hello"));
}

#[test]
fn replace_empty_string() {
    assert_eq!(call(&StrModule, "replace", &[s(""), s("a"), s("b")]).as_str(), Some(""));
}

// ReplaceFirst: 3-arg function

#[test]
fn replace_first_no_args_errors() {
    let result = call_err(&StrModule, "replace_first", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn replace_first_two_args_errors() {
    let result = call_err(&StrModule, "replace_first", &[s("hello"), s("l")]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn replace_first_first_arg_non_string_errors() {
    let result = call_err(&StrModule, "replace_first", &[n(42.0), s("l"), s("L")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn replace_first_second_arg_non_string_errors() {
    let result = call_err(&StrModule, "replace_first", &[s("hello"), n(42.0), s("L")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn replace_first_third_arg_non_string_errors() {
    let result = call_err(&StrModule, "replace_first", &[s("hello"), s("l"), n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn replace_first_no_match_edge() {
    assert_eq!(call(&StrModule, "replace_first", &[s("hello"), s("z"), s("x")]).as_str(), Some("hello"));
}

#[test]
fn replace_first_only_first() {
    assert_eq!(call(&StrModule, "replace_first", &[s("hello"), s("l"), s("L")]).as_str(), Some("heLlo"));
}

// Contains, starts_with, ends_with: 2-arg predicate functions

#[test]
fn contains_no_args_errors() {
    let result = call_err(&StrModule, "contains", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn contains_one_arg_errors() {
    let result = call_err(&StrModule, "contains", &[s("hello")]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn contains_first_arg_non_string_errors() {
    let result = call_err(&StrModule, "contains", &[n(42.0), s("ell")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn contains_second_arg_non_string_errors() {
    let result = call_err(&StrModule, "contains", &[s("hello"), n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn contains_empty_pattern() {
    assert_eq!(call(&StrModule, "contains", &[s("hello"), s("")]).as_bool(), Some(true));
}

#[test]
fn starts_with_no_args_errors() {
    let result = call_err(&StrModule, "starts_with", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn starts_with_first_arg_non_string_errors() {
    let result = call_err(&StrModule, "starts_with", &[n(42.0), s("he")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn starts_with_second_arg_non_string_errors() {
    let result = call_err(&StrModule, "starts_with", &[s("hello"), n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn ends_with_no_args_errors() {
    let result = call_err(&StrModule, "ends_with", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn ends_with_first_arg_non_string_errors() {
    let result = call_err(&StrModule, "ends_with", &[n(42.0), s("lo")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn ends_with_second_arg_non_string_errors() {
    let result = call_err(&StrModule, "ends_with", &[s("hello"), n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// Count: 2-arg function

#[test]
fn count_no_args_errors() {
    let result = call_err(&StrModule, "count", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn count_first_arg_non_string_errors() {
    let result = call_err(&StrModule, "count", &[n(42.0), s("l")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn count_second_arg_non_string_errors() {
    let result = call_err(&StrModule, "count", &[s("hello"), n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn count_empty_pattern_returns_zero() {
    assert_eq!(call(&StrModule, "count", &[s("hello"), s("")]).as_number(), Some(0.0));
}

#[test]
fn count_no_match() {
    assert_eq!(call(&StrModule, "count", &[s("hello"), s("z")]).as_number(), Some(0.0));
}

// Find: 2-arg function

#[test]
fn find_no_args_errors() {
    let result = call_err(&StrModule, "find", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn find_first_arg_non_string_errors() {
    let result = call_err(&StrModule, "find", &[n(42.0), s("l")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn find_second_arg_non_string_errors() {
    let result = call_err(&StrModule, "find", &[s("hello"), n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn find_no_match_returns_minus_one() {
    assert_eq!(call(&StrModule, "find", &[s("hello"), s("z")]).as_number(), Some(-1.0));
}

#[test]
fn find_empty_pattern() {
    assert_eq!(call(&StrModule, "find", &[s("hello"), s("")]).as_number(), Some(0.0));
}

#[test]
fn find_unicode() {
    assert_eq!(call(&StrModule, "find", &[s("café world"), s("world")]).as_number(), Some(5.0));
}

// CharAt: 2-arg function

#[test]
fn char_at_no_args_errors() {
    let result = call_err(&StrModule, "char_at", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn char_at_first_arg_non_string_errors() {
    let result = call_err(&StrModule, "char_at", &[n(42.0), n(0.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn char_at_second_arg_non_number_errors() {
    let result = call_err(&StrModule, "char_at", &[s("hello"), s("x")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn char_at_out_of_bounds_returns_null() {
    let result = call(&StrModule, "char_at", &[s("hi"), n(10.0)]);
    assert!(result.is_null());
}

#[test]
fn char_at_unicode() {
    assert_eq!(call(&StrModule, "char_at", &[s("café"), n(3.0)]).as_str(), Some("é"));
}

// Repeat: 2-arg function

#[test]
fn repeat_no_args_errors() {
    let result = call_err(&StrModule, "repeat", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn repeat_first_arg_non_string_errors() {
    let result = call_err(&StrModule, "repeat", &[n(42.0), n(3.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn repeat_second_arg_non_number_errors() {
    let result = call_err(&StrModule, "repeat", &[s("ab"), s("x")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn repeat_zero_times() {
    assert_eq!(call(&StrModule, "repeat", &[s("ab"), n(0.0)]).as_str(), Some(""));
}

#[test]
fn repeat_empty_string() {
    assert_eq!(call(&StrModule, "repeat", &[s(""), n(5.0)]).as_str(), Some(""));
}

// Slice: 3-arg function

#[test]
fn slice_no_args_errors() {
    let result = call_err(&StrModule, "slice", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn slice_two_args_errors() {
    let result = call_err(&StrModule, "slice", &[s("hello"), n(0.0)]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn slice_first_arg_non_string_errors() {
    let result = call_err(&StrModule, "slice", &[n(42.0), n(0.0), n(3.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn slice_negative_start_errors() {
    let result = call_err(&StrModule, "slice", &[s("hello"), n(-1.0), n(3.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn slice_start_beyond_length() {
    assert_eq!(call(&StrModule, "slice", &[s("hi"), n(10.0), n(15.0)]).as_str(), Some(""));
}

#[test]
fn slice_empty_string() {
    assert_eq!(call(&StrModule, "slice", &[s(""), n(0.0), n(5.0)]).as_str(), Some(""));
}

#[test]
fn slice_unicode_edge() {
    assert_eq!(call(&StrModule, "slice", &[s("café world"), n(0.0), n(4.0)]).as_str(), Some("café"));
}

// Concat: variadic function

#[test]
fn concat_no_args_errors() {
    let result = call_err(&StrModule, "concat", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn concat_single_arg_errors() {
    let result = call_err(&StrModule, "concat", &[s("a")]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn concat_non_string_arg_errors() {
    let result = call_err(&StrModule, "concat", &[s("a"), n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn concat_empty_strings() {
    assert_eq!(call(&StrModule, "concat", &[s(""), s("")]).as_str(), Some(""));
}

#[test]
fn concat_three_strings() {
    assert_eq!(call(&StrModule, "concat", &[s("a"), s("b"), s("c")]).as_str(), Some("abc"));
}

// PadLeft / PadRight: 3-arg functions

#[test]
fn pad_left_no_args_errors() {
    let result = call_err(&StrModule, "pad_left", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn pad_left_two_args_errors() {
    let result = call_err(&StrModule, "pad_left", &[s("hi"), n(5.0)]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn pad_left_first_arg_non_string_errors() {
    let result = call_err(&StrModule, "pad_left", &[n(42.0), n(5.0), s(" ")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn pad_left_second_arg_non_number_errors() {
    let result = call_err(&StrModule, "pad_left", &[s("hi"), s("x"), s(" ")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn pad_left_third_arg_non_string_errors() {
    let result = call_err(&StrModule, "pad_left", &[s("hi"), n(5.0), n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn pad_left_empty_pad_char() {
    assert_eq!(call(&StrModule, "pad_left", &[s("hi"), n(5.0), s("")]).as_str(), Some("hi"));
}

#[test]
fn pad_left_already_long_enough_extra() {
    assert_eq!(call(&StrModule, "pad_left", &[s("hello"), n(3.0), s(" ")]).as_str(), Some("hello"));
}

#[test]
fn pad_left_multi_char_pad() {
    assert_eq!(call(&StrModule, "pad_left", &[s("hi"), n(6.0), s("ab")]).as_str(), Some("ababhi"));
}

#[test]
fn pad_right_no_args_errors() {
    let result = call_err(&StrModule, "pad_right", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn pad_right_two_args_errors() {
    let result = call_err(&StrModule, "pad_right", &[s("hi"), n(5.0)]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn pad_right_first_arg_non_string_errors() {
    let result = call_err(&StrModule, "pad_right", &[n(42.0), n(5.0), s(" ")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn pad_right_second_arg_non_number_errors() {
    let result = call_err(&StrModule, "pad_right", &[s("hi"), s("x"), s(" ")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn pad_right_third_arg_non_string_errors() {
    let result = call_err(&StrModule, "pad_right", &[s("hi"), n(5.0), n(42.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn pad_right_negative_errors() {
    let result = call_err(&StrModule, "pad_right", &[s("hi"), n(-1.0), s(" ")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn pad_right_empty_pad_char() {
    assert_eq!(call(&StrModule, "pad_right", &[s("hi"), n(5.0), s("")]).as_str(), Some("hi"));
}

#[test]
fn pad_right_already_long_enough_extra() {
    assert_eq!(call(&StrModule, "pad_right", &[s("hello"), n(3.0), s(" ")]).as_str(), Some("hello"));
}

#[test]
fn pad_right_unicode() {
    assert_eq!(call(&StrModule, "pad_right", &[s("café"), n(6.0), s(" ")]).as_str(), Some("café  "));
}

// Upper/Lower edge cases

#[test]
fn upper_empty_string() {
    assert_eq!(call(&StrModule, "upper", &[s("")]).as_str(), Some(""));
}

#[test]
fn upper_unicode() {
    assert_eq!(call(&StrModule, "upper", &[s("héllo")]).as_str(), Some("HÉLLO"));
}

#[test]
fn lower_empty_string() {
    assert_eq!(call(&StrModule, "lower", &[s("")]).as_str(), Some(""));
}

#[test]
fn lower_unicode() {
    assert_eq!(call(&StrModule, "lower", &[s("HÉLLO")]).as_str(), Some("héllo"));
}

// Trim edge cases

#[test]
fn trim_empty_string() {
    assert_eq!(call(&StrModule, "trim", &[s("")]).as_str(), Some(""));
}

#[test]
fn trim_no_whitespace() {
    assert_eq!(call(&StrModule, "trim", &[s("hello")]).as_str(), Some("hello"));
}

#[test]
fn trim_all_whitespace() {
    assert_eq!(call(&StrModule, "trim", &[s("   ")]).as_str(), Some(""));
}

#[test]
fn trim_start_empty_string() {
    assert_eq!(call(&StrModule, "trim_start", &[s("")]).as_str(), Some(""));
}

#[test]
fn trim_end_empty_string() {
    assert_eq!(call(&StrModule, "trim_end", &[s("")]).as_str(), Some(""));
}
