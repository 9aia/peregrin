use std::sync::Arc;

use ac_runtime::NonFunctionValue;
use ac_runtime_core::Module;
use ac_vm::{Context, Error, Function};

use crate::error::PlatformError;

/// Output from a command execution.
#[derive(Debug, Clone)]
pub struct CmdOutput {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

/// Abstract command execution service.
///
/// Provides a way to interact with the command line — spawn processes,
/// capture stdout/stderr, and check exit codes.
///
/// Implementations:
/// - `ac-lib-platform-std` — backed by `std::process::Command`
/// - [`cmd_stub::StubCmd`] — canned outputs for testing
pub trait Cmd: Send + Sync {
    fn run(&self, program: &str, args: &[&str]) -> Result<CmdOutput, PlatformError>;
}

/// Module adapter for [`Cmd`] — exposes `cmd.*` functions.
pub struct CmdModule {
    run: RunFn,
}

impl CmdModule {
    pub fn new(command: impl Cmd + 'static) -> Self {
        CmdModule {
            run: RunFn {
                command: Arc::new(command),
            },
        }
    }
}

impl Module for CmdModule {
    fn name(&self) -> &str {
        "cmd"
    }

    fn function(&self, name: &str) -> Option<&dyn Function> {
        match name {
            "run" => Some(&self.run),
            _ => None,
        }
    }
}

fn output_to_value(output: CmdOutput) -> NonFunctionValue {
    NonFunctionValue::new(serde_json::json!({
        "exit_code": output.exit_code,
        "stdout": output.stdout,
        "stderr": output.stderr,
    }))
}

struct RunFn {
    command: Arc<dyn Cmd>,
}

impl Function for RunFn {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let program = args.first().ok_or(Error::ArgumentCount {
            expected: 1,
            got: 0,
        })?;
        let program = program.as_str().ok_or(Error::TypeMismatch {
            expected: "string",
            got: "non-string",
        })?;

        let arg_strings: Vec<String> = args
            .iter()
            .skip(1)
            .map(|v| {
                v.as_str().map(String::from).ok_or(Error::TypeMismatch {
                    expected: "string",
                    got: "non-string",
                })
            })
            .collect::<Result<_, _>>()?;

        let arg_refs: Vec<&str> = arg_strings.iter().map(String::as_str).collect();
        self.command
            .run(program, &arg_refs)
            .map(output_to_value)
            .map_err(Into::into)
    }
}
