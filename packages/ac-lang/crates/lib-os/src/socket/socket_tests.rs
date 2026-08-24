use ac_runtime_values::NonFunctionValue;
use ac_runtime_values::CanonicalContext;
use ac_runtime_modules::Module;

use super::socket::SocketModule;
use super::socket_stub::StubSocket;

fn call(module: &dyn Module, name: &str, args: &[NonFunctionValue]) -> NonFunctionValue {
    let func = module.function(name).expect("function exists");
    let mut ctx = CanonicalContext::new();
    func.call(args, &mut ctx).expect("call succeeds")
}

fn call_err(module: &dyn Module, name: &str, args: &[NonFunctionValue]) -> ac_runtime::Error {
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
    let module = SocketModule::new(StubSocket::default());
    assert_eq!(module.name(), "socket");
}

#[test]
fn connect_and_recv() {
    let socket = StubSocket::with_binding("echo://test", "hello");
    let module = SocketModule::new(socket);
    let handle = call(&module, "connect", &[s("echo://test")])
        .as_number()
        .unwrap() as i64;
    let result = call(&module, "recv", &[n(handle), n(1024)]);
    assert_eq!(result.as_str(), Some("hello"));
}

#[test]
fn send_and_close() {
    let socket = StubSocket::default();
    let module = SocketModule::new(socket);
    let handle = call(&module, "connect", &[s("echo://test")])
        .as_number()
        .unwrap() as i64;
    let sent = call(&module, "send", &[n(handle), s("data")])
        .as_number()
        .unwrap() as usize;
    assert_eq!(sent, 4);
    call(&module, "close", &[n(handle)]);
    let err = call_err(&module, "recv", &[n(handle), n(1024)]);
    assert!(matches!(err, ac_runtime::Error::PrimitiveError(_)));
}

#[test]
fn unknown_function() {
    let module = SocketModule::new(StubSocket::default());
    assert!(module.function("nope").is_none());
}

#[test]
fn argument_count() {
    let module = SocketModule::new(StubSocket::default());
    let err = call_err(&module, "connect", &[]);
    assert_eq!(
        err,
        ac_runtime::Error::ArgumentCount {
            expected: 1,
            got: 0
        }
    );
}