use handl_vm_common::Module;
use handl_vm_values_json::CanonicalContext;
use handl_vm_values_json::NonFunctionValue;

use super::process::ProcessModule;
use super::process_stub::StubProcess;

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
    let module = ProcessModule::new(StubProcess::default());
    assert_eq!(module.name(), "process");
}

#[test]
fn spawn() {
    let process = StubProcess::default();
    let module = ProcessModule::new(process);
    let result = call(&module, "spawn", &[s("echo"), s("hello")]);
    assert!(result.as_number().unwrap() > 0.0);
}

#[test]
fn spawn_and_status() {
    let process = StubProcess::default();
    let module = ProcessModule::new(process);
    let pid = call(&module, "spawn", &[s("echo"), s("hello")])
        .as_number()
        .unwrap() as i64;
    let status = call(&module, "status", &[n(pid)]);
    let obj = status.inner();
    assert_eq!(obj["process_id"], pid);
    assert_eq!(obj["running"], true);
    assert!(obj["exit_code"].is_null());
}

#[test]
fn kill() {
    let process = StubProcess::default();
    let module = ProcessModule::new(process);
    let pid = call(&module, "spawn", &[s("echo")]).as_number().unwrap() as i64;
    call(&module, "kill", &[n(pid)]);
    let status = call(&module, "status", &[n(pid)]);
    let obj = status.inner();
    assert_eq!(obj["running"], false);
    assert_eq!(obj["exit_code"], 137);
}

#[test]
fn wait() {
    let process = StubProcess::default();
    let module = ProcessModule::new(process);
    let pid = call(&module, "spawn", &[s("echo")]).as_number().unwrap() as i64;
    let code = call(&module, "wait", &[n(pid)]);
    assert_eq!(code.as_number().unwrap(), 0.0);
}

#[test]
fn status_finished() {
    let process = StubProcess::with(42, "echo", vec!["hello".into()], 7, false);
    let module = ProcessModule::new(process);
    let status = call(&module, "status", &[n(42)]);
    let obj = status.inner();
    assert_eq!(obj["running"], false);
    assert_eq!(obj["exit_code"], 7);
}

#[test]
fn kill_not_found() {
    let module = ProcessModule::new(StubProcess::default());
    let err = call_err(&module, "kill", &[n(1)]);
    assert!(matches!(err, handl_vm_values_json::Error::PrimitiveError(_)));
}

#[test]
fn wait_not_found() {
    let module = ProcessModule::new(StubProcess::default());
    let err = call_err(&module, "wait", &[n(1)]);
    assert!(matches!(err, handl_vm_values_json::Error::PrimitiveError(_)));
}

#[test]
fn status_not_found() {
    let module = ProcessModule::new(StubProcess::default());
    let err = call_err(&module, "status", &[n(1)]);
    assert!(matches!(err, handl_vm_values_json::Error::PrimitiveError(_)));
}

#[test]
fn unknown_function() {
    let module = ProcessModule::new(StubProcess::default());
    assert!(module.function("nope").is_none());
}

#[test]
fn spawn_missing_program() {
    let module = ProcessModule::new(StubProcess::default());
    let err = call_err(&module, "spawn", &[]);
    assert_eq!(
        err,
        handl_vm_values_json::Error::ArgumentCount {
            expected: 1,
            got: 0,
        }
    );
}

#[test]
fn spawn_non_string_arg() {
    let module = ProcessModule::new(StubProcess::default());
    let err = call_err(&module, "spawn", &[n(1)]);
    assert_eq!(
        err,
        handl_vm_values_json::Error::TypeMismatch {
            expected: "string",
            got: "non-string",
        }
    );
}

#[test]
fn kill_non_number() {
    let module = ProcessModule::new(StubProcess::default());
    let err = call_err(&module, "kill", &[s("one")]);
    assert_eq!(
        err,
        handl_vm_values_json::Error::TypeMismatch {
            expected: "number",
            got: "non-number",
        }
    );
}

#[test]
fn wait_missing_pid() {
    let module = ProcessModule::new(StubProcess::default());
    let err = call_err(&module, "wait", &[]);
    assert_eq!(
        err,
        handl_vm_values_json::Error::ArgumentCount {
            expected: 1,
            got: 0,
        }
    );
}

#[test]
fn status_missing_pid() {
    let module = ProcessModule::new(StubProcess::default());
    let err = call_err(&module, "status", &[]);
    assert_eq!(
        err,
        handl_vm_values_json::Error::ArgumentCount {
            expected: 1,
            got: 0,
        }
    );
}
