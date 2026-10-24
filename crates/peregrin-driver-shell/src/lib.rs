//! peregrin-driver-shell — runs node params as shell commands.

use async_trait::async_trait;
use peregrin_driver::{Driver, DriverError, DriverOutcome};

/// `driver: shell` — `params.command` runs under `/bin/sh -c`.
pub struct ShellDriver;

/// Named so the io-error arm is one region tests can invoke directly —
/// a spawned `/bin/sh` realistically never fails to launch.
#[allow(clippy::needless_pass_by_value)] // `map_err` hands ownership
fn fatal_io(e: std::io::Error) -> DriverError {
    DriverError::fatal(e.to_string())
}

#[async_trait]
impl Driver for ShellDriver {
    async fn run(
        &self,
        node: &peregrin_core::Node,
        _event: &peregrin_core::Event,
    ) -> Result<DriverOutcome, DriverError> {
        let command = node
            .params
            .get("command")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| DriverError::fatal("shell driver needs params.command"))?;
        let status = tokio::process::Command::new("/bin/sh")
            .arg("-c")
            .arg(command)
            .status()
            .await
            .map_err(fatal_io)?;
        if !status.success() {
            return Err(DriverError::fatal(format!("command exited {status}")));
        }
        Ok(DriverOutcome::default())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn fatal_io_arm() {
        let e = fatal_io(std::io::Error::new(std::io::ErrorKind::NotFound, "gone"));
        assert!(e.to_string().contains("gone"));
    }
}
