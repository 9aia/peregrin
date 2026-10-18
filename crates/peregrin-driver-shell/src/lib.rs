//! peregrin-driver-shell — runs node params as shell commands.

use async_trait::async_trait;
use peregrin_driver::{Driver, DriverError, DriverOutcome};

/// `driver: shell` — `params.command` runs under `/bin/sh -c`.
pub struct ShellDriver;

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
            .map_err(|e| DriverError::fatal(e.to_string()))?;
        if !status.success() {
            return Err(DriverError::fatal(format!("command exited {status}")));
        }
        Ok(DriverOutcome::default())
    }
}
