//! Canned-process [`Process`] stub for testing.

use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Mutex;

use crate::error::OsError;
use crate::process::{Process, ProcessStatus};

#[derive(Debug, Clone)]
struct Child {
    exit_code: i32,
    running: bool,
    killed: bool,
}

/// A process service with a controllable in-memory process table.
pub struct StubProcess {
    next_id: AtomicI64,
    processes: Mutex<HashMap<i64, Child>>,
}

impl StubProcess {
    pub fn new() -> Self {
        Self {
            next_id: AtomicI64::new(1000),
            processes: Mutex::new(HashMap::new()),
        }
    }

    pub fn with(
        process_id: i64,
        _program: impl Into<String>,
        _args: Vec<String>,
        exit_code: i32,
        running: bool,
    ) -> Self {
        let mut processes = HashMap::new();
        processes.insert(
            process_id,
            Child {
                exit_code,
                running,
                killed: !running,
            },
        );
        Self {
            next_id: AtomicI64::new(process_id + 1),
            processes: Mutex::new(processes),
        }
    }
}

impl Default for StubProcess {
    fn default() -> Self {
        Self::new()
    }
}

impl Process for StubProcess {
    fn spawn(&self, _program: &str, _args: &[&str]) -> Result<i64, OsError> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let child = Child {
            exit_code: 0,
            running: true,
            killed: false,
        };
        self.processes.lock().unwrap().insert(id, child);
        Ok(id)
    }

    fn kill(&self, process_id: i64) -> Result<(), OsError> {
        let mut processes = self.processes.lock().unwrap();
        let child = processes.get_mut(&process_id).ok_or(OsError::NotFound {
            path: format!("process: {process_id}"),
        })?;
        if !child.running {
            return Err(OsError::InvalidArgument {
                message: format!("process {process_id} is not running"),
            });
        }
        child.running = false;
        child.killed = true;
        child.exit_code = 137;
        Ok(())
    }

    fn wait(&self, process_id: i64) -> Result<i32, OsError> {
        let mut processes = self.processes.lock().unwrap();
        let child = processes.get_mut(&process_id).ok_or(OsError::NotFound {
            path: format!("process: {process_id}"),
        })?;
        if child.running {
            child.running = false;
        }
        Ok(child.exit_code)
    }

    fn status(&self, process_id: i64) -> Result<ProcessStatus, OsError> {
        let processes = self.processes.lock().unwrap();
        let child = processes.get(&process_id).ok_or(OsError::NotFound {
            path: format!("process: {process_id}"),
        })?;
        Ok(ProcessStatus {
            process_id,
            running: child.running,
            exit_code: if child.running { None } else { Some(child.exit_code) },
        })
    }
}
