use ac_runtime_canonical::CanonicalContext;
use ac_runtime_modules::Module;
use ac_runtime_values::Error;
use ac_runtime_values::NonFunctionValue;

use crate::common::{byte_array, num, s};
use crate::BytesModule;

fn ba(values: &[u8]) -> NonFunctionValue {
    byte_array(values)
}

fn call(name: &str, args: &[NonFunctionValue]) -> NonFunctionValue {
    let func = BytesModule.function(name).expect("function exists");
    let mut ctx = CanonicalContext::new();
    func.call(args, &mut ctx).expect("call succeeds")
}

fn call_err(name: &str, args: &[NonFunctionValue]) -> Result<NonFunctionValue, Error> {
    let func = BytesModule.function(name).expect("function exists");
    let mut ctx = CanonicalContext::new();
    func.call(args, &mut ctx)
}

#[test]
fn module_name() {
    assert_eq!(BytesModule.name(), "bytes");
}

#[test]
fn unknown_function_returns_none() {
    assert!(BytesModule.function("nope").is_none());
}

#[test]
fn length_of_array() {
    let result = call("length", &[ba(&[1, 2, 3])]);
    assert_eq!(result.as_number(), Some(3.0));
}

#[test]
fn length_of_string() {
    let result = call("length", &[s("hello")]);
    assert_eq!(result.as_number(), Some(5.0));
}

#[test]
fn is_empty_true() {
    let result = call("is_empty", &[ba(&[])]);
    assert_eq!(result.as_bool(), Some(true));
}

#[test]
fn is_empty_false() {
    let result = call("is_empty", &[ba(&[1])]);
    assert_eq!(result.as_bool(), Some(false));
}

#[test]
fn get_valid() {
    let result = call("get", &[ba(&[10, 20, 30]), num(1.0)]);
    assert_eq!(result.as_number(), Some(20.0));
}

#[test]
fn get_out_of_bounds() {
    let result = call("get", &[ba(&[10, 20]), num(5.0)]);
    assert!(result.is_null());
}

#[test]
fn set_valid() {
    let result = call("set", &[ba(&[10, 20, 30]), num(1.0), num(99.0)]);
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 3);
    assert_eq!(arr[0].as_f64(), Some(10.0));
    assert_eq!(arr[1].as_f64(), Some(99.0));
    assert_eq!(arr[2].as_f64(), Some(30.0));
}

#[test]
fn set_out_of_bounds_errors() {
    let result = call_err("set", &[ba(&[1, 2]), num(5.0), num(9.0)]);
    assert!(result.is_err());
}

#[test]
fn slice_basic() {
    let result = call("slice", &[ba(&[1, 2, 3, 4, 5]), num(1.0), num(4.0)]);
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 3);
    assert_eq!(arr[0].as_f64(), Some(2.0));
    assert_eq!(arr[1].as_f64(), Some(3.0));
    assert_eq!(arr[2].as_f64(), Some(4.0));
}

#[test]
fn slice_clamps() {
    let result = call("slice", &[ba(&[1, 2, 3]), num(1.0), num(100.0)]);
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 2);
}

#[test]
fn slice_start_after_end() {
    let result = call("slice", &[ba(&[1, 2, 3]), num(2.0), num(1.0)]);
    let arr = result.as_array().unwrap();
    assert!(arr.is_empty());
}

#[test]
fn concat_arrays() {
    let result = call("concat", &[ba(&[1, 2]), ba(&[3, 4])]);
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 4);
    assert_eq!(arr[3].as_f64(), Some(4.0));
}

#[test]
fn concat_strings() {
    let result = call("concat", &[s("ab"), s("cd")]);
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 4);
    assert_eq!(arr[2].as_f64(), Some(99.0)); // 'c'
    assert_eq!(arr[3].as_f64(), Some(100.0)); // 'd'
}

#[test]
fn concat_too_few_args_errors() {
    let result = call_err("concat", &[ba(&[1])]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn reverse_basic() {
    let result = call("reverse", &[ba(&[1, 2, 3])]);
    let arr = result.as_array().unwrap();
    assert_eq!(arr[0].as_f64(), Some(3.0));
    assert_eq!(arr[2].as_f64(), Some(1.0));
}

#[test]
fn index_of_found() {
    let result = call("index_of", &[ba(&[1, 2, 3, 2, 1]), num(2.0)]);
    assert_eq!(result.as_number(), Some(1.0));
}

#[test]
fn index_of_not_found() {
    let result = call("index_of", &[ba(&[1, 2, 3]), num(7.0)]);
    assert_eq!(result.as_number(), Some(-1.0));
}

#[test]
fn find_found() {
    let result = call("find", &[ba(&[1, 2, 3, 4, 5]), ba(&[3, 4])]);
    assert_eq!(result.as_number(), Some(2.0));
}

#[test]
fn find_not_found() {
    let result = call("find", &[ba(&[1, 2, 3]), ba(&[4, 5])]);
    assert_eq!(result.as_number(), Some(-1.0));
}

#[test]
fn find_empty_needle() {
    let result = call("find", &[ba(&[1, 2, 3]), ba(&[])]);
    assert_eq!(result.as_number(), Some(0.0));
}

#[test]
fn contains_number() {
    let result = call("contains", &[ba(&[1, 2, 3]), num(2.0)]);
    assert_eq!(result.as_bool(), Some(true));
}

#[test]
fn contains_subarray() {
    let result = call("contains", &[ba(&[1, 2, 3, 4]), ba(&[2, 3])]);
    assert_eq!(result.as_bool(), Some(true));
}

#[test]
fn contains_false() {
    let result = call("contains", &[ba(&[1, 2, 3]), num(5.0)]);
    assert_eq!(result.as_bool(), Some(false));
}

#[test]
fn starts_with_true() {
    let result = call("starts_with", &[ba(&[1, 2, 3]), ba(&[1, 2])]);
    assert_eq!(result.as_bool(), Some(true));
}

#[test]
fn starts_with_false() {
    let result = call("starts_with", &[ba(&[1, 2, 3]), ba(&[2, 3])]);
    assert_eq!(result.as_bool(), Some(false));
}

#[test]
fn ends_with_true() {
    let result = call("ends_with", &[ba(&[1, 2, 3]), ba(&[2, 3])]);
    assert_eq!(result.as_bool(), Some(true));
}

#[test]
fn ends_with_false() {
    let result = call("ends_with", &[ba(&[1, 2, 3]), ba(&[1, 2])]);
    assert_eq!(result.as_bool(), Some(false));
}

#[test]
fn equals_true() {
    let result = call("equals", &[ba(&[1, 2, 3]), ba(&[1, 2, 3])]);
    assert_eq!(result.as_bool(), Some(true));
}

#[test]
fn equals_false() {
    let result = call("equals", &[ba(&[1, 2, 3]), ba(&[1, 2])]);
    assert_eq!(result.as_bool(), Some(false));
}

#[test]
fn count_number() {
    let result = call("count", &[ba(&[1, 2, 2, 3, 2]), num(2.0)]);
    assert_eq!(result.as_number(), Some(3.0));
}

#[test]
fn count_subarray() {
    let result = call("count", &[ba(&[1, 2, 1, 2, 1, 2]), ba(&[1, 2])]);
    assert_eq!(result.as_number(), Some(3.0));
}

#[test]
fn sum_basic() {
    let result = call("sum", &[ba(&[1, 2, 3])]);
    assert_eq!(result.as_number(), Some(6.0));
}

#[test]
fn sum_empty() {
    let result = call("sum", &[ba(&[])]);
    assert_eq!(result.as_number(), Some(0.0));
}

#[test]
fn min_max_avg() {
    let result = call("min", &[ba(&[3, 1, 4, 1, 5])]);
    assert_eq!(result.as_number(), Some(1.0));
    let result = call("max", &[ba(&[3, 1, 4, 1, 5])]);
    assert_eq!(result.as_number(), Some(5.0));
    let result = call("avg", &[ba(&[2, 4, 6])]);
    assert_eq!(result.as_number(), Some(4.0));
}

#[test]
fn min_empty_returns_null() {
    let result = call("min", &[ba(&[])]);
    assert!(result.is_null());
}

#[test]
fn sort_basic() {
    let result = call("sort", &[ba(&[3, 1, 4, 1, 5])]);
    let arr = result.as_array().unwrap();
    assert_eq!(arr[0].as_f64(), Some(1.0));
    assert_eq!(arr[4].as_f64(), Some(5.0));
}

#[test]
fn from_string_basic() {
    let result = call("from_string", &[s("abc")]);
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 3);
    assert_eq!(arr[0].as_f64(), Some(97.0));
    assert_eq!(arr[1].as_f64(), Some(98.0));
    assert_eq!(arr[2].as_f64(), Some(99.0));
}

#[test]
fn from_string_non_string_errors() {
    let result = call_err("from_string", &[num(1.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn to_string_from_array() {
    let result = call("to_string", &[ba(&[104, 101, 108, 108, 111])]);
    assert_eq!(result.as_str(), Some("hello"));
}

#[test]
fn to_string_from_string() {
    let result = call("to_string", &[s("hello")]);
    assert_eq!(result.as_str(), Some("hello"));
}

#[test]
fn to_string_invalid_utf8_errors() {
    let result = call_err("to_string", &[ba(&[0, 159, 255])]);
    assert!(result.is_err());
}

#[test]
fn encode_hex() {
    let result = call("encode_hex", &[ba(&[0, 255, 16])]);
    assert_eq!(result.as_str(), Some("00ff10"));
}

#[test]
fn decode_hex() {
    let result = call("decode_hex", &[s("00ff10")]);
    let arr = result.as_array().unwrap();
    assert_eq!(arr[0].as_f64(), Some(0.0));
    assert_eq!(arr[1].as_f64(), Some(255.0));
    assert_eq!(arr[2].as_f64(), Some(16.0));
}

#[test]
fn decode_hex_invalid_errors() {
    let result = call_err("decode_hex", &[s("0g")]);
    assert!(result.is_err());
}

#[test]
fn encode_base64() {
    let result = call("encode_base64", &[ba(&[104, 101, 108, 108, 111])]);
    assert_eq!(result.as_str(), Some("aGVsbG8="));
}

#[test]
fn decode_base64() {
    let result = call("decode_base64", &[s("aGVsbG8=")]);
    let arr = result.as_array().unwrap();
    assert_eq!(arr[0].as_f64(), Some(104.0));
    assert_eq!(arr[4].as_f64(), Some(111.0));
}

#[test]
fn decode_base64_invalid_errors() {
    let result = call_err("decode_base64", &[s("!@#")]);
    assert!(result.is_err());
}

// ── error paths ───────────────────────────────────────────────────

#[test]
fn argument_count_errors() {
    assert!(matches!(call_err("length", &[]), Err(Error::ArgumentCount { .. })));
    assert!(matches!(call_err("get", &[ba(&[1])]), Err(Error::ArgumentCount { .. })));
    assert!(matches!(call_err("set", &[ba(&[1]), num(0.0)]), Err(Error::ArgumentCount { .. })));
    assert!(matches!(call_err("slice", &[ba(&[1])]), Err(Error::ArgumentCount { .. })));
    assert!(matches!(call_err("find", &[ba(&[1])]), Err(Error::ArgumentCount { .. })));
    assert!(matches!(call_err("contains", &[ba(&[1])]), Err(Error::ArgumentCount { .. })));
    assert!(matches!(call_err("starts_with", &[ba(&[1])]), Err(Error::ArgumentCount { .. })));
    assert!(matches!(call_err("equals", &[ba(&[1])]), Err(Error::ArgumentCount { .. })));
    assert!(matches!(call_err("count", &[ba(&[1])]), Err(Error::ArgumentCount { .. })));
    assert!(matches!(call_err("index_of", &[ba(&[1])]), Err(Error::ArgumentCount { .. })));
}

#[test]
fn type_mismatch_for_non_bytes() {
    assert!(matches!(call_err("length", &[num(1.0)]), Err(Error::TypeMismatch { .. })));
    assert!(matches!(call_err("reverse", &[num(1.0)]), Err(Error::TypeMismatch { .. })));
    assert!(matches!(call_err("sum", &[num(1.0)]), Err(Error::TypeMismatch { .. })));
}

#[test]
fn invalid_byte_array_errors() {
    let bad = NonFunctionValue::new(serde_json::json!([1, 256]));
    let result = call_err("length", &[bad]);
    assert!(result.is_err());
}

#[test]
fn negative_index_errors() {
    let result = call_err("get", &[ba(&[1, 2]), num(-1.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}
