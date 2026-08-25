use handl_vm_common::Module;
use handl_vm_values_json::{CanonicalContext, Error, NonFunctionValue};

use super::ResultModule;

fn n(v: f64) -> NonFunctionValue {
    NonFunctionValue::new(serde_json::json!(v))
}

fn s(v: &str) -> NonFunctionValue {
    NonFunctionValue::new(v.to_string())
}

fn call(name: &str, args: &[NonFunctionValue]) -> NonFunctionValue {
    let func = ResultModule.function(name).expect("function exists");
    let mut ctx = CanonicalContext::new();
    func.call(args, &mut ctx).expect("call succeeds")
}

fn call_err(name: &str, args: &[NonFunctionValue]) -> Result<NonFunctionValue, Error> {
    let func = ResultModule.function(name).expect("function exists");
    let mut ctx = CanonicalContext::new();
    func.call(args, &mut ctx)
}

// ── constructors ───────────────────────────────────────────────────

#[test]
fn ok_wraps_value() {
    let value = n(42.0);
    let result = call("ok", &[value]);
    assert_eq!(
        result.inner(),
        &serde_json::json!({"_tag": "Ok", "value": 42.0})
    );
}

#[test]
fn err_wraps_error() {
    let error = s("nope");
    let result = call("err", &[error]);
    assert_eq!(
        result.inner(),
        &serde_json::json!({"_tag": "Err", "error": "nope"})
    );
}

#[test]
fn constructors_require_one_arg() {
    assert!(matches!(
        call_err("ok", &[]),
        Err(Error::ArgumentCount { .. })
    ));
    assert!(matches!(
        call_err("err", &[]),
        Err(Error::ArgumentCount { .. })
    ));
}

// ── predicates ─────────────────────────────────────────────────────

#[test]
fn is_ok_true_for_ok() {
    let ok = call("ok", &[n(1.0)]);
    let result = call("is_ok", &[ok]);
    assert_eq!(result.as_bool(), Some(true));
}

#[test]
fn is_ok_false_for_err() {
    let err = call("err", &[s("bad")]);
    let result = call("is_ok", &[err]);
    assert_eq!(result.as_bool(), Some(false));
}

#[test]
fn is_ok_false_for_non_result() {
    let result = call("is_ok", &[n(42.0)]);
    assert_eq!(result.as_bool(), Some(false));
}

#[test]
fn is_err_true_for_err() {
    let err = call("err", &[s("bad")]);
    let result = call("is_err", &[err]);
    assert_eq!(result.as_bool(), Some(true));
}

#[test]
fn is_err_false_for_ok() {
    let ok = call("ok", &[n(1.0)]);
    let result = call("is_err", &[ok]);
    assert_eq!(result.as_bool(), Some(false));
}

#[test]
fn is_err_false_for_non_result() {
    let result = call("is_err", &[n(42.0)]);
    assert_eq!(result.as_bool(), Some(false));
}

#[test]
fn predicates_require_one_arg() {
    assert!(matches!(
        call_err("is_ok", &[]),
        Err(Error::ArgumentCount { .. })
    ));
    assert!(matches!(
        call_err("is_err", &[]),
        Err(Error::ArgumentCount { .. })
    ));
}
