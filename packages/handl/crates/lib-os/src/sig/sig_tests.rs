use handl_vm_common::Module;
use handl_vm_values_json::CanonicalContext;
use handl_vm_values_json::NonFunctionValue;

use super::sig::SigModule;
use super::sig_stub::StubSig;

fn call(module: &dyn Module, name: &str, args: &[NonFunctionValue]) -> NonFunctionValue {
    let func = module.function(name).expect("function exists");
    let mut ctx = CanonicalContext::new();
    func.call(args, &mut ctx).expect("call succeeds")
}

fn call_err(
    module: &dyn Module,
    name: &str,
    args: &[NonFunctionValue],
) -> handl_vm_values_json::Error {
    let func = module.function(name).expect("function exists");
    let mut ctx = CanonicalContext::new();
    func.call(args, &mut ctx).unwrap_err()
}

fn s(v: &str) -> NonFunctionValue {
    NonFunctionValue::new(v.to_string())
}

fn n(v: i64) -> NonFunctionValue {
    NonFunctionValue::new(v)
}

#[test]
fn module_name() {
    let module = SigModule::new(StubSig::default());
    assert_eq!(module.name(), "sig");
}

#[test]
fn list_signals() {
    let module = SigModule::new(StubSig::default());
    let result = call(&module, "list", &[]);
    let arr = result.as_array().unwrap();
    assert!(!arr.is_empty());
}

#[test]
fn send_signal() {
    let module = SigModule::new(StubSig::default());
    call(&module, "send", &[n(42), s("SIGINT")]);
}

#[test]
fn send_unknown_signal() {
    let module = SigModule::new(StubSig::default());
    let err = call_err(&module, "send", &[n(42), s("BOGUS")]);
    assert!(matches!(err, handl_vm_values_json::Error::PrimitiveError(_)));
}

#[test]
fn argument_count() {
    let module = SigModule::new(StubSig::default());
    let err = call_err(&module, "send", &[n(42)]);
    assert_eq!(
        err,
        handl_vm_values_json::Error::ArgumentCount {
            expected: 2,
            got: 1
        }
    );
}
