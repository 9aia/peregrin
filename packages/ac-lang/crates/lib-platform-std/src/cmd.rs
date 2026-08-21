use std::process::Command as StdProcessCommand;

use ac_lib_platform::{Cmd, CmdOutput, PlatformError};

/// Command execution backed by [`std::process::Command`].
#[derive(Default)]
pub struct StdCmd;

impl Cmd for StdCmd {
    fn run(&self, program: &str, args: &[&str]) -> Result<CmdOutput, PlatformError> {
        let output = StdProcessCommand::new(program)
            .args(args)
            .output()
            .map_err(|e| PlatformError::Io {
                message: e.to_string(),
            })?;

        Ok(CmdOutput {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}
