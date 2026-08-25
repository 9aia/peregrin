use handl_vm_values_json::Error;

use handl_vm_values_json::NonFunctionValue;

use serde_json::json;

use super::test_helpers::{
    arr, call, call_err, call_err_with_eval, call_with_eval, fn_value, n, s,
};

// ── take ───────────────────────────────────────────────────────────

#[test]
fn take_basic() {
    let result = call(
        "take",
        &[
            arr(&[json!(1), json!(2), json!(3), json!(4), json!(5)]),
            n(3.0),
        ],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array[0], json!(1));
    assert_eq!(array[2], json!(3));
}

#[test]
fn take_more_than_length() {
    let result = call("take", &[arr(&[json!(1), json!(2)]), n(10.0)]);
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 2);
}

#[test]
fn take_zero() {
    let result = call("take", &[arr(&[json!(1), json!(2)]), n(0.0)]);
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
fn take_empty_array() {
    let result = call("take", &[arr(&[]), n(3.0)]);
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
fn take_negative_errors() {
    let result = call_err("take", &[arr(&[json!(1)]), n(-1.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn take_non_number_errors() {
    let result = call_err("take", &[arr(&[json!(1)]), s("x")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn take_too_few_args_errors() {
    let result = call_err("take", &[arr(&[json!(1)])]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn take_non_array_errors() {
    let result = call_err("take", &[n(1.0), n(2.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── drop ───────────────────────────────────────────────────────────

#[test]
fn drop_basic() {
    let result = call(
        "drop",
        &[
            arr(&[json!(1), json!(2), json!(3), json!(4), json!(5)]),
            n(2.0),
        ],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array[0], json!(3));
    assert_eq!(array[2], json!(5));
}

#[test]
fn drop_more_than_length() {
    let result = call("drop", &[arr(&[json!(1), json!(2)]), n(10.0)]);
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
fn drop_zero() {
    let result = call("drop", &[arr(&[json!(1), json!(2)]), n(0.0)]);
    assert_eq!(result.as_array().unwrap().len(), 2);
}

#[test]
fn drop_empty_array() {
    let result = call("drop", &[arr(&[]), n(3.0)]);
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
fn drop_negative_errors() {
    let result = call_err("drop", &[arr(&[json!(1)]), n(-1.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn drop_non_array_errors() {
    let result = call_err("drop", &[n(1.0), n(2.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── enumerate ──────────────────────────────────────────────────────

#[test]
fn enumerate_basic() {
    let result = call("enumerate", &[arr(&[json!("a"), json!("b"), json!("c")])]);
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array[0], json!([0, "a"]));
    assert_eq!(array[1], json!([1, "b"]));
    assert_eq!(array[2], json!([2, "c"]));
}

#[test]
fn enumerate_empty() {
    let result = call("enumerate", &[arr(&[])]);
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
fn enumerate_single() {
    let result = call("enumerate", &[arr(&[json!(42)])]);
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 1);
    assert_eq!(array[0], json!([0, 42]));
}

#[test]
fn enumerate_non_array_errors() {
    let result = call_err("enumerate", &[n(1.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── unzip ──────────────────────────────────────────────────────────

#[test]
fn unzip_basic() {
    let result = call(
        "unzip",
        &[arr(&[json!([1, "a"]), json!([2, "b"]), json!([3, "c"])])],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 2);
    assert_eq!(array[0], json!([1, 2, 3]));
    assert_eq!(array[1], json!(["a", "b", "c"]));
}

#[test]
fn unzip_empty() {
    let result = call("unzip", &[arr(&[])]);
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 2);
    assert!(array[0].as_array().unwrap().is_empty());
    assert!(array[1].as_array().unwrap().is_empty());
}

#[test]
fn unzip_skips_non_pairs() {
    let result = call("unzip", &[arr(&[json!([1, 2]), json!(42), json!([3, 4])])]);
    let array = result.as_array().unwrap();
    assert_eq!(array[0], json!([1, 3]));
    assert_eq!(array[1], json!([2, 4]));
}

#[test]
fn unzip_non_array_errors() {
    let result = call_err("unzip", &[n(1.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── intersperse ────────────────────────────────────────────────────

#[test]
fn intersperse_basic() {
    let result = call(
        "intersperse",
        &[arr(&[json!(1), json!(2), json!(3)]), s(",")],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 5);
    assert_eq!(array[0], json!(1));
    assert_eq!(array[1], json!(","));
    assert_eq!(array[2], json!(2));
    assert_eq!(array[3], json!(","));
    assert_eq!(array[4], json!(3));
}

#[test]
fn intersperse_single_element() {
    let result = call("intersperse", &[arr(&[json!(1)]), s(",")]);
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 1);
    assert_eq!(array[0], json!(1));
}

#[test]
fn intersperse_empty() {
    let result = call("intersperse", &[arr(&[]), s(",")]);
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
fn intersperse_with_number_separator() {
    let result = call("intersperse", &[arr(&[json!(1), json!(2)]), n(0.0)]);
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array[1].as_f64(), Some(0.0));
}

#[test]
fn intersperse_too_few_args_errors() {
    let result = call_err("intersperse", &[arr(&[json!(1)])]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn intersperse_non_array_errors() {
    let result = call_err("intersperse", &[n(1.0), s(",")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── intercalate ────────────────────────────────────────────────────

#[test]
fn intercalate_basic() {
    let result = call(
        "intercalate",
        &[
            arr(&[json!([1, 2]), json!([3, 4]), json!([5])]),
            arr(&[json!(0), json!(0)]),
        ],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 9);
    assert_eq!(array[0], json!(1));
    assert_eq!(array[1], json!(2));
    assert_eq!(array[2], json!(0));
    assert_eq!(array[3], json!(0));
    assert_eq!(array[4], json!(3));
    assert_eq!(array[5], json!(4));
    assert_eq!(array[6], json!(0));
    assert_eq!(array[7], json!(0));
    assert_eq!(array[8], json!(5));
}

#[test]
fn intercalate_single_group() {
    let result = call("intercalate", &[arr(&[json!([1, 2])]), arr(&[json!(0)])]);
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 2);
    assert_eq!(array[0], json!(1));
    assert_eq!(array[1], json!(2));
}

#[test]
fn intercalate_empty_groups() {
    let result = call("intercalate", &[arr(&[]), arr(&[json!(0)])]);
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
fn intercalate_non_array_second_errors() {
    let result = call_err("intercalate", &[arr(&[json!([1])]), n(0.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── dedupe ─────────────────────────────────────────────────────────

#[test]
fn dedupe_consecutive() {
    let result = call(
        "dedupe",
        &[arr(&[
            json!(1),
            json!(1),
            json!(2),
            json!(2),
            json!(3),
            json!(1),
        ])],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 4);
    assert_eq!(array[0], json!(1));
    assert_eq!(array[1], json!(2));
    assert_eq!(array[2], json!(3));
    assert_eq!(array[3], json!(1));
}

#[test]
fn dedupe_all_same() {
    let result = call("dedupe", &[arr(&[json!(1), json!(1), json!(1)])]);
    assert_eq!(result.as_array().unwrap().len(), 1);
}

#[test]
fn dedupe_no_duplicates() {
    let result = call("dedupe", &[arr(&[json!(1), json!(2), json!(3)])]);
    assert_eq!(result.as_array().unwrap().len(), 3);
}

#[test]
fn dedupe_empty() {
    let result = call("dedupe", &[arr(&[])]);
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
fn dedupe_strings() {
    let result = call(
        "dedupe",
        &[arr(&[
            json!("a"),
            json!("a"),
            json!("b"),
            json!("b"),
            json!("a"),
        ])],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array[0], json!("a"));
    assert_eq!(array[1], json!("b"));
    assert_eq!(array[2], json!("a"));
}

#[test]
fn dedupe_non_array_errors() {
    let result = call_err("dedupe", &[n(1.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── prepend ────────────────────────────────────────────────────────

#[test]
fn prepend_basic() {
    let result = call("prepend", &[arr(&[json!(2), json!(3)]), n(1.0)]);
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array[0].as_f64(), Some(1.0));
    assert_eq!(array[1], json!(2));
    assert_eq!(array[2], json!(3));
}

#[test]
fn prepend_to_empty() {
    let result = call("prepend", &[arr(&[]), n(1.0)]);
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 1);
    assert_eq!(array[0].as_f64(), Some(1.0));
}

#[test]
fn prepend_string() {
    let result = call("prepend", &[arr(&[json!("b")]), s("a")]);
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 2);
    assert_eq!(array[0], json!("a"));
    assert_eq!(array[1], json!("b"));
}

#[test]
fn prepend_too_few_args_errors() {
    let result = call_err("prepend", &[arr(&[json!(1)])]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn prepend_non_array_errors() {
    let result = call_err("prepend", &[n(1.0), n(2.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── take_while ─────────────────────────────────────────────────────

#[test]
fn take_while_basic() {
    let result = call_with_eval(
        "take_while",
        &[
            arr(&[json!(1), json!(2), json!(3), json!(1), json!(0)]),
            fn_value("add(x, -3)", &["x"]),
        ],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 2);
    assert_eq!(array[0], json!(1));
    assert_eq!(array[1], json!(2));
}

#[test]
fn take_while_all_match() {
    let result = call_with_eval(
        "take_while",
        &[arr(&[json!(1), json!(2)]), fn_value("add(x, -10)", &["x"])],
    );
    assert_eq!(result.as_array().unwrap().len(), 2);
}

#[test]
fn take_while_none_match() {
    let result = call_with_eval(
        "take_while",
        &[arr(&[json!(3), json!(1)]), fn_value("add(x, -3)", &["x"])],
    );
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
fn take_while_empty() {
    let result = call_with_eval("take_while", &[arr(&[]), fn_value("add(x, -3)", &["x"])]);
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
fn take_while_non_function_errors() {
    let result = call_err_with_eval("take_while", &[arr(&[json!(1)]), n(1.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn take_while_non_array_errors() {
    let result = call_err_with_eval("take_while", &[n(1.0), fn_value("x", &["x"])]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── drop_while ─────────────────────────────────────────────────────

#[test]
fn drop_while_basic() {
    let result = call_with_eval(
        "drop_while",
        &[
            arr(&[json!(1), json!(2), json!(3), json!(1), json!(0)]),
            fn_value("add(x, -3)", &["x"]),
        ],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array[0], json!(3));
    assert_eq!(array[1], json!(1));
    assert_eq!(array[2], json!(0));
}

#[test]
fn drop_while_all_match() {
    let result = call_with_eval(
        "drop_while",
        &[arr(&[json!(1), json!(2)]), fn_value("add(x, -10)", &["x"])],
    );
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
fn drop_while_none_match() {
    let result = call_with_eval(
        "drop_while",
        &[arr(&[json!(3), json!(1)]), fn_value("add(x, -3)", &["x"])],
    );
    assert_eq!(result.as_array().unwrap().len(), 2);
}

#[test]
fn drop_while_empty() {
    let result = call_with_eval("drop_while", &[arr(&[]), fn_value("add(x, -3)", &["x"])]);
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
fn drop_while_non_function_errors() {
    let result = call_err_with_eval("drop_while", &[arr(&[json!(1)]), n(1.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── scan ───────────────────────────────────────────────────────────

#[test]
fn scan_basic() {
    let result = call_with_eval(
        "scan",
        &[
            arr(&[json!(1), json!(2), json!(3)]),
            fn_value("add(acc, x)", &["acc", "x"]),
            n(0.0),
        ],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array[0].as_f64(), Some(1.0));
    assert_eq!(array[1].as_f64(), Some(3.0));
    assert_eq!(array[2].as_f64(), Some(6.0));
}

#[test]
fn scan_empty() {
    let result = call_with_eval(
        "scan",
        &[arr(&[]), fn_value("add(acc, x)", &["acc", "x"]), n(0.0)],
    );
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
fn scan_single_element() {
    let result = call_with_eval(
        "scan",
        &[
            arr(&[json!(5)]),
            fn_value("add(acc, x)", &["acc", "x"]),
            n(0.0),
        ],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 1);
    assert_eq!(array[0].as_f64(), Some(5.0));
}

#[test]
fn scan_too_few_args_errors() {
    let result = call_err_with_eval(
        "scan",
        &[arr(&[json!(1)]), fn_value("add(acc, x)", &["acc", "x"])],
    );
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn scan_non_array_errors() {
    let result = call_err_with_eval(
        "scan",
        &[n(1.0), fn_value("add(acc, x)", &["acc", "x"]), n(0.0)],
    );
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── foldr ──────────────────────────────────────────────────────────

#[test]
fn foldr_basic() {
    let result = call_with_eval(
        "foldr",
        &[
            arr(&[json!(1), json!(2), json!(3)]),
            fn_value("add(acc, x)", &["acc", "x"]),
            n(0.0),
        ],
    );
    // Right fold: processes right-to-left with fn(acc, item):
    //   acc=0, item=3 -> add(0, 3) = 3
    //   acc=3, item=2 -> add(3, 2) = 5
    //   acc=5, item=1 -> add(5, 1) = 6
    assert_eq!(result.as_number(), Some(6.0));
}

#[test]
fn foldr_empty() {
    let result = call_with_eval(
        "foldr",
        &[arr(&[]), fn_value("add(acc, x)", &["acc", "x"]), n(42.0)],
    );
    assert_eq!(result.as_number(), Some(42.0));
}

#[test]
fn foldr_single_element() {
    let result = call_with_eval(
        "foldr",
        &[
            arr(&[json!(5)]),
            fn_value("add(acc, x)", &["acc", "x"]),
            n(0.0),
        ],
    );
    assert_eq!(result.as_number(), Some(5.0));
}

#[test]
fn foldr_too_few_args_errors() {
    let result = call_err_with_eval(
        "foldr",
        &[arr(&[json!(1)]), fn_value("add(acc, x)", &["acc", "x"])],
    );
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn foldr_non_array_errors() {
    let result = call_err_with_eval(
        "foldr",
        &[n(1.0), fn_value("add(acc, x)", &["acc", "x"]), n(0.0)],
    );
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── find_index ─────────────────────────────────────────────────────

#[test]
fn find_index_basic() {
    let result = call_with_eval(
        "find_index",
        &[
            arr(&[json!(0), json!(0), json!(3), json!(0)]),
            fn_value("add(x, 0)", &["x"]),
        ],
    );
    assert_eq!(result.as_number(), Some(2.0));
}

#[test]
fn find_index_not_found() {
    let result = call_with_eval(
        "find_index",
        &[arr(&[json!(0), json!(0)]), fn_value("add(x, 0)", &["x"])],
    );
    assert_eq!(result.as_number(), Some(-1.0));
}

#[test]
fn find_index_empty() {
    let result = call_with_eval("find_index", &[arr(&[]), fn_value("add(x, 0)", &["x"])]);
    assert_eq!(result.as_number(), Some(-1.0));
}

#[test]
fn find_index_non_function_errors() {
    let result = call_err_with_eval("find_index", &[arr(&[json!(1)]), n(1.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── find_last ──────────────────────────────────────────────────────

#[test]
fn find_last_basic() {
    let result = call_with_eval(
        "find_last",
        &[
            arr(&[json!(0), json!(0), json!(3), json!(5)]),
            fn_value("add(x, 0)", &["x"]),
        ],
    );
    assert_eq!(result.as_number(), Some(5.0));
}

#[test]
fn find_last_not_found() {
    let result = call_with_eval(
        "find_last",
        &[arr(&[json!(0), json!(0)]), fn_value("add(x, 0)", &["x"])],
    );
    assert!(result.is_null());
}

#[test]
fn find_last_empty() {
    let result = call_with_eval("find_last", &[arr(&[]), fn_value("add(x, 0)", &["x"])]);
    assert!(result.is_null());
}

#[test]
fn find_last_first_element() {
    let result = call_with_eval(
        "find_last",
        &[
            arr(&[json!(3), json!(0), json!(0)]),
            fn_value("add(x, 0)", &["x"]),
        ],
    );
    assert_eq!(result.as_number(), Some(3.0));
}

#[test]
fn find_last_non_function_errors() {
    let result = call_err_with_eval("find_last", &[arr(&[json!(1)]), n(1.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── find_last_index ────────────────────────────────────────────────

#[test]
fn find_last_index_basic() {
    let result = call_with_eval(
        "find_last_index",
        &[
            arr(&[json!(0), json!(0), json!(3), json!(5)]),
            fn_value("add(x, 0)", &["x"]),
        ],
    );
    assert_eq!(result.as_number(), Some(3.0));
}

#[test]
fn find_last_index_not_found() {
    let result = call_with_eval(
        "find_last_index",
        &[arr(&[json!(0), json!(0)]), fn_value("add(x, 0)", &["x"])],
    );
    assert_eq!(result.as_number(), Some(-1.0));
}

#[test]
fn find_last_index_empty() {
    let result = call_with_eval(
        "find_last_index",
        &[arr(&[]), fn_value("add(x, 0)", &["x"])],
    );
    assert_eq!(result.as_number(), Some(-1.0));
}

#[test]
fn find_last_index_single_match() {
    let result = call_with_eval(
        "find_last_index",
        &[
            arr(&[json!(3), json!(0), json!(0)]),
            fn_value("add(x, 0)", &["x"]),
        ],
    );
    assert_eq!(result.as_number(), Some(0.0));
}

#[test]
fn find_last_index_non_function_errors() {
    let result = call_err_with_eval("find_last_index", &[arr(&[json!(1)]), n(1.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── windows ───────────────────────────────────────────────────────

#[test]
fn windows_basic() {
    let result = call(
        "windows",
        &[arr(&[json!(1), json!(2), json!(3), json!(4)]), n(2.0)],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array[0], json!([1, 2]));
    assert_eq!(array[1], json!([2, 3]));
    assert_eq!(array[2], json!([3, 4]));
}

#[test]
fn windows_size_three() {
    let result = call(
        "windows",
        &[
            arr(&[json!(1), json!(2), json!(3), json!(4), json!(5)]),
            n(3.0),
        ],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array[0], json!([1, 2, 3]));
    assert_eq!(array[2], json!([3, 4, 5]));
}

#[test]
fn windows_size_equals_length() {
    let result = call("windows", &[arr(&[json!(1), json!(2)]), n(2.0)]);
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 1);
    assert_eq!(array[0], json!([1, 2]));
}

#[test]
fn windows_size_greater_than_length() {
    let result = call("windows", &[arr(&[json!(1), json!(2)]), n(5.0)]);
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
fn windows_empty_array() {
    let result = call("windows", &[arr(&[]), n(2.0)]);
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
fn windows_zero_size_errors() {
    let result = call_err("windows", &[arr(&[json!(1)]), n(0.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn windows_negative_size_errors() {
    let result = call_err("windows", &[arr(&[json!(1)]), n(-1.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn windows_non_number_errors() {
    let result = call_err("windows", &[arr(&[json!(1)]), s("x")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn windows_non_array_errors() {
    let result = call_err("windows", &[n(1.0), n(2.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn windows_too_few_args_errors() {
    let result = call_err("windows", &[arr(&[json!(1)])]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── transpose ─────────────────────────────────────────────────────

#[test]
fn transpose_basic() {
    let result = call("transpose", &[arr(&[json!([1, 2]), json!([3, 4])])]);
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 2);
    assert_eq!(array[0], json!([1, 3]));
    assert_eq!(array[1], json!([2, 4]));
}

#[test]
fn transpose_rectangular() {
    let result = call("transpose", &[arr(&[json!([1, 2, 3]), json!([4, 5, 6])])]);
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array[0], json!([1, 4]));
    assert_eq!(array[1], json!([2, 5]));
    assert_eq!(array[2], json!([3, 6]));
}

#[test]
fn transpose_single_row() {
    let result = call("transpose", &[arr(&[json!([1, 2, 3])])]);
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array[0], json!([1]));
    assert_eq!(array[1], json!([2]));
    assert_eq!(array[2], json!([3]));
}

#[test]
fn transpose_single_col() {
    let result = call("transpose", &[arr(&[json!([1]), json!([2]), json!([3])])]);
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 1);
    assert_eq!(array[0], json!([1, 2, 3]));
}

#[test]
fn transpose_empty() {
    let result = call("transpose", &[arr(&[])]);
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
fn transpose_ragged() {
    let result = call("transpose", &[arr(&[json!([1, 2, 3]), json!([4, 5])])]);
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array[0], json!([1, 4]));
    assert_eq!(array[1], json!([2, 5]));
    assert_eq!(array[2], json!([3]));
}

#[test]
fn transpose_non_array_errors() {
    let result = call_err("transpose", &[n(1.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn transpose_no_args_errors() {
    let result = call_err("transpose", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── insert_at ─────────────────────────────────────────────────────

#[test]
fn insert_at_beginning() {
    let result = call(
        "insert_at",
        &[
            arr(&[json!(2), json!(3)]),
            n(0.0),
            NonFunctionValue::new(json!(1)),
        ],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array, &[json!(1), json!(2), json!(3)]);
}

#[test]
fn insert_at_middle() {
    let result = call(
        "insert_at",
        &[
            arr(&[json!(1), json!(3)]),
            n(1.0),
            NonFunctionValue::new(json!(2)),
        ],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array, &[json!(1), json!(2), json!(3)]);
}

#[test]
fn insert_at_end() {
    let result = call(
        "insert_at",
        &[
            arr(&[json!(1), json!(2)]),
            n(2.0),
            NonFunctionValue::new(json!(3)),
        ],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array, &[json!(1), json!(2), json!(3)]);
}

#[test]
fn insert_at_index_beyond_length_appends() {
    let result = call(
        "insert_at",
        &[arr(&[json!(1)]), n(10.0), NonFunctionValue::new(json!(2))],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array, &[json!(1), json!(2)]);
}

#[test]
fn insert_at_empty_array() {
    let result = call(
        "insert_at",
        &[arr(&[]), n(0.0), NonFunctionValue::new(json!(1))],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array, &[json!(1)]);
}

#[test]
fn insert_at_negative_index_errors() {
    let result = call_err(
        "insert_at",
        &[arr(&[json!(1)]), n(-1.0), NonFunctionValue::new(json!(2))],
    );
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn insert_at_non_number_index_errors() {
    let result = call_err(
        "insert_at",
        &[arr(&[json!(1)]), s("x"), NonFunctionValue::new(json!(2))],
    );
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn insert_at_non_array_errors() {
    let result = call_err(
        "insert_at",
        &[n(1.0), n(0.0), NonFunctionValue::new(json!(2))],
    );
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn insert_at_too_few_args_errors() {
    let result = call_err("insert_at", &[arr(&[json!(1)]), n(0.0)]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── remove_at ─────────────────────────────────────────────────────

#[test]
fn remove_at_beginning() {
    let result = call("remove_at", &[arr(&[json!(1), json!(2), json!(3)]), n(0.0)]);
    let array = result.as_array().unwrap();
    assert_eq!(array, &[json!(2), json!(3)]);
}

#[test]
fn remove_at_middle() {
    let result = call("remove_at", &[arr(&[json!(1), json!(2), json!(3)]), n(1.0)]);
    let array = result.as_array().unwrap();
    assert_eq!(array, &[json!(1), json!(3)]);
}

#[test]
fn remove_at_end() {
    let result = call("remove_at", &[arr(&[json!(1), json!(2), json!(3)]), n(2.0)]);
    let array = result.as_array().unwrap();
    assert_eq!(array, &[json!(1), json!(2)]);
}

#[test]
fn remove_at_index_beyond_length_returns_unchanged() {
    let result = call("remove_at", &[arr(&[json!(1), json!(2)]), n(10.0)]);
    let array = result.as_array().unwrap();
    assert_eq!(array, &[json!(1), json!(2)]);
}

#[test]
fn remove_at_single_element() {
    let result = call("remove_at", &[arr(&[json!(1)]), n(0.0)]);
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
fn remove_at_negative_index_errors() {
    let result = call_err("remove_at", &[arr(&[json!(1)]), n(-1.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn remove_at_non_number_index_errors() {
    let result = call_err("remove_at", &[arr(&[json!(1)]), s("x")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn remove_at_non_array_errors() {
    let result = call_err("remove_at", &[n(1.0), n(0.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn remove_at_too_few_args_errors() {
    let result = call_err("remove_at", &[arr(&[json!(1)])]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}
