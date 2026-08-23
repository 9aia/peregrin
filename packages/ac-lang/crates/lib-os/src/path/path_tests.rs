use ac_runtime::NonFunctionValue;
use ac_runtime::{CanonicalContext, Context};
use ac_runtime_modules::Module;

use super::path::PathModule;
use super::path_stub::StubPath;

fn call(module: &dyn Module, name: &str, args: &[NonFunctionValue]) -> NonFunctionValue {
    let func = module.function(name).expect("function exists");
    let mut ctx = CanonicalContext::new();
    func.call(args, &mut ctx).expect("call succeeds")
}

fn s(v: &str) -> NonFunctionValue {
    NonFunctionValue::new(v.to_string())
}

#[test]
fn module_name() {
    let module = PathModule::new(StubPath::default());
    assert_eq!(module.name(), "path");
}

#[test]
fn join() {
    let module = PathModule::new(StubPath::default());
    assert_eq!(
        call(&module, "join", &[s("a"), s("b"), s("c")]).as_str(),
        Some("a/b/c")
    );
}

#[test]
fn resolve_absolute() {
    let module = PathModule::new(StubPath::default());
    assert_eq!(
        call(&module, "resolve", &[s("/base"), s("rel")]).as_str(),
        Some("/base/rel")
    );
}

#[test]
fn resolve_relative() {
    let module = PathModule::new(StubPath::default());
    assert_eq!(call(&module, "resolve", &[s("rel")]).as_str(), Some("/rel"));
}

#[test]
fn basename() {
    let module = PathModule::new(StubPath::default());
    assert_eq!(
        call(&module, "basename", &[s("/path/to/file.txt")]).as_str(),
        Some("file.txt")
    );
}

#[test]
fn dirname() {
    let module = PathModule::new(StubPath::default());
    assert_eq!(
        call(&module, "dirname", &[s("/path/to/file.txt")]).as_str(),
        Some("/path/to")
    );
}

#[test]
fn extname() {
    let module = PathModule::new(StubPath::default());
    assert_eq!(
        call(&module, "extname", &[s("file.txt")]).as_str(),
        Some(".txt")
    );
}

#[test]
fn extname_no_extension() {
    let module = PathModule::new(StubPath::default());
    assert_eq!(call(&module, "extname", &[s("README")]).as_str(), Some(""));
}

#[test]
fn unknown_function() {
    let module = PathModule::new(StubPath::default());
    assert!(module.function("nope").is_none());
}
