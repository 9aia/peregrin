use ac_runtime::RuntimeValue;
use ac_runtime_core::Module;
use ac_vm::{Context, Error};

use super::bool::BoolModule;

fn call(module: &dyn Module, name: &str, args: &[RuntimeValue]) -> RuntimeValue {
    let func = module.function(name).expect("function exists");
    let mut ctx = Context::new();
    func.call(args, &mut ctx).expect("call succeeds")
}

fn b(v: bool) -> RuntimeValue {
    RuntimeValue::new(v)
}

fn n(v: f64) -> RuntimeValue {
    RuntimeValue::new(serde_json::json!(v))
}

fn s(v: &str) -> RuntimeValue {
    RuntimeValue::new(v.to_string())
}

fn null() -> RuntimeValue {
    RuntimeValue::new(serde_json::Value::Null)
}

fn arr(items: &[serde_json::Value]) -> RuntimeValue {
    RuntimeValue::new(serde_json::Value::Array(items.to_vec()))
}

#[test]
fn module_name() {
    assert_eq!(BoolModule.name(), "bool");
}

#[test]
fn and_true() {
    assert_eq!(
        call(&BoolModule, "and", &[b(true), b(true)]).as_bool(),
        Some(true)
    );
}

#[test]
fn and_false() {
    assert_eq!(
        call(&BoolModule, "and", &[b(true), b(false)]).as_bool(),
        Some(false)
    );
    assert_eq!(
        call(&BoolModule, "and", &[b(false), b(true)]).as_bool(),
        Some(false)
    );
    assert_eq!(
        call(&BoolModule, "and", &[b(false), b(false)]).as_bool(),
        Some(false)
    );
}

#[test]
fn or_true() {
    assert_eq!(
        call(&BoolModule, "or", &[b(true), b(false)]).as_bool(),
        Some(true)
    );
    assert_eq!(
        call(&BoolModule, "or", &[b(false), b(true)]).as_bool(),
        Some(true)
    );
    assert_eq!(
        call(&BoolModule, "or", &[b(true), b(true)]).as_bool(),
        Some(true)
    );
}

#[test]
fn or_false() {
    assert_eq!(
        call(&BoolModule, "or", &[b(false), b(false)]).as_bool(),
        Some(false)
    );
}

#[test]
fn not_true() {
    assert_eq!(call(&BoolModule, "not", &[b(true)]).as_bool(), Some(false));
}

#[test]
fn not_false() {
    assert_eq!(call(&BoolModule, "not", &[b(false)]).as_bool(), Some(true));
}

#[test]
fn xor_true() {
    assert_eq!(
        call(&BoolModule, "xor", &[b(true), b(false)]).as_bool(),
        Some(true)
    );
    assert_eq!(
        call(&BoolModule, "xor", &[b(false), b(true)]).as_bool(),
        Some(true)
    );
}

#[test]
fn xor_false() {
    assert_eq!(
        call(&BoolModule, "xor", &[b(true), b(true)]).as_bool(),
        Some(false)
    );
    assert_eq!(
        call(&BoolModule, "xor", &[b(false), b(false)]).as_bool(),
        Some(false)
    );
}

#[test]
fn nand_true() {
    assert_eq!(
        call(&BoolModule, "nand", &[b(true), b(false)]).as_bool(),
        Some(true)
    );
    assert_eq!(
        call(&BoolModule, "nand", &[b(false), b(false)]).as_bool(),
        Some(true)
    );
}

#[test]
fn nand_false() {
    assert_eq!(
        call(&BoolModule, "nand", &[b(true), b(true)]).as_bool(),
        Some(false)
    );
}

#[test]
fn nor_true() {
    assert_eq!(
        call(&BoolModule, "nor", &[b(false), b(false)]).as_bool(),
        Some(true)
    );
}

#[test]
fn nor_false() {
    assert_eq!(
        call(&BoolModule, "nor", &[b(true), b(false)]).as_bool(),
        Some(false)
    );
    assert_eq!(
        call(&BoolModule, "nor", &[b(true), b(true)]).as_bool(),
        Some(false)
    );
}

#[test]
fn if_true() {
    assert_eq!(
        call(&BoolModule, "if", &[b(false), b(false)]).as_bool(),
        Some(true)
    );
    assert_eq!(
        call(&BoolModule, "if", &[b(false), b(true)]).as_bool(),
        Some(true)
    );
    assert_eq!(
        call(&BoolModule, "if", &[b(true), b(true)]).as_bool(),
        Some(true)
    );
}

#[test]
fn if_false() {
    assert_eq!(
        call(&BoolModule, "if", &[b(true), b(false)]).as_bool(),
        Some(false)
    );
}

#[test]
fn eq_true() {
    assert_eq!(
        call(&BoolModule, "eq", &[b(true), b(true)]).as_bool(),
        Some(true)
    );
    assert_eq!(
        call(&BoolModule, "eq", &[b(false), b(false)]).as_bool(),
        Some(true)
    );
}

#[test]
fn eq_false() {
    assert_eq!(
        call(&BoolModule, "eq", &[b(true), b(false)]).as_bool(),
        Some(false)
    );
    assert_eq!(
        call(&BoolModule, "eq", &[b(false), b(true)]).as_bool(),
        Some(false)
    );
}

#[test]
fn from_bool() {
    assert_eq!(call(&BoolModule, "from", &[b(true)]).as_bool(), Some(true));
    assert_eq!(
        call(&BoolModule, "from", &[b(false)]).as_bool(),
        Some(false)
    );
}

#[test]
fn from_number() {
    assert_eq!(call(&BoolModule, "from", &[n(1.0)]).as_bool(), Some(true));
    assert_eq!(call(&BoolModule, "from", &[n(0.0)]).as_bool(), Some(false));
}

#[test]
fn from_string() {
    assert_eq!(
        call(&BoolModule, "from", &[s("hello")]).as_bool(),
        Some(true)
    );
    assert_eq!(call(&BoolModule, "from", &[s("")]).as_bool(), Some(false));
}

#[test]
fn from_null() {
    assert_eq!(call(&BoolModule, "from", &[null()]).as_bool(), Some(false));
}

#[test]
fn from_array() {
    assert_eq!(
        call(&BoolModule, "from", &[arr(&[serde_json::json!(1)])]).as_bool(),
        Some(true)
    );
    assert_eq!(
        call(&BoolModule, "from", &[arr(&[])]).as_bool(),
        Some(false)
    );
}

#[test]
fn unknown_function_returns_none() {
    assert!(BoolModule.function("nope").is_none());
}

#[test]
fn type_mismatch_on_non_bool() {
    let func = BoolModule.function("not").unwrap();
    let mut ctx = Context::new();
    let result = func.call(&[n(42.0)], &mut ctx);
    assert_eq!(
        result,
        Err(Error::TypeMismatch {
            expected: "boolean",
            got: "non-boolean",
        })
    );
}

#[test]
fn argument_count_on_missing_args() {
    let func = BoolModule.function("not").unwrap();
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
