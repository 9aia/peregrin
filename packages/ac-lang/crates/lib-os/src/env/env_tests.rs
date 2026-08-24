use ac_runtime_values::NonFunctionValue;
use ac_runtime_values::CanonicalContext;
use ac_runtime_modules::Module;

use super::env::EnvModule;
use super::env_stub::StubEnv;

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
    let module = EnvModule::new(StubEnv::default());
    assert_eq!(module.name(), "env");
}

#[test]
fn get_existing() {
    let env = StubEnv::with("HOME", "/home/user");
    let module = EnvModule::new(env);
    assert_eq!(
        call(&module, "get", &[s("HOME")]).as_str(),
        Some("/home/user")
    );
}

#[test]
fn get_missing_returns_null() {
    let module = EnvModule::new(StubEnv::default());
    let result = call(&module, "get", &[s("NOPE")]);
    assert!(result.is_null());
}

#[test]
fn has_true() {
    let env = StubEnv::with("PATH", "/usr/bin");
    let module = EnvModule::new(env);
    assert_eq!(call(&module, "has", &[s("PATH")]).as_bool(), Some(true));
}

#[test]
fn has_false() {
    let module = EnvModule::new(StubEnv::default());
    assert_eq!(call(&module, "has", &[s("NOPE")]).as_bool(), Some(false));
}

#[test]
fn keys() {
    let env = StubEnv::default();
    {
        let mut vars = env.vars.lock().unwrap();
        vars.insert("A".into(), "1".into());
        vars.insert("B".into(), "2".into());
    }
    let module = EnvModule::new(env);
    let result = call(&module, "keys", &[]);
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 2);
}

#[test]
fn unknown_function() {
    let module = EnvModule::new(StubEnv::default());
    assert!(module.function("nope").is_none());
}