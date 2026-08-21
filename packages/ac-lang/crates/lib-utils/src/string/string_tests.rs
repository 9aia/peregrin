use ac_runtime::NonFunctionValue;
use ac_runtime_core::Module;
use ac_vm::{Context, Error};

use super::string::StringModule;

fn call(module: &dyn Module, name: &str, args: &[NonFunctionValue]) -> NonFunctionValue {
    let func = module.function(name).expect("function exists");
    let mut ctx = Context::new();
    func.call(args, &mut ctx).expect("call succeeds")
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
    assert_eq!(StringModule.name(), "string");
}

#[test]
fn upper() {
    assert_eq!(
        call(&StringModule, "upper", &[s("hello")]).as_str(),
        Some("HELLO")
    );
}

#[test]
fn lower() {
    assert_eq!(
        call(&StringModule, "lower", &[s("HELLO")]).as_str(),
        Some("hello")
    );
}

#[test]
fn trim() {
    assert_eq!(
        call(&StringModule, "trim", &[s("  hi  ")]).as_str(),
        Some("hi")
    );
}

#[test]
fn split() {
    let result = call(&StringModule, "split", &[s("a,b,c"), s(",")]);
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array[0].as_str(), Some("a"));
    assert_eq!(array[1].as_str(), Some("b"));
    assert_eq!(array[2].as_str(), Some("c"));
}

#[test]
fn join() {
    let result = call(&StringModule, "join", &[arr(&["a", "b", "c"]), s("-")]);
    assert_eq!(result.as_str(), Some("a-b-c"));
}

#[test]
fn replace() {
    let result = call(
        &StringModule,
        "replace",
        &[s("foo bar foo"), s("foo"), s("baz")],
    );
    assert_eq!(result.as_str(), Some("baz bar baz"));
}

#[test]
fn contains_true() {
    assert_eq!(
        call(&StringModule, "contains", &[s("hello world"), s("world")]).as_bool(),
        Some(true)
    );
}

#[test]
fn contains_false() {
    assert_eq!(
        call(&StringModule, "contains", &[s("hello world"), s("xyz")]).as_bool(),
        Some(false)
    );
}

#[test]
fn starts_with_true() {
    assert_eq!(
        call(&StringModule, "starts_with", &[s("hello"), s("he")]).as_bool(),
        Some(true)
    );
}

#[test]
fn starts_with_false() {
    assert_eq!(
        call(&StringModule, "starts_with", &[s("hello"), s("lo")]).as_bool(),
        Some(false)
    );
}

#[test]
fn ends_with_true() {
    assert_eq!(
        call(&StringModule, "ends_with", &[s("hello"), s("lo")]).as_bool(),
        Some(true)
    );
}

#[test]
fn ends_with_false() {
    assert_eq!(
        call(&StringModule, "ends_with", &[s("hello"), s("he")]).as_bool(),
        Some(false)
    );
}

#[test]
fn slice() {
    let result = call(&StringModule, "slice", &[s("hello"), n(1.0), n(4.0)]);
    assert_eq!(result.as_str(), Some("ell"));
}

#[test]
fn slice_out_of_bounds_clamps() {
    let result = call(&StringModule, "slice", &[s("hi"), n(0.0), n(10.0)]);
    assert_eq!(result.as_str(), Some("hi"));
}

#[test]
fn reverse() {
    assert_eq!(
        call(&StringModule, "reverse", &[s("hello")]).as_str(),
        Some("olleh")
    );
}

#[test]
fn trim_start() {
    assert_eq!(
        call(&StringModule, "trim_start", &[s("  hi  ")]).as_str(),
        Some("hi  ")
    );
}

#[test]
fn trim_end() {
    assert_eq!(
        call(&StringModule, "trim_end", &[s("  hi  ")]).as_str(),
        Some("  hi")
    );
}

#[test]
fn repeat() {
    assert_eq!(
        call(&StringModule, "repeat", &[s("ab"), n(3.0)]).as_str(),
        Some("ababab")
    );
}

#[test]
fn repeat_zero() {
    assert_eq!(
        call(&StringModule, "repeat", &[s("ab"), n(0.0)]).as_str(),
        Some("")
    );
}

#[test]
fn find_found() {
    assert_eq!(
        call(&StringModule, "find", &[s("hello world"), s("world")]).as_number(),
        Some(6.0)
    );
}

#[test]
fn find_not_found() {
    assert_eq!(
        call(&StringModule, "find", &[s("hello"), s("xyz")]).as_number(),
        Some(-1.0)
    );
}

#[test]
fn count() {
    assert_eq!(
        call(&StringModule, "count", &[s("foo foo foo"), s("foo")]).as_number(),
        Some(3.0)
    );
}

#[test]
fn count_empty_pattern() {
    assert_eq!(
        call(&StringModule, "count", &[s("hello"), s("")]).as_number(),
        Some(0.0)
    );
}

#[test]
fn char_at() {
    assert_eq!(
        call(&StringModule, "char_at", &[s("hello"), n(1.0)]).as_str(),
        Some("e")
    );
}

#[test]
fn char_at_out_of_bounds() {
    let result = call(&StringModule, "char_at", &[s("hi"), n(10.0)]);
    assert!(result.as_str().is_none());
}

#[test]
fn pad_left() {
    assert_eq!(
        call(&StringModule, "pad_left", &[s("hi"), n(5.0), s(" ")]).as_str(),
        Some("   hi")
    );
}

#[test]
fn pad_left_multi_char() {
    assert_eq!(
        call(&StringModule, "pad_left", &[s("ab"), n(7.0), s("-+")]).as_str(),
        Some("-+-+-ab")
    );
}

#[test]
fn pad_left_already_long_enough() {
    assert_eq!(
        call(&StringModule, "pad_left", &[s("hello"), n(3.0), s(" ")]).as_str(),
        Some("hello")
    );
}

#[test]
fn pad_right() {
    assert_eq!(
        call(&StringModule, "pad_right", &[s("hi"), n(5.0), s(" ")]).as_str(),
        Some("hi   ")
    );
}

#[test]
fn pad_right_multi_char() {
    assert_eq!(
        call(&StringModule, "pad_right", &[s("ab"), n(7.0), s("-+")]).as_str(),
        Some("ab-+-+-")
    );
}

#[test]
fn pad_right_already_long_enough() {
    assert_eq!(
        call(&StringModule, "pad_right", &[s("hello"), n(3.0), s(" ")]).as_str(),
        Some("hello")
    );
}

#[test]
fn length_ascii() {
    assert_eq!(
        call(&StringModule, "length", &[s("hello")]).as_number(),
        Some(5.0)
    );
}

#[test]
fn length_empty() {
    assert_eq!(
        call(&StringModule, "length", &[s("")]).as_number(),
        Some(0.0)
    );
}

#[test]
fn length_unicode() {
    assert_eq!(
        call(&StringModule, "length", &[s("héllo")]).as_number(),
        Some(5.0)
    );
}

#[test]
fn concat_two() {
    assert_eq!(
        call(&StringModule, "concat", &[s("hello"), s(" world")]).as_str(),
        Some("hello world")
    );
}

#[test]
fn concat_three() {
    assert_eq!(
        call(&StringModule, "concat", &[s("a"), s("b"), s("c")]).as_str(),
        Some("abc")
    );
}

#[test]
fn concat_one_arg_errors() {
    let func = StringModule.function("concat").unwrap();
    let mut ctx = Context::new();
    let result = func.call(&[s("a")], &mut ctx);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn replace_first() {
    assert_eq!(
        call(
            &StringModule,
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
            &StringModule,
            "replace_first",
            &[s("hello"), s("xyz"), s("abc")]
        )
        .as_str(),
        Some("hello")
    );
}

#[test]
fn slice_unicode() {
    let result = call(&StringModule, "slice", &[s("héllo"), n(1.0), n(4.0)]);
    assert_eq!(result.as_str(), Some("éll"));
}

#[test]
fn find_unicode_returns_char_offset() {
    let result = call(&StringModule, "find", &[s("héllo"), s("lo")]);
    assert_eq!(result.as_number(), Some(3.0));
}

#[test]
fn char_at_negative_errors() {
    let func = StringModule.function("char_at").unwrap();
    let mut ctx = Context::new();
    let result = func.call(&[s("hello"), n(-1.0)], &mut ctx);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn slice_negative_errors() {
    let func = StringModule.function("slice").unwrap();
    let mut ctx = Context::new();
    let result = func.call(&[s("hello"), n(-1.0), n(3.0)], &mut ctx);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn repeat_negative_errors() {
    let func = StringModule.function("repeat").unwrap();
    let mut ctx = Context::new();
    let result = func.call(&[s("ab"), n(-1.0)], &mut ctx);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn pad_left_negative_errors() {
    let func = StringModule.function("pad_left").unwrap();
    let mut ctx = Context::new();
    let result = func.call(&[s("hi"), n(-1.0), s(" ")], &mut ctx);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn pad_left_unicode() {
    assert_eq!(
        call(&StringModule, "pad_left", &[s("café"), n(6.0), s(" ")]).as_str(),
        Some("  café")
    );
}

#[test]
fn unknown_function_returns_none() {
    assert!(StringModule.function("nope").is_none());
}

#[test]
fn type_mismatch_on_non_string() {
    let func = StringModule.function("upper").unwrap();
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
    let func = StringModule.function("upper").unwrap();
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
