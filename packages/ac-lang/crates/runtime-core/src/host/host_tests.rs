use ac_runtime::RuntimeValue;
use ac_vm::{Context, Host, Vm};

use crate::module::module_stub::StubModule;
use crate::host::ModuleHost;

use ac_ast::Expr;

fn make_call(name: &str, args: &[RuntimeValue]) -> Expr<RuntimeValue> {
    Expr::Call {
        name: name.to_string(),
        args: args.iter().map(|v| Expr::Value(v.clone())).collect(),
    }
}

#[test]
fn empty_host_returns_none() {
    let host = ModuleHost::new();
    assert!(host.function("anything").is_none());
}

#[test]
fn qualified_name_resolves() {
    let host = ModuleHost::new().with(StubModule);
    assert!(host.function("stub.echo").is_some());
}

#[test]
fn unqualified_name_resolves() {
    let host = ModuleHost::new().with(StubModule);
    assert!(host.function("echo").is_some());
}

#[test]
fn unknown_qualified_returns_none() {
    let host = ModuleHost::new().with(StubModule);
    assert!(host.function("stub.nope").is_none());
    assert!(host.function("other.echo").is_none());
}

#[test]
fn unknown_unqualified_returns_none() {
    let host = ModuleHost::new().with(StubModule);
    assert!(host.function("nope").is_none());
}

#[test]
fn module_count() {
    let host = ModuleHost::new().with(StubModule);
    assert_eq!(host.module_count(), 1);
}

#[test]
fn vm_evaluates_qualified_call() {
    let mut vm = Vm::new(ModuleHost::new().with(StubModule));
    let mut ctx = Context::new();

    let expr = make_call("stub.echo", &[RuntimeValue::new("hello")]);
    let result = vm.eval(&expr, &mut ctx).unwrap();
    assert_eq!(result.as_str(), Some("hello"));
}

#[test]
fn vm_evaluates_unqualified_call() {
    let mut vm = Vm::new(ModuleHost::new().with(StubModule));
    let mut ctx = Context::new();

    let expr = make_call("echo", &[RuntimeValue::new(42)]);
    let result = vm.eval(&expr, &mut ctx).unwrap();
    assert_eq!(result.as_number(), Some(42.0));
}
