//! peregrin-keyring — OS keyring impl of `Secrets` (sturn pattern).

use std::io::Write;
use std::process::{Command, Stdio};

/// `secret-tool` (libsecret) backend — headless boxes without dbus
/// simply miss every lookup and the caller falls through to the file
/// store.
pub struct KeyringSecrets {
    /// secret-tool `service` attribute — groups all peregrin entries.
    pub service: String,
}

impl Default for KeyringSecrets {
    fn default() -> Self {
        Self {
            service: "peregrin".into(),
        }
    }
}

impl peregrin_secret::Secrets for KeyringSecrets {
    fn get(&self, name: &str) -> Option<String> {
        let o = Command::new("secret-tool")
            .args(["lookup", "service", &self.service, "name", name])
            .output()
            .ok()?;
        o.status
            .success()
            .then(|| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .filter(|v| !v.is_empty())
    }

    fn set(&self, name: &str, value: &str) -> std::io::Result<()> {
        let mut c = Command::new("secret-tool")
            .args([
                "store",
                "--label",
                &format!("{}: {name}", self.service),
                "service",
                &self.service,
                "name",
                name,
            ])
            .stdin(Stdio::piped())
            .spawn()?;
        c.stdin
            .as_mut()
            .ok_or_else(|| std::io::Error::other("secret-tool stdin"))?
            .write_all(value.as_bytes())?;
        c.wait().map(|_| ())
    }
}
