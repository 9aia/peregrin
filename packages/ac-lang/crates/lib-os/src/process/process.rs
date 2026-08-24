use ac_runtime_values::NonFunctionValue;
use std::sync::Arc;

use ac_runtime::{Context, Function};
use ac_runtime_modules::Module;
use ac_runtime_values::Error;

use crate::error::OsError;

/// Snapshot of a managed child process.
#[derive(Debug, Clone)]
pub struct ProcessStatus {
    pub process_id: i64,
    pub running: bool,
    pub exit_code: Option<i32>,
}

/// Abstract child-process management service.
///
/// Provides lifecycle control over OS processes: spawn, kill, wait, and poll.
///
/// Implementations:
/// - `ac-lib-os-std` — backed by `std::process`
/// - [`process_stub::StubProcess`] — canned process lifecycle for testing
pub trait Process: Send + Sync {
    fn spawn(&self, program: &str, args: &[&str]) -> Result<i64, OsError>;
    fn kill(&self, process_id: i64) -> Result<(), OsError>;
    fn wait(&self, process_id: i64) -> Result<i32, OsError>;
    fn status(&self, process_id: i64) -> Result<ProcessStatus, OsError>;
}

/// Module adapter for [`Process`] — exposes `process.*` functions.
pub struct ProcessModule {
    spawn: SpawnFn,
    kill: KillFn,
    wait: WaitFn,
    status: StatusFn,
}

impl ProcessModule {
    pub fn new(process: impl Process + 'static) -> Self {
        let process = Arc::new(process);
        ProcessModule {
            spawn: SpawnFn {
                process: process.clone(),
            },
            kill: KillFn {
                process: process.clone(),
            },
            wait: WaitFn {
                process: process.clone(),
            },
            status: StatusFn { process },
        }
    }
}

impl Module for ProcessModule {
    fn name(&self) -> &str {
        "process"
    }

    fn function(&self, name: &str) -> Option<&dyn Function<NonFunctionValue>> {
        match name {
            "spawn" => Some(&self.spawn),
            "kill" => Some(&self.kill),
            "wait" => Some(&self.wait),
            "status" => Some(&self.status),
            _ => None,
        }
    }
}

fn status_to_value(status: ProcessStatus) -> NonFunctionValue {
    NonFunctionValue::new(serde_json::json!({
        "process_id": status.process_id,
        "running": status.running,
        "exit_code": status.exit_code,
    }))
}

struct SpawnFn {
    process: Arc<dyn Process>,
}

impl Function<NonFunctionValue> for SpawnFn {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
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

        self.process
            .spawn(program, &arg_refs)
            .map(|id| NonFunctionValue::new(serde_json::json!(id)))
            .map_err(Into::into)
    }
}

struct KillFn {
    process: Arc<dyn Process>,
}

impl Function<NonFunctionValue> for KillFn {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        if args.is_empty() {
            return Err(Error::ArgumentCount {
                expected: 1,
                got: 0,
            });
        }
        let process_id = args[0].as_number().ok_or(Error::TypeMismatch {
            expected: "number",
            got: "non-number",
        })? as i64;
        self.process.kill(process_id)?;
        Ok(NonFunctionValue::default())
    }
}

struct WaitFn {
    process: Arc<dyn Process>,
}

impl Function<NonFunctionValue> for WaitFn {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        if args.is_empty() {
            return Err(Error::ArgumentCount {
                expected: 1,
                got: 0,
            });
        }
        let process_id = args[0].as_number().ok_or(Error::TypeMismatch {
            expected: "number",
            got: "non-number",
        })? as i64;
        self.process
            .wait(process_id)
            .map(|code| NonFunctionValue::new(serde_json::json!(code)))
            .map_err(Into::into)
    }
}

struct StatusFn {
    process: Arc<dyn Process>,
}

impl Function<NonFunctionValue> for StatusFn {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        if args.is_empty() {
            return Err(Error::ArgumentCount {
                expected: 1,
                got: 0,
            });
        }
        let process_id = args[0].as_number().ok_or(Error::TypeMismatch {
            expected: "number",
            got: "non-number",
        })? as i64;
        self.process
            .status(process_id)
            .map(status_to_value)
            .map_err(Into::into)
    }
}
