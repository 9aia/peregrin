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
fn add_positive() {
    assert_eq!(
        call(&MathModule, "add", &[n(2.0), n(3.0)]).as_number(),
        Some(5.0)
    );
}

#[test]
fn add_negative() {
    assert_eq!(
        call(&MathModule, "add", &[n(-2.0), n(3.0)]).as_number(),
        Some(1.0)
    );
}

#[test]
fn add_no_args_errors() {
    let result = call_err(&MathModule, "add", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn add_one_arg_errors() {
    let result = call_err(&MathModule, "add", &[n(2.0)]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn add_non_number_errors() {
    let result = call_err(&MathModule, "add", &[s("x"), n(3.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn sub_positive() {
    assert_eq!(
        call(&MathModule, "sub", &[n(5.0), n(3.0)]).as_number(),
        Some(2.0)
    );
}

#[test]
fn sub_negative() {
    assert_eq!(
        call(&MathModule, "sub", &[n(2.0), n(5.0)]).as_number(),
        Some(-3.0)
    );
}

#[test]
fn sub_no_args_errors() {
    let result = call_err(&MathModule, "sub", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn sub_non_number_errors() {
    let result = call_err(&MathModule, "sub", &[s("x"), n(3.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn mul_positive() {
    assert_eq!(
        call(&MathModule, "mul", &[n(4.0), n(3.0)]).as_number(),
        Some(12.0)
    );
}

#[test]
fn mul_negative() {
    assert_eq!(
        call(&MathModule, "mul", &[n(-2.0), n(3.0)]).as_number(),
        Some(-6.0)
    );
}

#[test]
fn mul_no_args_errors() {
    let result = call_err(&MathModule, "mul", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn div_positive() {
    assert_eq!(
        call(&MathModule, "div", &[n(12.0), n(3.0)]).as_number(),
        Some(4.0)
    );
}

#[test]
fn div_negative() {
    assert_eq!(
        call(&MathModule, "div", &[n(-12.0), n(3.0)]).as_number(),
        Some(-4.0)
    );
}

#[test]
fn div_by_zero_errors() {
    let result = call_err(&MathModule, "div", &[n(1.0), n(0.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}

#[test]
fn div_no_args_errors() {
    let result = call_err(&MathModule, "div", &[]);
    assert!(matches!(result, Err(Error::ArgumentCount { .. })));
}

#[test]
fn div_non_number_errors() {
    let result = call_err(&MathModule, "div", &[s("x"), n(3.0)]);
    assert!(matches!(result, Err(Error::TypeMismatch { .. })));
}
