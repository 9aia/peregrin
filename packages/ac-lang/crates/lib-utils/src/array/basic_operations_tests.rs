use ac_runtime::NonFunctionValue;
use ac_runtime_core::Module;
use ac_vm::{Context, Error};

use serde_json::json;

use super::module::ArrayModule;
use super::test_helpers::{arr, call, n, ni, s};

#[test]
fn module_name() {
    assert_eq!(ArrayModule.name(), "array");
}

#[test]
fn length() {
    assert_eq!(
        call("length", &[arr(&[json!(1), json!(2), json!(3)])]).as_number(),
        Some(3.0)
    );
}

#[test]
fn length_empty() {
    assert_eq!(call("length", &[arr(&[])]).as_number(), Some(0.0));
}

#[test]
fn reverse() {
    let result = call("reverse", &[arr(&[json!(1), json!(2), json!(3)])]);
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array[0], json!(3));
    assert_eq!(array[1], json!(2));
    assert_eq!(array[2], json!(1));
}

#[test]
fn concat() {
    let result = call(
        "concat",
        &[arr(&[json!(1), json!(2)]), arr(&[json!(3), json!(4)])],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 4);
    assert_eq!(array[0], json!(1));
    assert_eq!(array[3], json!(4));
}

#[test]
fn contains_true() {
    assert_eq!(
        call("contains", &[arr(&[json!(1), json!(2), json!(3)]), ni(2)]).as_bool(),
        Some(true)
    );
}

#[test]
fn contains_false() {
    assert_eq!(
        call("contains", &[arr(&[json!(1), json!(2), json!(3)]), ni(5)]).as_bool(),
        Some(false)
    );
}

#[test]
fn contains_string() {
    assert_eq!(
        call("contains", &[arr(&[json!("a"), json!("b")]), s("b")]).as_bool(),
        Some(true)
    );
}

#[test]
fn index_of_found() {
    assert_eq!(
        call("index_of", &[arr(&[json!(10), json!(20), json!(30)]), ni(20)]).as_number(),
        Some(1.0)
    );
}

#[test]
fn index_of_not_found() {
    assert_eq!(
        call("index_of", &[arr(&[json!(10), json!(20)]), ni(99)]).as_number(),
        Some(-1.0)
    );
}

#[test]
fn slice() {
    let result = call(
        "slice",
        &[
            arr(&[json!(1), json!(2), json!(3), json!(4), json!(5)]),
            n(1.0),
            n(4.0),
        ],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array[0], json!(2));
    assert_eq!(array[2], json!(4));
}

#[test]
fn slice_out_of_bounds_clamps() {
    let result = call("slice", &[arr(&[json!(1), json!(2)]), n(0.0), n(10.0)]);
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 2);
}

#[test]
fn first() {
    let result = call("first", &[arr(&[json!(1), json!(2), json!(3)])]);
    assert_eq!(result.as_number(), Some(1.0));
}

#[test]
fn first_empty() {
    let result = call("first", &[arr(&[])]);
    assert!(result.as_number().is_none());
    assert!(result.as_str().is_none());
}

#[test]
fn last() {
    let result = call("last", &[arr(&[json!(1), json!(2), json!(3)])]);
    assert_eq!(result.as_number(), Some(3.0));
}

#[test]
fn last_empty() {
    let result = call("last", &[arr(&[])]);
    assert!(result.as_number().is_none());
    assert!(result.as_str().is_none());
}

#[test]
fn sort_numbers() {
    let result = call("sort", &[arr(&[json!(3), json!(1), json!(2)])]);
    let array = result.as_array().unwrap();
    assert_eq!(array[0], json!(1));
    assert_eq!(array[1], json!(2));
    assert_eq!(array[2], json!(3));
}

#[test]
fn sort_strings() {
    let result = call("sort", &[arr(&[json!("c"), json!("a"), json!("b")])]);
    let array = result.as_array().unwrap();
    assert_eq!(array[0], json!("a"));
    assert_eq!(array[1], json!("b"));
    assert_eq!(array[2], json!("c"));
}

#[test]
fn sort_mixed_numbers_before_strings() {
    let result = call("sort", &[arr(&[json!("b"), json!(2), json!("a"), json!(1)])]);
    let array = result.as_array().unwrap();
    assert_eq!(array[0], json!(1));
    assert_eq!(array[1], json!(2));
    assert_eq!(array[2], json!("a"));
    assert_eq!(array[3], json!("b"));
}

#[test]
fn flatten() {
    let result = call("flatten", &[arr(&[json!(1), json!([2, 3]), json!(4)])]);
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 4);
    assert_eq!(array[0], json!(1));
    assert_eq!(array[1], json!(2));
    assert_eq!(array[2], json!(3));
    assert_eq!(array[3], json!(4));
}

#[test]
fn flatten_no_nested() {
    let result = call("flatten", &[arr(&[json!(1), json!(2), json!(3)])]);
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
}

#[test]
fn at_valid_index() {
    let result = call("at", &[arr(&[json!(10), json!(20), json!(30)]), n(1.0)]);
    assert_eq!(result.as_number(), Some(20.0));
}

#[test]
fn at_out_of_bounds_returns_null() {
    let result = call("at", &[arr(&[json!(1)]), n(5.0)]);
    assert!(result.is_null());
}

#[test]
fn at_negative_errors() {
    let func = ArrayModule.function("at").unwrap();
    let mut ctx = Context::new();
    let result = func.call(&[arr(&[json!(1)]), n(-1.0)], &mut ctx);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn push_single() {
    let result = call("push", &[arr(&[json!(1), json!(2)]), n(3.0)]);
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array[2].as_f64(), Some(3.0));
}

#[test]
fn push_multiple() {
    let result = call("push", &[arr(&[json!(1)]), n(2.0), n(3.0), s("a")]);
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 4);
    assert_eq!(array[3], json!("a"));
}

#[test]
fn fill_three() {
    let result = call("fill", &[n(3.0), n(0.0)]);
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array[0].as_f64(), Some(0.0));
    assert_eq!(array[2].as_f64(), Some(0.0));
}

#[test]
fn fill_zero() {
    let result = call("fill", &[n(0.0), s("x")]);
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
fn fill_negative_errors() {
    let func = ArrayModule.function("fill").unwrap();
    let mut ctx = Context::new();
    let result = func.call(&[n(-1.0), n(0.0)], &mut ctx);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn deep_flatten_nested() {
    let result = call(
        "deep_flatten",
        &[arr(&[json!(1), json!([2, json!([3, 4])]), json!(5)])],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 5);
    assert_eq!(array[0], json!(1));
    assert_eq!(array[4], json!(5));
}

#[test]
fn deep_flatten_no_nested() {
    let result = call("deep_flatten", &[arr(&[json!(1), json!(2), json!(3)])]);
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
}

#[test]
fn slice_negative_errors() {
    let func = ArrayModule.function("slice").unwrap();
    let mut ctx = Context::new();
    let result = func.call(&[arr(&[json!(1)]), n(-1.0), n(1.0)], &mut ctx);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn pop_removes_last() {
    let result = call("pop", &[arr(&[json!(1), json!(2), json!(3)])]);
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 2);
    assert_eq!(array[0].as_f64(), Some(1.0));
    assert_eq!(array[1].as_f64(), Some(2.0));
}

#[test]
fn pop_empty_returns_empty() {
    let result = call("pop", &[arr(&[])]);
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
fn pop_single_element() {
    let result = call("pop", &[arr(&[json!(42)])]);
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
fn shift_removes_first() {
    let result = call("shift", &[arr(&[json!(1), json!(2), json!(3)])]);
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 2);
    assert_eq!(array[0].as_f64(), Some(2.0));
    assert_eq!(array[1].as_f64(), Some(3.0));
}

#[test]
fn shift_empty_returns_empty() {
    let result = call("shift", &[arr(&[])]);
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
fn shift_single_element() {
    let result = call("shift", &[arr(&[json!(42)])]);
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
fn unknown_function_returns_none() {
    assert!(ArrayModule.function("nope").is_none());
}

#[test]
fn type_mismatch_on_non_array() {
    let func = ArrayModule.function("length").unwrap();
    let mut ctx = Context::new();
    let result: Result<NonFunctionValue, Error> = func.call(&[s("not an array")], &mut ctx);
    assert_eq!(
        result,
        Err(Error::TypeMismatch {
            expected: "array",
            got: "non-array",
        })
    );
}

#[test]
fn argument_count_on_missing_args() {
    let func = ArrayModule.function("length").unwrap();
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
