use handl_vm_values_json::NonFunctionValue;

use handl_vm_values_json::{CanonicalContext, Error};

use handl_vm_common::Module;

use super::MathModule;

fn call(module: &dyn Module, name: &str, args: &[NonFunctionValue]) -> NonFunctionValue {
    let func = module.function(name).expect("function exists");
    let mut ctx = CanonicalContext::new();
    func.call(args, &mut ctx).expect("call succeeds")
}

fn call_err(
    module: &dyn Module,
    name: &str,
    args: &[NonFunctionValue],
) -> Result<NonFunctionValue, Error> {
    let func = module.function(name).expect("function exists");
    let mut ctx = CanonicalContext::new();
    func.call(args, &mut ctx)
}

fn s(v: &str) -> NonFunctionValue {
    NonFunctionValue::new(v.to_string())
}

fn n(v: f64) -> NonFunctionValue {
    NonFunctionValue::new(serde_json::json!(v))
}

#[test]
fn module_name() {
    assert_eq!(MathModule.name(), "math");
}

#[test]
fn abs_positive() {
    assert_eq!(call(&MathModule, "abs", &[n(5.0)]).as_number(), Some(5.0));
}

#[test]
fn abs_negative() {
    assert_eq!(call(&MathModule, "abs", &[n(-5.0)]).as_number(), Some(5.0));
}

#[test]
fn floor() {
    assert_eq!(call(&MathModule, "floor", &[n(3.7)]).as_number(), Some(3.0));
}

#[test]
fn ceil() {
    assert_eq!(call(&MathModule, "ceil", &[n(3.2)]).as_number(), Some(4.0));
}

#[test]
fn round() {
    assert_eq!(call(&MathModule, "round", &[n(3.5)]).as_number(), Some(4.0));
    assert_eq!(call(&MathModule, "round", &[n(3.4)]).as_number(), Some(3.0));
}

#[test]
fn min() {
    assert_eq!(
        call(&MathModule, "min", &[n(3.0), n(7.0)]).as_number(),
        Some(3.0)
    );
}

#[test]
fn max() {
    assert_eq!(
        call(&MathModule, "max", &[n(3.0), n(7.0)]).as_number(),
        Some(7.0)
    );
}

#[test]
fn sqrt() {
    assert_eq!(call(&MathModule, "sqrt", &[n(9.0)]).as_number(), Some(3.0));
}

#[test]
fn pow() {
    assert_eq!(
        call(&MathModule, "pow", &[n(2.0), n(3.0)]).as_number(),
        Some(8.0)
    );
}

#[test]
fn sign_positive() {
    assert_eq!(call(&MathModule, "sign", &[n(5.0)]).as_number(), Some(1.0));
}

#[test]
fn sign_negative() {
    assert_eq!(
        call(&MathModule, "sign", &[n(-5.0)]).as_number(),
        Some(-1.0)
    );
}

#[test]
fn sign_zero() {
    assert_eq!(call(&MathModule, "sign", &[n(0.0)]).as_number(), Some(1.0));
}

#[test]
fn trunc() {
    assert_eq!(call(&MathModule, "trunc", &[n(3.7)]).as_number(), Some(3.0));
    assert_eq!(
        call(&MathModule, "trunc", &[n(-3.7)]).as_number(),
        Some(-3.0)
    );
}

#[test]
fn clamp_within_range() {
    assert_eq!(
        call(&MathModule, "clamp", &[n(5.0), n(0.0), n(10.0)]).as_number(),
        Some(5.0)
    );
}

#[test]
fn clamp_below_min() {
    assert_eq!(
        call(&MathModule, "clamp", &[n(-5.0), n(0.0), n(10.0)]).as_number(),
        Some(0.0)
    );
}

#[test]
fn clamp_above_max() {
    assert_eq!(
        call(&MathModule, "clamp", &[n(15.0), n(0.0), n(10.0)]).as_number(),
        Some(10.0)
    );
}

#[test]
fn log() {
    let result = call(&MathModule, "log", &[n(std::f64::consts::E)]).as_number();
    assert!(result.is_some());
    assert!((result.unwrap() - 1.0).abs() < 1e-10);
}

#[test]
fn log10() {
    assert_eq!(
        call(&MathModule, "log10", &[n(100.0)]).as_number(),
        Some(2.0)
    );
}

#[test]
fn exp() {
    let result = call(&MathModule, "exp", &[n(1.0)]).as_number();
    assert!(result.is_some());
    assert!((result.unwrap() - std::f64::consts::E).abs() < 1e-10);
}

#[test]
fn sin() {
    let result = call(&MathModule, "sin", &[n(0.0)]).as_number();
    assert!(result.is_some());
    assert!(result.unwrap().abs() < 1e-10);
}

#[test]
fn cos() {
    let result = call(&MathModule, "cos", &[n(0.0)]).as_number();
    assert!(result.is_some());
    assert!((result.unwrap() - 1.0).abs() < 1e-10);
}

#[test]
fn tan() {
    let result = call(&MathModule, "tan", &[n(0.0)]).as_number();
    assert!(result.is_some());
    assert!(result.unwrap().abs() < 1e-10);
}

#[test]
fn asin() {
    let result = call(&MathModule, "asin", &[n(1.0)]).as_number();
    assert!(result.is_some());
    assert!((result.unwrap() - std::f64::consts::FRAC_PI_2).abs() < 1e-10);
}

#[test]
fn acos() {
    let result = call(&MathModule, "acos", &[n(1.0)]).as_number();
    assert!(result.is_some());
    assert!(result.unwrap().abs() < 1e-10);
}

#[test]
fn atan() {
    let result = call(&MathModule, "atan", &[n(1.0)]).as_number();
    assert!(result.is_some());
    assert!((result.unwrap() - std::f64::consts::FRAC_PI_4).abs() < 1e-10);
}

#[test]
fn atan2() {
    let result = call(&MathModule, "atan2", &[n(1.0), n(1.0)]).as_number();
    assert!(result.is_some());
    assert!((result.unwrap() - std::f64::consts::FRAC_PI_4).abs() < 1e-10);
}

#[test]
fn hypot() {
    let result = call(&MathModule, "hypot", &[n(3.0), n(4.0)]).as_number();
    assert!(result.is_some());
    assert!((result.unwrap() - 5.0).abs() < 1e-10);
}

#[test]
fn mod_op() {
    assert_eq!(
        call(&MathModule, "mod", &[n(10.0), n(3.0)]).as_number(),
        Some(1.0)
    );
}

#[test]
fn pi() {
    let result = call(&MathModule, "pi", &[]).as_number();
    assert!(result.is_some());
    assert!((result.unwrap() - std::f64::consts::PI).abs() < 1e-10);
}

#[test]
fn e() {
    let result = call(&MathModule, "e", &[]).as_number();
    assert!(result.is_some());
    assert!((result.unwrap() - std::f64::consts::E).abs() < 1e-10);
}

#[test]
fn log2() {
    assert_eq!(call(&MathModule, "log2", &[n(8.0)]).as_number(), Some(3.0));
}

#[test]
fn cbrt() {
    assert_eq!(call(&MathModule, "cbrt", &[n(27.0)]).as_number(), Some(3.0));
}

#[test]
fn degrees() {
    let result = call(&MathModule, "degrees", &[n(std::f64::consts::PI)]).as_number();
    assert!(result.is_some());
    assert!((result.unwrap() - 180.0).abs() < 1e-10);
}

#[test]
fn radians() {
    let result = call(&MathModule, "radians", &[n(180.0)]).as_number();
    assert!(result.is_some());
    assert!((result.unwrap() - std::f64::consts::PI).abs() < 1e-10);
}

#[test]
fn round_to_two_decimals() {
    let result = call(&MathModule, "round_to", &[n(3.14159), n(2.0)]).as_number();
    assert!(result.is_some());
    assert!((result.unwrap() - 3.14).abs() < 1e-10);
}

#[test]
fn round_to_zero_decimals() {
    let result = call(&MathModule, "round_to", &[n(3.7), n(0.0)]).as_number();
    assert_eq!(result, Some(4.0));
}

#[test]
fn round_to_three_decimals() {
    let result = call(&MathModule, "round_to", &[n(2.71828), n(3.0)]).as_number();
    assert!(result.is_some());
    assert!((result.unwrap() - 2.718).abs() < 1e-10);
}

#[test]
fn round_to_negative_errors() {
    let func = MathModule.function("round_to").unwrap();
    let mut ctx = CanonicalContext::new();
    let result = func.call(&[n(1.5), n(-1.0)], &mut ctx);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn unknown_function_returns_none() {
    assert!(MathModule.function("nope").is_none());
}

#[test]
fn type_mismatch_on_non_number() {
    let func = MathModule.function("abs").unwrap();
    let mut ctx = CanonicalContext::new();
    let result = func.call(&[NonFunctionValue::new("not a number")], &mut ctx);
    assert_eq!(
        result,
        Err(Error::TypeMismatch {
            expected: "number",
            got: "non-number"
        })
    );
}

#[test]
fn argument_count_on_missing_args() {
    let func = MathModule.function("abs").unwrap();
    let mut ctx = CanonicalContext::new();
    let result = func.call(&[], &mut ctx);
    assert_eq!(
        result,
        Err(Error::ArgumentCount {
            expected: 1,
            got: 0
        })
    );
}

// ── additional edge-case tests ─────────────────────────────────────

// Unary functions: missing args and type mismatch for all

#[test]
fn floor_no_args_errors() {
    let result = call_err(&MathModule, "floor", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn floor_non_number_errors() {
    let result = call_err(&MathModule, "floor", &[s("x")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn ceil_no_args_errors() {
    let result = call_err(&MathModule, "ceil", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn ceil_non_number_errors() {
    let result = call_err(&MathModule, "ceil", &[s("x")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn round_no_args_errors() {
    let result = call_err(&MathModule, "round", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn round_non_number_errors() {
    let result = call_err(&MathModule, "round", &[s("x")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn sqrt_no_args_errors() {
    let result = call_err(&MathModule, "sqrt", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn sqrt_non_number_errors() {
    let result = call_err(&MathModule, "sqrt", &[s("x")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn sign_no_args_errors() {
    let result = call_err(&MathModule, "sign", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn sign_non_number_errors() {
    let result = call_err(&MathModule, "sign", &[s("x")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn sign_negative_edge() {
    assert_eq!(
        call(&MathModule, "sign", &[n(-5.0)]).as_number(),
        Some(-1.0)
    );
}

#[test]
fn sign_zero_edge() {
    assert_eq!(call(&MathModule, "sign", &[n(0.0)]).as_number(), Some(1.0));
}

#[test]
fn trunc_no_args_errors() {
    let result = call_err(&MathModule, "trunc", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn trunc_non_number_errors() {
    let result = call_err(&MathModule, "trunc", &[s("x")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn log_no_args_errors() {
    let result = call_err(&MathModule, "log", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn log_non_number_errors() {
    let result = call_err(&MathModule, "log", &[s("x")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn log10_no_args_errors() {
    let result = call_err(&MathModule, "log10", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn log10_non_number_errors() {
    let result = call_err(&MathModule, "log10", &[s("x")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn exp_no_args_errors() {
    let result = call_err(&MathModule, "exp", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn exp_non_number_errors() {
    let result = call_err(&MathModule, "exp", &[s("x")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn sin_no_args_errors() {
    let result = call_err(&MathModule, "sin", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn sin_non_number_errors() {
    let result = call_err(&MathModule, "sin", &[s("x")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn cos_no_args_errors() {
    let result = call_err(&MathModule, "cos", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn cos_non_number_errors() {
    let result = call_err(&MathModule, "cos", &[s("x")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn tan_no_args_errors() {
    let result = call_err(&MathModule, "tan", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn tan_non_number_errors() {
    let result = call_err(&MathModule, "tan", &[s("x")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn asin_no_args_errors() {
    let result = call_err(&MathModule, "asin", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn asin_non_number_errors() {
    let result = call_err(&MathModule, "asin", &[s("x")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn acos_no_args_errors() {
    let result = call_err(&MathModule, "acos", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn acos_non_number_errors() {
    let result = call_err(&MathModule, "acos", &[s("x")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn atan_no_args_errors() {
    let result = call_err(&MathModule, "atan", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn atan_non_number_errors() {
    let result = call_err(&MathModule, "atan", &[s("x")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn log2_no_args_errors() {
    let result = call_err(&MathModule, "log2", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn log2_non_number_errors() {
    let result = call_err(&MathModule, "log2", &[s("x")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn cbrt_no_args_errors() {
    let result = call_err(&MathModule, "cbrt", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn cbrt_non_number_errors() {
    let result = call_err(&MathModule, "cbrt", &[s("x")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn cbrt_negative() {
    assert_eq!(
        call(&MathModule, "cbrt", &[n(-27.0)]).as_number(),
        Some(-3.0)
    );
}

#[test]
fn degrees_no_args_errors() {
    let result = call_err(&MathModule, "degrees", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn degrees_non_number_errors() {
    let result = call_err(&MathModule, "degrees", &[s("x")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn radians_no_args_errors() {
    let result = call_err(&MathModule, "radians", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn radians_non_number_errors() {
    let result = call_err(&MathModule, "radians", &[s("x")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// Binary functions: missing args and type mismatch

#[test]
fn min_no_args_errors() {
    let result = call_err(&MathModule, "min", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn min_one_arg_errors() {
    let result = call_err(&MathModule, "min", &[n(1.0)]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn min_first_arg_non_number_errors() {
    let result = call_err(&MathModule, "min", &[s("x"), n(1.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn min_second_arg_non_number_errors() {
    let result = call_err(&MathModule, "min", &[n(1.0), s("x")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn max_no_args_errors() {
    let result = call_err(&MathModule, "max", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn max_one_arg_errors() {
    let result = call_err(&MathModule, "max", &[n(1.0)]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn max_first_arg_non_number_errors() {
    let result = call_err(&MathModule, "max", &[s("x"), n(1.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn max_second_arg_non_number_errors() {
    let result = call_err(&MathModule, "max", &[n(1.0), s("x")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn pow_no_args_errors() {
    let result = call_err(&MathModule, "pow", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn pow_one_arg_errors() {
    let result = call_err(&MathModule, "pow", &[n(2.0)]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn pow_first_arg_non_number_errors() {
    let result = call_err(&MathModule, "pow", &[s("x"), n(2.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn pow_second_arg_non_number_errors() {
    let result = call_err(&MathModule, "pow", &[n(2.0), s("x")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn atan2_no_args_errors() {
    let result = call_err(&MathModule, "atan2", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn atan2_one_arg_errors() {
    let result = call_err(&MathModule, "atan2", &[n(1.0)]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn atan2_first_arg_non_number_errors() {
    let result = call_err(&MathModule, "atan2", &[s("x"), n(1.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn atan2_second_arg_non_number_errors() {
    let result = call_err(&MathModule, "atan2", &[n(1.0), s("x")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn hypot_no_args_errors() {
    let result = call_err(&MathModule, "hypot", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn hypot_one_arg_errors() {
    let result = call_err(&MathModule, "hypot", &[n(3.0)]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn hypot_first_arg_non_number_errors() {
    let result = call_err(&MathModule, "hypot", &[s("x"), n(4.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn hypot_second_arg_non_number_errors() {
    let result = call_err(&MathModule, "hypot", &[n(3.0), s("x")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn mod_op_no_args_errors() {
    let result = call_err(&MathModule, "mod", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn mod_op_one_arg_errors() {
    let result = call_err(&MathModule, "mod", &[n(10.0)]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn mod_op_first_arg_non_number_errors() {
    let result = call_err(&MathModule, "mod", &[s("x"), n(3.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn mod_op_second_arg_non_number_errors() {
    let result = call_err(&MathModule, "mod", &[n(10.0), s("x")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// Clamp: 3-arg function

#[test]
fn clamp_no_args_errors() {
    let result = call_err(&MathModule, "clamp", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn clamp_one_arg_errors() {
    let result = call_err(&MathModule, "clamp", &[n(5.0)]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn clamp_two_args_errors() {
    let result = call_err(&MathModule, "clamp", &[n(5.0), n(0.0)]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn clamp_first_arg_non_number_errors() {
    let result = call_err(&MathModule, "clamp", &[s("x"), n(0.0), n(10.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn clamp_second_arg_non_number_errors() {
    let result = call_err(&MathModule, "clamp", &[n(5.0), s("x"), n(10.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn clamp_third_arg_non_number_errors() {
    let result = call_err(&MathModule, "clamp", &[n(5.0), n(0.0), s("x")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn clamp_edge_values() {
    assert_eq!(
        call(&MathModule, "clamp", &[n(5.0), n(0.0), n(10.0)]).as_number(),
        Some(5.0)
    );
}

#[test]
fn clamp_below_min_edge() {
    assert_eq!(
        call(&MathModule, "clamp", &[n(-3.0), n(0.0), n(10.0)]).as_number(),
        Some(0.0)
    );
}

#[test]
fn clamp_above_max_edge() {
    assert_eq!(
        call(&MathModule, "clamp", &[n(15.0), n(0.0), n(10.0)]).as_number(),
        Some(10.0)
    );
}

// RoundTo: 2-arg function

#[test]
fn round_to_no_args_errors() {
    let result = call_err(&MathModule, "round_to", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn round_to_one_arg_errors() {
    let result = call_err(&MathModule, "round_to", &[n(1.5)]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn round_to_first_arg_non_number_errors() {
    let result = call_err(&MathModule, "round_to", &[s("x"), n(2.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn round_to_second_arg_non_number_errors() {
    let result = call_err(&MathModule, "round_to", &[n(1.5), s("x")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn round_to_zero_decimals_alt() {
    assert_eq!(
        call(&MathModule, "round_to", &[n(1.567), n(0.0)]).as_number(),
        Some(2.0)
    );
}

#[test]
fn round_to_two_decimals_alt() {
    assert_eq!(
        call(&MathModule, "round_to", &[n(1.567), n(2.0)]).as_number(),
        Some(1.57)
    );
}

#[test]
fn round_to_two_decimals_edge() {
    assert_eq!(
        call(&MathModule, "round_to", &[n(3.14159), n(2.0)]).as_number(),
        Some(3.14)
    );
}

#[test]
fn round_to_zero_decimals_edge() {
    assert_eq!(
        call(&MathModule, "round_to", &[n(3.7), n(0.0)]).as_number(),
        Some(4.0)
    );
}

// Pi and E constants

#[test]
fn pi_returns_constant() {
    assert_eq!(
        call(&MathModule, "pi", &[]).as_number(),
        Some(std::f64::consts::PI)
    );
}

#[test]
fn e_returns_constant() {
    assert_eq!(
        call(&MathModule, "e", &[]).as_number(),
        Some(std::f64::consts::E)
    );
}

// Edge values

#[test]
fn abs_zero() {
    assert_eq!(call(&MathModule, "abs", &[n(0.0)]).as_number(), Some(0.0));
}

#[test]
fn floor_already_integer() {
    assert_eq!(call(&MathModule, "floor", &[n(5.0)]).as_number(), Some(5.0));
}

#[test]
fn ceil_already_integer() {
    assert_eq!(call(&MathModule, "ceil", &[n(5.0)]).as_number(), Some(5.0));
}

#[test]
fn round_half_up() {
    assert_eq!(call(&MathModule, "round", &[n(2.5)]).as_number(), Some(3.0));
}

#[test]
fn round_half_down() {
    assert_eq!(call(&MathModule, "round", &[n(2.4)]).as_number(), Some(2.0));
}

#[test]
fn trunc_positive() {
    assert_eq!(call(&MathModule, "trunc", &[n(2.7)]).as_number(), Some(2.0));
}

#[test]
fn trunc_negative() {
    assert_eq!(
        call(&MathModule, "trunc", &[n(-2.7)]).as_number(),
        Some(-2.0)
    );
}

#[test]
fn sqrt_zero() {
    assert_eq!(call(&MathModule, "sqrt", &[n(0.0)]).as_number(), Some(0.0));
}

#[test]
fn pow_zero_exponent() {
    assert_eq!(
        call(&MathModule, "pow", &[n(5.0), n(0.0)]).as_number(),
        Some(1.0)
    );
}

#[test]
fn mod_op_normal() {
    assert_eq!(
        call(&MathModule, "mod", &[n(10.0), n(3.0)]).as_number(),
        Some(1.0)
    );
}

#[test]
fn degrees_zero() {
    assert_eq!(
        call(&MathModule, "degrees", &[n(0.0)]).as_number(),
        Some(0.0)
    );
}

#[test]
fn radians_zero() {
    assert_eq!(
        call(&MathModule, "radians", &[n(0.0)]).as_number(),
        Some(0.0)
    );
}

// ── is_nan / is_finite / is_infinite ───────────────────────────────
// Note: NaN and Infinity cannot be represented in JSON (serde_json
// serializes them as null). These predicates work correctly for all
// valid JSON numbers; is_nan/is_infinite will always return false and
// is_finite will always return true for representable values.

#[test]
fn is_nan_false() {
    assert_eq!(
        call(&MathModule, "is_nan", &[n(1.0)]).as_bool(),
        Some(false)
    );
}

#[test]
fn is_nan_zero() {
    assert_eq!(
        call(&MathModule, "is_nan", &[n(0.0)]).as_bool(),
        Some(false)
    );
}

#[test]
fn is_nan_large() {
    assert_eq!(
        call(&MathModule, "is_nan", &[n(1e308)]).as_bool(),
        Some(false)
    );
}

#[test]
fn is_finite_normal() {
    assert_eq!(
        call(&MathModule, "is_finite", &[n(1.0)]).as_bool(),
        Some(true)
    );
}

#[test]
fn is_finite_zero() {
    assert_eq!(
        call(&MathModule, "is_finite", &[n(0.0)]).as_bool(),
        Some(true)
    );
}

#[test]
fn is_finite_large() {
    assert_eq!(
        call(&MathModule, "is_finite", &[n(1e308)]).as_bool(),
        Some(true)
    );
}

#[test]
fn is_finite_negative() {
    assert_eq!(
        call(&MathModule, "is_finite", &[n(-1.0)]).as_bool(),
        Some(true)
    );
}

#[test]
fn is_infinite_false() {
    assert_eq!(
        call(&MathModule, "is_infinite", &[n(1.0)]).as_bool(),
        Some(false)
    );
}

#[test]
fn is_infinite_zero() {
    assert_eq!(
        call(&MathModule, "is_infinite", &[n(0.0)]).as_bool(),
        Some(false)
    );
}

#[test]
fn is_nan_no_args_errors() {
    let result = call_err(&MathModule, "is_nan", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn is_nan_non_number_errors() {
    let result = call_err(&MathModule, "is_nan", &[s("x")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn is_finite_no_args_errors() {
    let result = call_err(&MathModule, "is_finite", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn is_infinite_no_args_errors() {
    let result = call_err(&MathModule, "is_infinite", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

// ── is_integer ─────────────────────────────────────────────────────

#[test]
fn is_integer_true() {
    assert_eq!(
        call(&MathModule, "is_integer", &[n(5.0)]).as_bool(),
        Some(true)
    );
}

#[test]
fn is_integer_zero() {
    assert_eq!(
        call(&MathModule, "is_integer", &[n(0.0)]).as_bool(),
        Some(true)
    );
}

#[test]
fn is_integer_negative() {
    assert_eq!(
        call(&MathModule, "is_integer", &[n(-3.0)]).as_bool(),
        Some(true)
    );
}

#[test]
fn is_integer_false() {
    assert_eq!(
        call(&MathModule, "is_integer", &[n(3.7)]).as_bool(),
        Some(false)
    );
}

#[test]
fn is_integer_large() {
    assert_eq!(
        call(&MathModule, "is_integer", &[n(1e15)]).as_bool(),
        Some(true)
    );
}

#[test]
fn is_integer_no_args_errors() {
    let result = call_err(&MathModule, "is_integer", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn is_integer_non_number_errors() {
    let result = call_err(&MathModule, "is_integer", &[s("x")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── nan / infinity / tau constants ─────────────────────────────────
// Note: NaN and Infinity cannot be represented in JSON. The nan and
// infinity constants return null (serde_json's representation of these
// values).

#[test]
fn nan_constant() {
    assert!(call(&MathModule, "nan", &[]).is_null());
}

#[test]
fn infinity_constant() {
    assert!(call(&MathModule, "infinity", &[]).is_null());
}

#[test]
fn tau_constant() {
    let result = call(&MathModule, "tau", &[]).as_number();
    assert!(result.is_some());
    assert!((result.unwrap() - std::f64::consts::TAU).abs() < 1e-10);
}

#[test]
fn tau_is_two_pi() {
    let result = call(&MathModule, "tau", &[]).as_number();
    assert!(result.is_some());
    assert!((result.unwrap() - 2.0 * std::f64::consts::PI).abs() < 1e-10);
}

// ── lerp ───────────────────────────────────────────────────────────

#[test]
fn lerp_start() {
    assert_eq!(
        call(&MathModule, "lerp", &[n(0.0), n(10.0), n(0.0)]).as_number(),
        Some(0.0)
    );
}

#[test]
fn lerp_end() {
    assert_eq!(
        call(&MathModule, "lerp", &[n(0.0), n(10.0), n(1.0)]).as_number(),
        Some(10.0)
    );
}

#[test]
fn lerp_mid() {
    assert_eq!(
        call(&MathModule, "lerp", &[n(0.0), n(10.0), n(0.5)]).as_number(),
        Some(5.0)
    );
}

#[test]
fn lerp_negative_range() {
    assert_eq!(
        call(&MathModule, "lerp", &[n(-10.0), n(10.0), n(0.5)]).as_number(),
        Some(0.0)
    );
}

#[test]
fn lerp_no_args_errors() {
    let result = call_err(&MathModule, "lerp", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn lerp_two_args_errors() {
    let result = call_err(&MathModule, "lerp", &[n(0.0), n(10.0)]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn lerp_non_number_errors() {
    let result = call_err(&MathModule, "lerp", &[s("x"), n(10.0), n(0.5)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── map_range ──────────────────────────────────────────────────────

#[test]
fn map_range_basic() {
    let result = call(
        &MathModule,
        "map_range",
        &[n(5.0), n(0.0), n(10.0), n(0.0), n(100.0)],
    )
    .as_number();
    assert_eq!(result, Some(50.0));
}

#[test]
fn map_range_start() {
    let result = call(
        &MathModule,
        "map_range",
        &[n(0.0), n(0.0), n(10.0), n(0.0), n(100.0)],
    )
    .as_number();
    assert_eq!(result, Some(0.0));
}

#[test]
fn map_range_end() {
    let result = call(
        &MathModule,
        "map_range",
        &[n(10.0), n(0.0), n(10.0), n(0.0), n(100.0)],
    )
    .as_number();
    assert_eq!(result, Some(100.0));
}

#[test]
fn map_range_negative() {
    let result = call(
        &MathModule,
        "map_range",
        &[n(0.0), n(-10.0), n(10.0), n(0.0), n(100.0)],
    )
    .as_number();
    assert_eq!(result, Some(50.0));
}

#[test]
fn map_range_no_args_errors() {
    let result = call_err(&MathModule, "map_range", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn map_range_four_args_errors() {
    let result = call_err(&MathModule, "map_range", &[n(5.0), n(0.0), n(10.0), n(0.0)]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn map_range_non_number_errors() {
    let result = call_err(
        &MathModule,
        "map_range",
        &[s("x"), n(0.0), n(10.0), n(0.0), n(100.0)],
    );
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── fract ──────────────────────────────────────────────────────────

#[test]
fn fract_positive() {
    let result = call(&MathModule, "fract", &[n(3.7)]).as_number().unwrap();
    assert!((result - 0.7).abs() < 1e-10);
}

#[test]
fn fract_negative() {
    let result = call(&MathModule, "fract", &[n(-3.7)]).as_number().unwrap();
    assert!((result - (-0.7)).abs() < 1e-10);
}

#[test]
fn fract_integer() {
    assert_eq!(call(&MathModule, "fract", &[n(5.0)]).as_number(), Some(0.0));
}

#[test]
fn fract_no_args_errors() {
    let result = call_err(&MathModule, "fract", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn fract_non_number_errors() {
    let result = call_err(&MathModule, "fract", &[s("x")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── gcd / lcm ──────────────────────────────────────────────────────

#[test]
fn gcd_basic() {
    assert_eq!(
        call(&MathModule, "gcd", &[n(12.0), n(8.0)]).as_number(),
        Some(4.0)
    );
}

#[test]
fn gcd_with_one() {
    assert_eq!(
        call(&MathModule, "gcd", &[n(7.0), n(1.0)]).as_number(),
        Some(1.0)
    );
}

#[test]
fn gcd_same() {
    assert_eq!(
        call(&MathModule, "gcd", &[n(5.0), n(5.0)]).as_number(),
        Some(5.0)
    );
}

#[test]
fn gcd_zero() {
    assert_eq!(
        call(&MathModule, "gcd", &[n(0.0), n(5.0)]).as_number(),
        Some(5.0)
    );
}

#[test]
fn gcd_both_zero() {
    assert_eq!(
        call(&MathModule, "gcd", &[n(0.0), n(0.0)]).as_number(),
        Some(0.0)
    );
}

#[test]
fn lcm_basic() {
    assert_eq!(
        call(&MathModule, "lcm", &[n(4.0), n(6.0)]).as_number(),
        Some(12.0)
    );
}

#[test]
fn lcm_with_one() {
    assert_eq!(
        call(&MathModule, "lcm", &[n(7.0), n(1.0)]).as_number(),
        Some(7.0)
    );
}

#[test]
fn lcm_zero() {
    assert_eq!(
        call(&MathModule, "lcm", &[n(0.0), n(5.0)]).as_number(),
        Some(0.0)
    );
}

#[test]
fn gcd_no_args_errors() {
    let result = call_err(&MathModule, "gcd", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn gcd_one_arg_errors() {
    let result = call_err(&MathModule, "gcd", &[n(12.0)]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn gcd_non_number_errors() {
    let result = call_err(&MathModule, "gcd", &[s("x"), n(8.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn lcm_no_args_errors() {
    let result = call_err(&MathModule, "lcm", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn lcm_non_number_errors() {
    let result = call_err(&MathModule, "lcm", &[s("x"), n(6.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── sinh / cosh / tanh ─────────────────────────────────────────────

#[test]
fn sinh_zero() {
    let result = call(&MathModule, "sinh", &[n(0.0)]).as_number();
    assert!(result.unwrap().abs() < 1e-10);
}

#[test]
fn sinh_one() {
    let result = call(&MathModule, "sinh", &[n(1.0)]).as_number();
    assert!((result.unwrap() - 1.1752011936438014).abs() < 1e-10);
}

#[test]
fn cosh_zero() {
    let result = call(&MathModule, "cosh", &[n(0.0)]).as_number();
    assert!((result.unwrap() - 1.0).abs() < 1e-10);
}

#[test]
fn cosh_one() {
    let result = call(&MathModule, "cosh", &[n(1.0)]).as_number();
    assert!((result.unwrap() - 1.5430806348152437).abs() < 1e-10);
}

#[test]
fn tanh_zero() {
    let result = call(&MathModule, "tanh", &[n(0.0)]).as_number();
    assert!(result.unwrap().abs() < 1e-10);
}

#[test]
fn tanh_one() {
    let result = call(&MathModule, "tanh", &[n(1.0)]).as_number();
    assert!((result.unwrap() - 0.7615941559557649).abs() < 1e-10);
}

#[test]
fn sinh_no_args_errors() {
    let result = call_err(&MathModule, "sinh", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn cosh_no_args_errors() {
    let result = call_err(&MathModule, "cosh", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn tanh_no_args_errors() {
    let result = call_err(&MathModule, "tanh", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn sinh_non_number_errors() {
    let result = call_err(&MathModule, "sinh", &[s("x")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── asinh / acosh / atanh ──────────────────────────────────────────

#[test]
fn asinh_zero() {
    let result = call(&MathModule, "asinh", &[n(0.0)]).as_number();
    assert!(result.unwrap().abs() < 1e-10);
}

#[test]
fn asinh_one() {
    let result = call(&MathModule, "asinh", &[n(1.0)]).as_number();
    assert!((result.unwrap() - 0.881373587019543).abs() < 1e-10);
}

#[test]
fn acosh_one() {
    let result = call(&MathModule, "acosh", &[n(1.0)]).as_number();
    assert!(result.unwrap().abs() < 1e-10);
}

#[test]
fn acosh_two() {
    let result = call(&MathModule, "acosh", &[n(2.0)]).as_number();
    assert!((result.unwrap() - 1.3169578969248166).abs() < 1e-10);
}

#[test]
fn atanh_zero() {
    let result = call(&MathModule, "atanh", &[n(0.0)]).as_number();
    assert!(result.unwrap().abs() < 1e-10);
}

#[test]
fn atanh_half() {
    let result = call(&MathModule, "atanh", &[n(0.5)]).as_number();
    assert!((result.unwrap() - 0.5493061443340549).abs() < 1e-10);
}

#[test]
fn asinh_no_args_errors() {
    let result = call_err(&MathModule, "asinh", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn acosh_no_args_errors() {
    let result = call_err(&MathModule, "acosh", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn atanh_no_args_errors() {
    let result = call_err(&MathModule, "atanh", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn atanh_non_number_errors() {
    let result = call_err(&MathModule, "atanh", &[s("x")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── factorial ──────────────────────────────────────────────────────

#[test]
fn factorial_zero() {
    assert_eq!(
        call(&MathModule, "factorial", &[n(0.0)]).as_number(),
        Some(1.0)
    );
}

#[test]
fn factorial_one() {
    assert_eq!(
        call(&MathModule, "factorial", &[n(1.0)]).as_number(),
        Some(1.0)
    );
}

#[test]
fn factorial_five() {
    assert_eq!(
        call(&MathModule, "factorial", &[n(5.0)]).as_number(),
        Some(120.0)
    );
}

#[test]
fn factorial_ten() {
    assert_eq!(
        call(&MathModule, "factorial", &[n(10.0)]).as_number(),
        Some(3628800.0)
    );
}

#[test]
fn factorial_no_args_errors() {
    let result = call_err(&MathModule, "factorial", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn factorial_non_number_errors() {
    let result = call_err(&MathModule, "factorial", &[s("x")]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn factorial_negative_errors() {
    let result = call_err(&MathModule, "factorial", &[n(-1.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn factorial_non_integer_errors() {
    let result = call_err(&MathModule, "factorial", &[n(3.5)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

// ── floor_div ──────────────────────────────────────────────────────

#[test]
fn floor_div_positive() {
    assert_eq!(
        call(&MathModule, "floor_div", &[n(7.0), n(3.0)]).as_number(),
        Some(2.0)
    );
}

#[test]
fn floor_div_exact() {
    assert_eq!(
        call(&MathModule, "floor_div", &[n(6.0), n(3.0)]).as_number(),
        Some(2.0)
    );
}

#[test]
fn floor_div_negative_dividend() {
    assert_eq!(
        call(&MathModule, "floor_div", &[n(-7.0), n(3.0)]).as_number(),
        Some(-3.0)
    );
}

#[test]
fn floor_div_negative_divisor() {
    assert_eq!(
        call(&MathModule, "floor_div", &[n(7.0), n(-3.0)]).as_number(),
        Some(-3.0)
    );
}

#[test]
fn floor_div_both_negative() {
    assert_eq!(
        call(&MathModule, "floor_div", &[n(-7.0), n(-3.0)]).as_number(),
        Some(2.0)
    );
}

#[test]
fn floor_div_no_args_errors() {
    let result = call_err(&MathModule, "floor_div", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn floor_div_one_arg_errors() {
    let result = call_err(&MathModule, "floor_div", &[n(7.0)]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn floor_div_non_number_errors() {
    let result = call_err(&MathModule, "floor_div", &[s("x"), n(3.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn floor_div_zero_divisor_errors() {
    let result = call_err(&MathModule, "floor_div", &[n(7.0), n(0.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}
