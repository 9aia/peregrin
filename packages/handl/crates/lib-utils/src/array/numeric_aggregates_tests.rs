use handl_vm_values_json::Error;

use serde_json::json;

use super::test_helpers::{arr, call, call_err, n, s};

// ── sum ────────────────────────────────────────────────────────────

#[test]
fn sum() {
    assert_eq!(
        call("sum", &[arr(&[json!(1), json!(2), json!(3), json!(4)])]).as_number(),
        Some(10.0)
    );
}

#[test]
fn sum_empty() {
    assert_eq!(call("sum", &[arr(&[])]).as_number(), Some(0.0));
}

#[test]
fn sum_mixed_types() {
    assert_eq!(
        call("sum", &[arr(&[json!(1), json!("ignore"), json!(2)])]).as_number(),
        Some(3.0)
    );
}

// ── avg ────────────────────────────────────────────────────────────

#[test]
fn avg() {
    assert_eq!(
        call("avg", &[arr(&[json!(1), json!(2), json!(3)])]).as_number(),
        Some(2.0)
    );
}

#[test]
fn avg_empty() {
    assert_eq!(call("avg", &[arr(&[])]).as_number(), Some(0.0));
}

// ── min ────────────────────────────────────────────────────────────

#[test]
fn min() {
    assert_eq!(
        call("min", &[arr(&[json!(3), json!(1), json!(2)])]).as_number(),
        Some(1.0)
    );
}

#[test]
fn min_empty() {
    let result = call("min", &[arr(&[])]);
    assert!(result.as_number().is_none());
}

// ── max ────────────────────────────────────────────────────────────

#[test]
fn max() {
    assert_eq!(
        call("max", &[arr(&[json!(3), json!(1), json!(2)])]).as_number(),
        Some(3.0)
    );
}

#[test]
fn max_empty() {
    let result = call("max", &[arr(&[])]);
    assert!(result.as_number().is_none());
}

// ── product ────────────────────────────────────────────────────────

#[test]
fn product_basic() {
    assert_eq!(
        call("product", &[arr(&[json!(2), json!(3), json!(4)])]).as_number(),
        Some(24.0)
    );
}

#[test]
fn product_single() {
    assert_eq!(call("product", &[arr(&[json!(7)])]).as_number(), Some(7.0));
}

#[test]
fn product_empty() {
    assert_eq!(call("product", &[arr(&[])]).as_number(), Some(1.0));
}

#[test]
fn product_with_zero() {
    assert_eq!(
        call("product", &[arr(&[json!(5), json!(0), json!(3)])]).as_number(),
        Some(0.0)
    );
}

#[test]
fn product_negative() {
    assert_eq!(
        call("product", &[arr(&[json!(-2), json!(3)])]).as_number(),
        Some(-6.0)
    );
}

#[test]
fn product_no_args_errors() {
    let result = call_err("product", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn product_non_array_errors() {
    let result = call_err("product", &[n(1.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── mean ───────────────────────────────────────────────────────────

#[test]
fn mean_basic() {
    assert_eq!(
        call("mean", &[arr(&[json!(1), json!(2), json!(3)])]).as_number(),
        Some(2.0)
    );
}

#[test]
fn mean_single() {
    assert_eq!(call("mean", &[arr(&[json!(42)])]).as_number(), Some(42.0));
}

#[test]
fn mean_empty() {
    assert_eq!(call("mean", &[arr(&[])]).as_number(), Some(0.0));
}

#[test]
fn mean_negative() {
    assert_eq!(
        call("mean", &[arr(&[json!(-4), json!(0), json!(4)])]).as_number(),
        Some(0.0)
    );
}

#[test]
fn mean_no_args_errors() {
    let result = call_err("mean", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn mean_non_array_errors() {
    let result = call_err("mean", &[s("x")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── median ─────────────────────────────────────────────────────────

#[test]
fn median_odd() {
    assert_eq!(
        call("median", &[arr(&[json!(3), json!(1), json!(2)])]).as_number(),
        Some(2.0)
    );
}

#[test]
fn median_even() {
    assert_eq!(
        call("median", &[arr(&[json!(1), json!(2), json!(3), json!(4)])]).as_number(),
        Some(2.5)
    );
}

#[test]
fn median_single() {
    assert_eq!(call("median", &[arr(&[json!(5)])]).as_number(), Some(5.0));
}

#[test]
fn median_empty() {
    assert_eq!(call("median", &[arr(&[])]).as_number(), Some(0.0));
}

#[test]
fn median_unsorted() {
    assert_eq!(
        call(
            "median",
            &[arr(&[json!(5), json!(1), json!(3), json!(2), json!(4)])]
        )
        .as_number(),
        Some(3.0)
    );
}

#[test]
fn median_no_args_errors() {
    let result = call_err("median", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn median_non_array_errors() {
    let result = call_err("median", &[n(1.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── stddev ─────────────────────────────────────────────────────────

#[test]
fn stddev_basic() {
    let result = call(
        "stddev",
        &[arr(&[
            json!(2),
            json!(4),
            json!(4),
            json!(4),
            json!(5),
            json!(5),
            json!(7),
            json!(9),
        ])],
    )
    .as_number()
    .unwrap();
    assert!((result - 2.0).abs() < 1e-10);
}

#[test]
fn stddev_single() {
    assert_eq!(call("stddev", &[arr(&[json!(5)])]).as_number(), Some(0.0));
}

#[test]
fn stddev_empty() {
    assert_eq!(call("stddev", &[arr(&[])]).as_number(), Some(0.0));
}

#[test]
fn stddev_uniform() {
    assert_eq!(
        call("stddev", &[arr(&[json!(3), json!(3), json!(3)])]).as_number(),
        Some(0.0)
    );
}

#[test]
fn stddev_no_args_errors() {
    let result = call_err("stddev", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn stddev_non_array_errors() {
    let result = call_err("stddev", &[s("x")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}
