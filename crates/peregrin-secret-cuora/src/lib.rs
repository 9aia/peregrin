//! Cuora impl of `hypha_secret::Secrets` — the canonical secrets
//! story, mirroring `bupha-secret-cuora`/`mimus-cuora`.
//!
//! Each peregrin secret NAME resolves to a fixed cuora ref:
//! `cuora://peregrin/<NAME>` — e.g. `GITHUB_TOKEN` maps to
//! `cuora://peregrin/GITHUB_TOKEN`. Values are fetched through the
//! `cuora` CLI (`get`/`put`) so audit events, guardrails, and vault
//! policy apply exactly as for every other consumer; peregrin never
//! stores a value itself.
//!
//! `get` shells `cuora get` per lookup. Secret resolution happens a
//! handful of times per run/verb, so a per-call process is cheaper
//! than a persistent `cuora serve` session and needs no shared
//! mutable state. A missing `cuora` binary, a denied op, or an
//! absent ref all answer `None` — "not here" — so the caller's chain
//! falls through to the legacy stores (OS keyring, env file)
//! exactly like a keyring miss on a headless box.
//!
//! Env knobs: `PEREGRIN_CUORA_BIN` overrides the binary path (tests
//! stub it); `CUORA_*` and `--config` resolution stay cuora's own —
//! the child inherits the environment, so `CUORA_ENV` picks the
//! vault env with no peregrin-side plumbing.

use hypha_secret::Secrets;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// Ref namespace — every peregrin secret name lives under
/// `cuora://peregrin/…` so `cuora list` shows the whole surface.
/// (Push-backend keys live one level deeper, at
/// `cuora://peregrin/push/<key>` — see `peregrin-http`'s
/// `CuoraPushBackend`.)
pub const NAMESPACE: &str = "peregrin";

/// `Secrets` over the `cuora` CLI. Stateless — each call spawns a
/// fresh `cuora get`/`put`, inheriting cuora's own config search
/// (`$CUORA_CONFIG` → `./cuora.toml` → `~/.config/cuora/cuora.toml`).
#[derive(Clone, Debug)]
pub struct CuoraSecrets {
    /// The `cuora` binary — `PEREGRIN_CUORA_BIN` when set, else
    /// `cuora` resolved on PATH.
    pub bin: PathBuf,
}

impl Default for CuoraSecrets {
    fn default() -> Self {
        Self {
            bin: std::env::var_os("PEREGRIN_CUORA_BIN")
                .map_or_else(|| PathBuf::from("cuora"), PathBuf::from),
        }
    }
}

/// Build the ref for a secret name. A free fn so tests pin the
/// mapping without constructing the store.
///
/// `GITHUB_TOKEN` → `cuora://peregrin/GITHUB_TOKEN`. Names are
/// env-var-shaped (`[A-Z0-9_]`) which cuora's ref grammar accepts
/// verbatim — no slugging, the name IS the ref tail, so
/// `cuora put cuora://peregrin/<NAME>` is the migration path with
/// zero translation.
pub fn rref(name: &str) -> String {
    format!("cuora://{NAMESPACE}/{name}")
}

impl Secrets for CuoraSecrets {
    fn get(&self, name: &str) -> Option<String> {
        let o = Command::new(&self.bin)
            .args(["--actor", "peregrin", "get", &rref(name)])
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .ok()?;
        if !o.status.success() {
            return None;
        }
        // `cuora get` prints the value via println! — strip exactly
        // one trailing newline (and a CR before it); interior
        // whitespace is part of the value.
        let mut s = String::from_utf8_lossy(&o.stdout).into_owned();
        if let Some(stripped) = s.strip_suffix('\n') {
            s = stripped.strip_suffix('\r').unwrap_or(stripped).to_string();
        }
        (!s.is_empty()).then_some(s)
    }

    /// Deploy-time migration: `cuora put` reads the value on stdin
    /// (never argv — argv lands in shell history and `/proc`).
    fn set(&self, name: &str, value: &str) -> std::io::Result<()> {
        let mut c = Command::new(&self.bin)
            .args(["--actor", "peregrin", "put", &rref(name)])
            .stdin(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        if let Some(mut stdin) = c.stdin.take() {
            stdin.write_all(value.as_bytes())?;
        }
        let status = c.wait()?;
        if status.success() {
            Ok(())
        } else {
            Err(std::io::Error::other(format!(
                "cuora put {} exited {status}",
                rref(name)
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    /// Write an executable stub `cuora` that records argv+stdin to
    /// `$STUB_DIR/calls.log` and answers `get` from `$STUB_DIR/value`.
    fn stub(dir: &std::path::Path, script: &str) -> PathBuf {
        let bin = dir.join("cuora-stub.sh");
        std::fs::write(&bin, script).unwrap();
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
        bin
    }

    #[test]
    fn rref_maps_name_under_peregrin_namespace() {
        assert_eq!(rref("GITHUB_TOKEN"), "cuora://peregrin/GITHUB_TOKEN");
    }

    #[test]
    fn get_returns_value_and_strips_one_newline() {
        let dir = tempfile::tempdir().unwrap();
        let bin = stub(dir.path(), "#!/bin/sh\nprintf 's3cret\\n'\n");
        let s = CuoraSecrets { bin };
        assert_eq!(s.get("ANY_NAME"), Some("s3cret".to_string()));
    }

    #[test]
    fn get_answers_none_on_failure_or_empty() {
        let dir = tempfile::tempdir().unwrap();
        let failing = stub(dir.path(), "#!/bin/sh\nexit 1\n");
        assert_eq!(CuoraSecrets { bin: failing }.get("X"), None);
        let empty = stub(dir.path(), "#!/bin/sh\nprintf '\\n'\n");
        assert_eq!(CuoraSecrets { bin: empty }.get("X"), None);
    }

    #[test]
    fn set_writes_value_on_stdin() {
        let dir = tempfile::tempdir().unwrap();
        // The stub can't see our env — bake the capture path into
        // the generated script.
        let script = format!(
            "#!/bin/sh\ncat > '{}'\n",
            dir.path().join("stdin.txt").display()
        );
        let bin = stub(dir.path(), &script);
        CuoraSecrets { bin }.set("NAME", "value-1").unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.path().join("stdin.txt")).unwrap(),
            "value-1"
        );
    }

    #[test]
    fn set_errors_on_nonzero_exit() {
        let dir = tempfile::tempdir().unwrap();
        let bin = stub(dir.path(), "#!/bin/sh\nexit 3\n");
        assert!(CuoraSecrets { bin }.set("NAME", "v").is_err());
    }
}
