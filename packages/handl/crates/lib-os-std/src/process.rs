use std::collections::HashMap;
use std::process::Command as StdProcessCommand;
use std::sync::Mutex;

use handl_lib_os::{OsError, Process, ProcessStatus};

/// Process management backed by [`std::process`].
#[derive(Default)]
pub struct StdProcess {
    processes: Mutex<HashMap<i64, std::process::Child>>,
}

impl Process for StdProcess {
    fn spawn(&self, program: &str, args: &[&str]) -> Result<i64, OsError> {
        let child = StdProcessCommand::new(program)
            .args(args)
            .spawn()
            .map_err(|e| match e.kind() {
                std::io::ErrorKind::NotFound => OsError::NotFound {
                    path: program.into(),
                },
                _ => OsError::Io {
                    message: e.to_string(),
                },
            })?;
        let process_id = child.id() as i64;
        self.processes.lock().unwrap().insert(process_id, child);
        Ok(process_id)
    }

    fn kill(&self, process_id: i64) -> Result<(), OsError> {
        let mut processes = self.processes.lock().unwrap();
        let child = processes.get_mut(&process_id).ok_or(OsError::NotFound {
            path: format!("process: {process_id}"),
        })?;
        child.kill().map_err(|e| OsError::Io {
            message: e.to_string(),
        })
    }

    fn wait(&self, process_id: i64) -> Result<i32, OsError> {
        let mut processes = self.processes.lock().unwrap();
        let child = processes.get_mut(&process_id).ok_or(OsError::NotFound {
            path: format!("process: {process_id}"),
        })?;
        let status = child.wait().map_err(|e| OsError::Io {
            message: e.to_string(),
        })?;
        Ok(exit_code(status))
    }

    fn status(&self, process_id: i64) -> Result<ProcessStatus, OsError> {
        let mut processes = self.processes.lock().unwrap();
        let child = processes.get_mut(&process_id).ok_or(OsError::NotFound {
            path: format!("process: {process_id}"),
        })?;
        match child.try_wait().map_err(|e| OsError::Io {
            message: e.to_string(),
        })? {
            Some(status) => Ok(ProcessStatus {
                process_id,
                running: false,
                exit_code: Some(exit_code(status)),
            }),
            None => Ok(ProcessStatus {
                process_id,
                running: true,
                exit_code: None,
            }),
        }
    }
}

#[cfg(unix)]
fn exit_code(status: std::process::ExitStatus) -> i32 {
    use std::os::unix::process::ExitStatusExt;
    status
        .code()
        .unwrap_or_else(|| 128 + status.signal().unwrap_or(0))
}

#[cfg(not(unix))]
fn exit_code(status: std::process::ExitStatus) -> i32 {
    status.code().unwrap_or(-1)
}
