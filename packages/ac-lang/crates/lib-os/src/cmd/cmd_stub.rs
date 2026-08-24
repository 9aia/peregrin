//! Canned-output [`Cmd`] stub for testing.

use std::collections::HashMap;
use std::sync::Mutex;

use crate::cmd::{Cmd, CmdOutput};
use crate::error::OsError;

/// A command executor that returns canned outputs by program name.
#[derive(Default)]
pub struct StubCmd {
    outputs: Mutex<HashMap<String, CmdOutput>>,
}

impl StubCmd {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with(program: impl Into<String>, output: CmdOutput) -> Self {
        let mut outputs = HashMap::new();
        outputs.insert(program.into(), output);
        StubCmd {
            outputs: Mutex::new(outputs),
        }
    }
}

impl Cmd for StubCmd {
    fn run(&self, program: &str, _args: &[&str]) -> Result<CmdOutput, OsError> {
        self.outputs
            .lock()
            .unwrap()
            .get(program)
            .cloned()
            .ok_or(OsError::NotFound {
                path: format!("command: {program}"),
            })
    }
}