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

/// A usable secret name — non-empty and not `-`-leading (a leading
/// dash would parse as a `secret-tool` flag, not an attribute value).
fn valid_name(name: &str) -> bool {
    !name.is_empty() && !name.starts_with('-')
}

impl peregrin_secret::Secrets for KeyringSecrets {
    fn get(&self, name: &str) -> Option<String> {
        if !valid_name(name) {
            return None;
        }
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
        if !valid_name(name) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "secret name must be non-empty and not start with '-'",
            ));
        }
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
