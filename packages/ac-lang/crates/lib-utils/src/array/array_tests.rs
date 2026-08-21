use ac_runtime::RuntimeValue;
use ac_runtime_core::Module;
use ac_vm::{Context, Error};

use serde_json::json;

use super::array::ArrayModule;

fn call(module: &dyn Module, name: &str, args: &[RuntimeValue]) -> RuntimeValue {
    let func = module.function(name).expect("function exists");
    let mut ctx = Context::new();
    func.call(args, &mut ctx).expect("call succeeds")
}

fn arr(items: &[serde_json::Value]) -> RuntimeValue {
    RuntimeValue::new(serde_json::Value::Array(items.to_vec()))
}

fn n(v: f64) -> RuntimeValue {
    RuntimeValue::new(serde_json::json!(v))
}

fn ni(v: i64) -> RuntimeValue {
    RuntimeValue::new(serde_json::json!(v))
}

fn s(v: &str) -> RuntimeValue {
    RuntimeValue::new(v.to_string())
}

#[test]
fn module_name() {
    assert_eq!(ArrayModule.name(), "array");
}

#[test]
fn length() {
    assert_eq!(
        call(
            &ArrayModule,
            "length",
            &[arr(&[json!(1), json!(2), json!(3)])]
        )
        .as_number(),
        Some(3.0)
    );
}

#[test]
fn length_empty() {
    assert_eq!(
        call(&ArrayModule, "length", &[arr(&[])]).as_number(),
        Some(0.0)
    );
}

#[test]
fn reverse() {
    let result = call(
        &ArrayModule,
        "reverse",
        &[arr(&[json!(1), json!(2), json!(3)])],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array[0], json!(3));
    assert_eq!(array[1], json!(2));
    assert_eq!(array[2], json!(1));
}

#[test]
fn concat() {
    let result = call(
        &ArrayModule,
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
        call(
            &ArrayModule,
            "contains",
            &[arr(&[json!(1), json!(2), json!(3)]), ni(2)]
        )
        .as_bool(),
        Some(true)
    );
}

#[test]
fn contains_false() {
    assert_eq!(
        call(
            &ArrayModule,
            "contains",
            &[arr(&[json!(1), json!(2), json!(3)]), ni(5)]
        )
        .as_bool(),
        Some(false)
    );
}

#[test]
fn contains_string() {
    assert_eq!(
        call(
            &ArrayModule,
            "contains",
            &[arr(&[json!("a"), json!("b")]), s("b")]
        )
        .as_bool(),
        Some(true)
    );
}

#[test]
fn index_of_found() {
    assert_eq!(
        call(
            &ArrayModule,
            "index_of",
            &[arr(&[json!(10), json!(20), json!(30)]), ni(20)]
        )
        .as_number(),
        Some(1.0)
    );
}

#[test]
fn index_of_not_found() {
    assert_eq!(
        call(
            &ArrayModule,
            "index_of",
            &[arr(&[json!(10), json!(20)]), ni(99)]
        )
        .as_number(),
        Some(-1.0)
    );
}

#[test]
fn slice() {
    let result = call(
        &ArrayModule,
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
    let result = call(
        &ArrayModule,
        "slice",
        &[arr(&[json!(1), json!(2)]), n(0.0), n(10.0)],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 2);
}

#[test]
fn first() {
    let result = call(
        &ArrayModule,
        "first",
        &[arr(&[json!(1), json!(2), json!(3)])],
    );
    assert_eq!(result.as_number(), Some(1.0));
}

#[test]
fn first_empty() {
    let result = call(&ArrayModule, "first", &[arr(&[])]);
    assert!(result.as_number().is_none());
    assert!(result.as_str().is_none());
}

#[test]
fn last() {
    let result = call(
        &ArrayModule,
        "last",
        &[arr(&[json!(1), json!(2), json!(3)])],
    );
    assert_eq!(result.as_number(), Some(3.0));
}

#[test]
fn last_empty() {
    let result = call(&ArrayModule, "last", &[arr(&[])]);
    assert!(result.as_number().is_none());
    assert!(result.as_str().is_none());
}

#[test]
fn sort_numbers() {
    let result = call(
        &ArrayModule,
        "sort",
        &[arr(&[json!(3), json!(1), json!(2)])],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array[0], json!(1));
    assert_eq!(array[1], json!(2));
    assert_eq!(array[2], json!(3));
}

#[test]
fn sort_strings() {
    let result = call(
        &ArrayModule,
        "sort",
        &[arr(&[json!("c"), json!("a"), json!("b")])],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array[0], json!("a"));
    assert_eq!(array[1], json!("b"));
    assert_eq!(array[2], json!("c"));
}

#[test]
fn sort_mixed_numbers_before_strings() {
    let result = call(
        &ArrayModule,
        "sort",
        &[arr(&[json!("b"), json!(2), json!("a"), json!(1)])],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array[0], json!(1));
    assert_eq!(array[1], json!(2));
    assert_eq!(array[2], json!("a"));
    assert_eq!(array[3], json!("b"));
}

#[test]
fn flatten() {
    let result = call(
        &ArrayModule,
        "flatten",
        &[arr(&[json!(1), json!([2, 3]), json!(4)])],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 4);
    assert_eq!(array[0], json!(1));
    assert_eq!(array[1], json!(2));
    assert_eq!(array[2], json!(3));
    assert_eq!(array[3], json!(4));
}

#[test]
fn flatten_no_nested() {
    let result = call(
        &ArrayModule,
        "flatten",
        &[arr(&[json!(1), json!(2), json!(3)])],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
}

#[test]
fn unique_numbers() {
    let result = call(
        &ArrayModule,
        "unique",
        &[arr(&[json!(1), json!(2), json!(1), json!(3), json!(2)])],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array[0], json!(1));
    assert_eq!(array[1], json!(2));
    assert_eq!(array[2], json!(3));
}

#[test]
fn unique_strings() {
    let result = call(
        &ArrayModule,
        "unique",
        &[arr(&[json!("a"), json!("b"), json!("a")])],
    );
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 2);
    assert_eq!(array[0], json!("a"));
    assert_eq!(array[1], json!("b"));
}

#[test]
fn sum() {
    assert_eq!(
        call(
            &ArrayModule,
            "sum",
            &[arr(&[json!(1), json!(2), json!(3), json!(4)])]
        )
        .as_number(),
        Some(10.0)
    );
}

#[test]
fn sum_empty() {
    assert_eq!(
        call(&ArrayModule, "sum", &[arr(&[])]).as_number(),
        Some(0.0)
    );
}

#[test]
fn sum_mixed_types() {
    assert_eq!(
        call(
            &ArrayModule,
            "sum",
            &[arr(&[json!(1), json!("ignore"), json!(2)])]
        )
        .as_number(),
        Some(3.0)
    );
}

#[test]
fn avg() {
    assert_eq!(
        call(&ArrayModule, "avg", &[arr(&[json!(1), json!(2), json!(3)])]).as_number(),
        Some(2.0)
    );
}

#[test]
fn avg_empty() {
    assert_eq!(
        call(&ArrayModule, "avg", &[arr(&[])]).as_number(),
        Some(0.0)
    );
}

#[test]
fn min() {
    assert_eq!(
        call(&ArrayModule, "min", &[arr(&[json!(3), json!(1), json!(2)])]).as_number(),
        Some(1.0)
    );
}

#[test]
fn min_empty() {
    let result = call(&ArrayModule, "min", &[arr(&[])]);
    assert!(result.as_number().is_none());
}

#[test]
fn max() {
    assert_eq!(
        call(&ArrayModule, "max", &[arr(&[json!(3), json!(1), json!(2)])]).as_number(),
        Some(3.0)
    );
}

#[test]
fn max_empty() {
    let result = call(&ArrayModule, "max", &[arr(&[])]);
    assert!(result.as_number().is_none());
}

#[test]
fn unknown_function_returns_none() {
    assert!(ArrayModule.function("nope").is_none());
}

#[test]
fn type_mismatch_on_non_array() {
    let func = ArrayModule.function("length").unwrap();
    let mut ctx = Context::new();
    let result = func.call(&[s("not an array")], &mut ctx);
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
