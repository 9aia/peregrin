//! Cuora impl of `hypha_secret::Secrets` — the canonical secrets
//! story, mirroring `bupha-secret-cuora`/`mimus-cuora`.
//!
//! Each peregrin secret NAME resolves to a fixed cuora ref:
//! `cuora://peregrin/<NAME>` — e.g. `GITHUB_TOKEN` maps to
//! `cuora://peregrin/GITHUB_TOKEN`. Values are fetched through cuora
//! so audit events, guardrails, and vault policy apply exactly as
//! for every other consumer; peregrin never stores a value itself.
//!
//! Two transports, selected by `PEREGRIN_CUORA_TRANSPORT`:
//!
//! - `cli` (default) — shell `cuora get`/`put` per call. Secret
//!   resolution happens a handful of times per run/verb, so a
//!   per-call process is cheap and needs no shared mutable state.
//! - `serve` — a persistent `cuora serve` child speaking the
//!   ndjson JSON-RPC wire (`cuora-serve/1`) on stdio — see
//!   [`serve`]/[`CuoraServe`]. Spawned lazily on first use, reused
//!   for the client's lifetime, respawned on death. This is the
//!   transport `cuora serve` exists for (orchestrator embedding).
//!
//! A missing `cuora` binary, a denied op, or an absent ref all
//! answer `None` — "not here" — so the caller's chain falls through
//! to the legacy stores (OS keyring, env file) exactly like a
//! keyring miss on a headless box, on either transport.
//!
//! Env knobs: `PEREGRIN_CUORA_BIN` overrides the binary path (tests
//! stub it); `PEREGRIN_CUORA_TRANSPORT` picks `cli`/`serve`;
//! `CUORA_*` and `--config` resolution stay cuora's own — the child
//! inherits the environment, so `CUORA_ENV` picks the vault env
//! with no peregrin-side plumbing (the serve child resolves env at
//! spawn; per-request `env` overrides are not used).

mod serve;

pub use serve::{CuoraServe, CuoraTransport};

use hypha_secret::Secrets;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Arc, OnceLock};

/// Ref namespace — every peregrin secret name lives under
/// `cuora://peregrin/…` so `cuora list` shows the whole surface.
/// (Push-backend keys live one level deeper, at
/// `cuora://peregrin/push/<key>` — see `peregrin-http`'s
/// `CuoraPushBackend`.)
pub const NAMESPACE: &str = "peregrin";

/// `Secrets` over cuora — CLI or `cuora serve` transport per
/// [`CuoraTransport`]. The serve leg holds one shared child per
/// `CuoraSecrets` clone group ([`Arc`]); clones share the process.
#[derive(Clone)]
pub struct CuoraSecrets {
    /// The `cuora` binary — `PEREGRIN_CUORA_BIN` when set, else
    /// `cuora` resolved on PATH.
    pub bin: PathBuf,
    /// CLI vs persistent serve transport.
    transport: CuoraTransport,
    /// The serve client — constructed eagerly but spawns lazily on
    /// first request, so the CLI transport never pays for it.
    serve: Arc<CuoraServe>,
}

impl Default for CuoraSecrets {
    fn default() -> Self {
        Self::from_env()
    }
}

impl std::fmt::Debug for CuoraSecrets {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CuoraSecrets")
            .field("bin", &self.bin)
            .field("transport", &self.transport)
            .finish_non_exhaustive()
    }
}

impl CuoraSecrets {
    /// The env-selected store — `PEREGRIN_CUORA_BIN` +
    /// `PEREGRIN_CUORA_TRANSPORT`.
    fn from_env() -> Self {
        Self::with_transport(
            std::env::var_os("PEREGRIN_CUORA_BIN")
                .map_or_else(|| PathBuf::from("cuora"), PathBuf::from),
            CuoraTransport::from_env(),
        )
    }

    /// A store over `bin` with the env-selected transport.
    #[must_use]
    pub fn with_bin(bin: PathBuf) -> Self {
        Self::with_transport(bin, CuoraTransport::from_env())
    }

    /// A store over `bin` on an explicit transport — the test seam
    /// and the "I know what I'm doing" constructor.
    #[must_use]
    pub fn with_transport(bin: PathBuf, transport: CuoraTransport) -> Self {
        Self {
            serve: Arc::new(CuoraServe::new(bin.clone())),
            bin,
            transport,
        }
    }

    /// The process-wide default store.
    ///
    /// Worth sharing under the serve transport: `CuoraSecrets` is
    /// cheap to construct but the serve child's whole point is
    /// persistence — a fresh store per lookup would spawn and reap
    /// `cuora serve` on every secret read, worse than the CLI. Under
    /// `cli` sharing is a no-op behaviorally.
    #[must_use]
    pub fn shared_default() -> &'static Self {
        static DEFAULT: OnceLock<CuoraSecrets> = OnceLock::new();
        DEFAULT.get_or_init(CuoraSecrets::from_env)
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
        match self.transport {
            CuoraTransport::Cli => self.get_cli(name),
            CuoraTransport::Serve => self.serve.get(&rref(name)),
        }
    }

    /// Deploy-time migration: `cuora put` reads the value on stdin
    /// (never argv — argv lands in shell history and `/proc`).
    /// Under serve the value rides the request's `value` field —
    /// the wire's designed channel, still never argv.
    fn set(&self, name: &str, value: &str) -> std::io::Result<()> {
        match self.transport {
            CuoraTransport::Cli => self.set_cli(name, value),
            CuoraTransport::Serve => self.serve.put(&rref(name), value),
        }
    }
}

impl CuoraSecrets {
    /// The `cli` transport's `get` — shell `cuora get` per lookup.
    fn get_cli(&self, name: &str) -> Option<String> {
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

    /// The `cli` transport's `set` — `cuora put` with the value on
    /// stdin (never argv — argv lands in shell history and `/proc`).
    fn set_cli(&self, name: &str, value: &str) -> std::io::Result<()> {
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
        let s = CuoraSecrets::with_bin(bin);
        assert_eq!(s.get("ANY_NAME"), Some("s3cret".to_string()));
    }

    #[test]
    fn get_answers_none_on_failure_or_empty() {
        let dir = tempfile::tempdir().unwrap();
        let failing = stub(dir.path(), "#!/bin/sh\nexit 1\n");
        assert_eq!(CuoraSecrets::with_bin(failing).get("X"), None);
        let empty = stub(dir.path(), "#!/bin/sh\nprintf '\\n'\n");
        assert_eq!(CuoraSecrets::with_bin(empty).get("X"), None);
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
        CuoraSecrets::with_bin(bin).set("NAME", "value-1").unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.path().join("stdin.txt")).unwrap(),
            "value-1"
        );
    }

    #[test]
    fn set_errors_on_nonzero_exit() {
        let dir = tempfile::tempdir().unwrap();
        let bin = stub(dir.path(), "#!/bin/sh\nexit 3\n");
        assert!(CuoraSecrets::with_bin(bin).set("NAME", "v").is_err());
    }

    #[test]
    fn serve_transport_roundtrips_through_the_wire() {
        // A `cuora serve` stub in the ndjson wire shape — the same
        // fake `serve.rs` tests use, inlined here so this test pins
        // the *Secrets* dispatch, not the codec.
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("db");
        let script = format!(
            "#!/bin/sh\n\
             while IFS= read -r line; do\n\
               op=$(printf '%s' \"$line\" | grep -o '\"op\":\"[a-z]*\"' | cut -d'\"' -f4)\n\
               ref=$(printf '%s' \"$line\" | grep -o '\"ref\":\"[^\"]*\"' | cut -d'\"' -f4)\n\
               val=$(printf '%s' \"$line\" | grep -o '\"value\":\"[^\"]*\"' | cut -d'\"' -f4)\n\
               key=${{ref##*/}}\n\
               if [ \"$op\" = put ]; then printf '%s' \"$val\" > '{db}-'\"$key\"; \\\n                 printf '{{\"id\":1,\"ok\":true,\"result\":{{}}}}\\n';\n\
               elif [ \"$op\" = get ] && [ -f '{db}-'\"$key\" ]; then \\\n                 printf '{{\"id\":1,\"ok\":true,\"result\":{{\"value\":\"%s\"}}}}\\n' \"$(cat '{db}-'\"$key\")\";\n\
               else printf '{{\"id\":1,\"ok\":false,\"error\":{{\"code\":\"not_found\",\"message\":\"x\"}}}}\\n'; fi\n\
             done\n",
            db = db.display(),
        );
        let bin = stub(dir.path(), &script);
        let s = CuoraSecrets::with_transport(bin, CuoraTransport::Serve);
        assert_eq!(s.get("MISSING"), None);
        s.set("TOKEN", "sekret").unwrap();
        assert_eq!(s.get("TOKEN").as_deref(), Some("sekret"));
    }
}
