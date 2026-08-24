use ac_runtime_values::NonFunctionValue;
use ac_runtime_values::CanonicalContext;
use ac_runtime_modules::Module;

use super::cmd::{CmdModule, CmdOutput};
use super::cmd_stub::StubCmd;

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
    let module = CmdModule::new(StubCmd::default());
    assert_eq!(module.name(), "cmd");
}

#[test]
fn run() {
    let cmd = StubCmd::with(
        "echo",
        CmdOutput {
            exit_code: 0,
            stdout: "hello".into(),
            stderr: "".into(),
        },
    );
    let module = CmdModule::new(cmd);
    let result = call(&module, "run", &[s("echo"), s("hello")]);
    let obj = result.inner();
    assert_eq!(obj["exit_code"], 0);
    assert_eq!(obj["stdout"], "hello");
    assert_eq!(obj["stderr"], "");
}

#[test]
fn run_with_args() {
    let cmd = StubCmd::with(
        "ls",
        CmdOutput {
            exit_code: 0,
            stdout: "file1\nfile2".into(),
            stderr: "".into(),
        },
    );
    let module = CmdModule::new(cmd);
    let result = call(&module, "run", &[s("ls"), s("-la"), s("/tmp")]);
    let obj = result.inner();
    assert_eq!(obj["stdout"], "file1\nfile2");
}

#[test]
fn run_nonzero_exit() {
    let cmd = StubCmd::with(
        "false",
        CmdOutput {
            exit_code: 1,
            stdout: "".into(),
            stderr: "error".into(),
        },
    );
    let module = CmdModule::new(cmd);
    let result = call(&module, "run", &[s("false")]);
    let obj = result.inner();
    assert_eq!(obj["exit_code"], 1);
    assert_eq!(obj["stderr"], "error");
}

#[test]
fn run_not_found() {
    let module = CmdModule::new(StubCmd::default());
    let func = module.function("run").unwrap();
    let mut ctx = CanonicalContext::new();
    let err = func.call(&[s("nope")], &mut ctx).unwrap_err();
    match err {
        ac_runtime::Error::PrimitiveError(msg) => assert!(msg.contains("not found")),
        other => panic!("expected PrimitiveError, got {other:?}"),
    }
}

#[test]
fn unknown_function() {
    let module = CmdModule::new(StubCmd::default());
    assert!(module.function("nope").is_none());
}

#[test]
fn missing_program_arg() {
    let module = CmdModule::new(StubCmd::default());
    let func = module.function("run").unwrap();
    let mut ctx = CanonicalContext::new();
    let err = func.call(&[], &mut ctx).unwrap_err();
    assert_eq!(
        err,
        ac_runtime::Error::ArgumentCount {
            expected: 1,
            got: 0
        }
    );
}