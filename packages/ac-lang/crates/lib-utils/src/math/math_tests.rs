use ac_runtime::RuntimeValue;
use ac_runtime_core::Module;
use ac_vm::{Context, Error};

use super::math::MathModule;

fn call(module: &dyn Module, name: &str, args: &[RuntimeValue]) -> RuntimeValue {
    let func = module.function(name).expect("function exists");
    let mut ctx = Context::new();
    func.call(args, &mut ctx).expect("call succeeds")
}

fn n(v: f64) -> RuntimeValue {
    RuntimeValue::new(serde_json::json!(v))
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
    assert_eq!(
        call(&MathModule, "sqrt", &[n(9.0)]).as_number(),
        Some(3.0)
    );
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
    assert_eq!(call(&MathModule, "sign", &[n(-5.0)]).as_number(), Some(-1.0));
}

#[test]
fn sign_zero() {
    assert_eq!(call(&MathModule, "sign", &[n(0.0)]).as_number(), Some(1.0));
}

#[test]
fn trunc() {
    assert_eq!(call(&MathModule, "trunc", &[n(3.7)]).as_number(), Some(3.0));
    assert_eq!(call(&MathModule, "trunc", &[n(-3.7)]).as_number(), Some(-3.0));
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
    assert_eq!(call(&MathModule, "log10", &[n(100.0)]).as_number(), Some(2.0));
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
fn unknown_function_returns_none() {
    assert!(MathModule.function("nope").is_none());
}

#[test]
fn type_mismatch_on_non_number() {
    let func = MathModule.function("abs").unwrap();
    let mut ctx = Context::new();
    let result = func.call(&[RuntimeValue::new("not a number")], &mut ctx);
    assert_eq!(
        result,
        Err(Error::TypeMismatch {
            expected: "number",
            got: "non-number",
        })
    );
}

#[test]
fn argument_count_on_missing_args() {
    let func = MathModule.function("abs").unwrap();
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
