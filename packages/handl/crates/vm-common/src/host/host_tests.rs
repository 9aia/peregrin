use handl_vm::Host;
use handl_vm_backend_rust_interpreter::{CanonicalContext, Vm};
use handl_vm_values_json::NonFunctionValue;

use crate::host::ModuleHost;
use crate::module::module_stub::StubModule;

use handl_ast_canonical::Expr;

fn make_call(name: &str, args: &[NonFunctionValue]) -> Expr<NonFunctionValue> {
    Expr::Call {
        name: name.to_string(),
        args: args.iter().map(|v| Expr::Value(v.clone())).collect(),
        type_args: Vec::new(),
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
fn qualified_async_resolves() {
    let host = ModuleHost::new().with(StubModule);
    assert!(host.async_function("stub.async_echo").is_some());
}

#[test]
fn unqualified_async_resolves() {
    let host = ModuleHost::new().with(StubModule);
    assert!(host.async_function("async_echo").is_some());
}

#[test]
fn qualified_operation_resolves() {
    let host = ModuleHost::new().with(StubModule);
    assert!(host.operation("stub.raise").is_some());
}

#[test]
fn unqualified_operation_resolves() {
    let host = ModuleHost::new().with(StubModule);
    assert!(host.operation("raise").is_some());
}

#[test]
fn module_count() {
    let host = ModuleHost::new().with(StubModule);
    assert_eq!(host.module_count(), 1);
}

#[test]
fn vm_evaluates_qualified_call() {
    let vm = Vm::new(ModuleHost::new().with(StubModule));
    let mut ctx = CanonicalContext::new();

    let expr = make_call("stub.echo", &[NonFunctionValue::new("hello")]);
    let result = vm.eval(&expr, &mut ctx).unwrap();
    assert_eq!(result.as_str(), Some("hello"));
}

#[test]
fn vm_evaluates_unqualified_call() {
    let vm = Vm::new(ModuleHost::new().with(StubModule));
    let mut ctx = CanonicalContext::new();

    let expr = make_call("echo", &[NonFunctionValue::new(42)]);
    let result = vm.eval(&expr, &mut ctx).unwrap();
    assert_eq!(result.as_number(), Some(42.0));
}
