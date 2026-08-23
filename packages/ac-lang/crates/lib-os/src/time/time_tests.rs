use ac_runtime::NonFunctionValue;
use ac_runtime_modules::Module;
use ac_runtime::{CanonicalContext, Context};

use super::time::TimeModule;
use super::time_stub::StubTime;

fn call(module: &dyn Module, name: &str, args: &[NonFunctionValue]) -> NonFunctionValue {
    let func = module.function(name).expect("function exists");
    let mut ctx = CanonicalContext::new();
    func.call(args, &mut ctx).expect("call succeeds")
}

#[test]
fn module_name() {
    let module = TimeModule::new(StubTime::default());
    assert_eq!(module.name(), "time");
}

#[test]
fn now() {
    let module = TimeModule::new(StubTime::new(1_000, 1));
    assert_eq!(call(&module, "now", &[]).as_number(), Some(1000.0));
}

#[test]
fn epoch() {
    let module = TimeModule::new(StubTime::new(1_000, 42));
    assert_eq!(call(&module, "epoch", &[]).as_number(), Some(42.0));
}

#[test]
fn unknown_function() {
    let module = TimeModule::new(StubTime::default());
    assert!(module.function("nope").is_none());
}
