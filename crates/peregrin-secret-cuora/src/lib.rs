//! Cuora impl of `hypha_secret::Secrets` — peregrin's env-facing
//! shim over the shared [`hypha_secret_cuora`] adapter.
//!
//! Each peregrin secret NAME resolves to a fixed cuora ref:
//! `cuora://peregrin/<NAME>` — e.g. `GITHUB_TOKEN` maps to
//! `cuora://peregrin/GITHUB_TOKEN`. Values are fetched through cuora
//! so audit events, guardrails, and vault policy apply exactly as
//! for every other consumer; peregrin never stores a value itself.
//!
//! Two transports, selected by `PEREGRIN_CUORA_TRANSPORT` (the
//! `<PRODUCT>_CUORA_TRANSPORT` convention — see `hypha-secret-cuora`):
//!
//! - `cli` (default) — shell `cuora get`/`put` per call. Secret
//!   resolution happens a handful of times per run/verb, so a
//!   per-call process is cheap and needs no shared mutable state.
//! - `serve` — a persistent `cuora serve` child speaking the
//!   ndjson JSON-RPC wire (`cuora-serve/1`) on stdio — see
//!   [`CuoraServe`]. Spawned lazily on first use, reused for the
//!   client's lifetime, respawned on death.
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

pub use hypha_secret_cuora::{CuoraServe, CuoraTransport};

use hypha_secret::Secrets;
use std::path::PathBuf;
use std::sync::OnceLock;

/// Ref namespace — every peregrin secret name lives under
/// `cuora://peregrin/…` so `cuora list` shows the whole surface.
/// (Push-backend keys live one level deeper, at
/// `cuora://peregrin/push/<key>` — see `peregrin-http`'s
/// `CuoraPushBackend`.)
pub const NAMESPACE: &str = "peregrin";

/// The env knobs this shim reads — re-exported for consumers that
/// build their own transport legs (`peregrin-http`'s push backend).
pub const BIN_ENV: &str = "PEREGRIN_CUORA_BIN";
pub const TRANSPORT_ENV: &str = "PEREGRIN_CUORA_TRANSPORT";

/// `Secrets` over cuora — CLI or `cuora serve` transport per
/// [`CuoraTransport`]. Thin wrapper over
/// [`hypha_secret_cuora::CuoraSecrets`] pinned to the `peregrin`
/// scope; the env knobs stay peregrin's.
///
/// Clones share the serve child — see
/// [`CuoraSecrets::shared_default`] for why that matters.
#[derive(Clone)]
pub struct CuoraSecrets {
    /// The `cuora` binary — `PEREGRIN_CUORA_BIN` when set, else
    /// `cuora` resolved on PATH. Kept as a field (not just inside
    /// `inner`) because consumers document it as the test seam.
    pub bin: PathBuf,
    inner: hypha_secret_cuora::CuoraSecrets,
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
            .field("transport", &self.inner.transport())
            .finish_non_exhaustive()
    }
}

impl CuoraSecrets {
    /// The env-selected store — `PEREGRIN_CUORA_BIN` +
    /// `PEREGRIN_CUORA_TRANSPORT`.
    fn from_env() -> Self {
        Self::with_transport(
            std::env::var_os(BIN_ENV).map_or_else(|| PathBuf::from("cuora"), PathBuf::from),
            CuoraTransport::from_env(TRANSPORT_ENV),
        )
    }

    /// A store over `bin` with the env-selected transport.
    #[must_use]
    pub fn with_bin(bin: PathBuf) -> Self {
        Self::with_transport(bin, CuoraTransport::from_env(TRANSPORT_ENV))
    }

    /// A store over `bin` on an explicit transport — the test seam
    /// and the "I know what I'm doing" constructor.
    #[must_use]
    pub fn with_transport(bin: PathBuf, transport: CuoraTransport) -> Self {
        Self {
            bin: bin.clone(),
            inner: hypha_secret_cuora::CuoraSecrets::new(NAMESPACE)
                .with_bin(bin)
                .with_transport(transport),
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
/// `GITHUB_TOKEN` → `cuora://peregrin/GITHUB_TOKEN`.
#[must_use]
pub fn rref(name: &str) -> String {
    hypha_secret_cuora::rref(NAMESPACE, name)
}

impl Secrets for CuoraSecrets {
    fn get(&self, name: &str) -> Option<String> {
        self.inner.get(name)
    }

    /// Deploy-time migration — `cuora put` reads the value on stdin
    /// (never argv) on the CLI leg; under serve it rides the
    /// request's `value` field, still never argv.
    fn set(&self, name: &str, value: &str) -> std::io::Result<()> {
        self.inner.set(name, value)
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
        // fake `serve.rs` tests use in hypha-secret-cuora, inlined
        // here so this test pins the *wrapper* dispatch.
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
