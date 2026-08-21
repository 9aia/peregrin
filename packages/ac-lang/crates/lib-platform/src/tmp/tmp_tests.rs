use ac_runtime::RuntimeValue;
use ac_runtime_core::Module;
use ac_vm::Context;

use super::tmp::TmpModule;
use super::tmp_stub::StubTmp;

fn call(module: &dyn Module, name: &str, args: &[RuntimeValue]) -> RuntimeValue {
    let func = module.function(name).expect("function exists");
    let mut ctx = Context::new();
    func.call(args, &mut ctx).expect("call succeeds")
}

#[test]
fn module_name() {
    let module = TmpModule::new(StubTmp::default());
    assert_eq!(module.name(), "tmp");
}

#[test]
fn file_returns_path() {
    let module = TmpModule::new(StubTmp::default());
    let result = call(&module, "file", &[]);
    assert!(result.as_str().unwrap().starts_with("/tmp/stub-file-"));
}

#[test]
fn dir_returns_path() {
    let module = TmpModule::new(StubTmp::default());
    let result = call(&module, "dir", &[]);
    assert!(result.as_str().unwrap().starts_with("/tmp/stub-dir-"));
}

#[test]
fn file_and_dir_are_unique() {
    let module = TmpModule::new(StubTmp::default());
    let a = call(&module, "file", &[]).as_str().unwrap().to_string();
    let b = call(&module, "file", &[]).as_str().unwrap().to_string();
    assert_ne!(a, b);
}

#[test]
fn unknown_function() {
    let module = TmpModule::new(StubTmp::default());
    assert!(module.function("nope").is_none());
}
