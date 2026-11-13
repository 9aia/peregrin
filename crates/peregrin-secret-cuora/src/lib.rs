//! Cuora impl of `hypha_secret::Secrets` — peregrin's env-facing
//! shim over the shared [`hypha_secret_cuora`] adapter.
//!
//! Each peregrin secret NAME resolves to a fixed cuora ref:
//! `cuora://peregrin/<NAME>` — e.g. `GITHUB_TOKEN` maps to
//! `cuora://peregrin/GITHUB_TOKEN`. Values are fetched through cuora
//! so audit events, guardrails, and vault policy apply exactly as
//! for every other consumer; peregrin never stores a value itself.
//!
//! ## Scope fallback (T-053)
//!
//! The bupha fleet surface is the *canonical* home for the shared
//! secret names — `cuora://bupha/<NAME>` holds the single copy of
//! every `BUPHA_*`/`PEREGRIN_*` fleet secret; `cuora://peregrin/`
//! copies survive only where the two scopes legitimately diverge
//! (T-053). Resolution is therefore
//! a two-scope chain, own scope first:
//!
//! 1. `cuora://peregrin/<NAME>` — peregrine-owned names
//!    (`GITHUB_TOKEN`, `push/*`) and any deliberate per-product
//!    override;
//! 2. `cuora://bupha/<NAME>` — the canonical fleet copy;
//! 3. the caller's next leg (OS keyring, env file).
//!
//! Own-scope-first ordering means a name present under both scopes
//! keeps resolving to the peregrin value — the mechanism preserves
//! today's behavior for names whose scopes legitimately diverge.
//! `set` always writes the *primary* scope: migrating a shared
//! fleet name is `cuora put cuora://bupha/<NAME>`, never this
//! store. When cuora's `[ref-aliases]` (T-052) lands, the alias
//! table can replace this client-side chain — the fallback list
//! here is deliberately a per-consumer concern, not vault config.
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

/// Fallback ref namespace — the canonical bupha fleet surface
/// (T-053). Consulted only when `cuora://peregrin/<NAME>` misses;
/// the shared fleet names' single copy lives there.
pub const FALLBACK_NAMESPACE: &str = "bupha";

/// The env knobs this shim reads — re-exported for consumers that
/// build their own transport legs (`peregrin-http`'s push backend).
pub const BIN_ENV: &str = "PEREGRIN_CUORA_BIN";
pub const TRANSPORT_ENV: &str = "PEREGRIN_CUORA_TRANSPORT";
/// Parallel workers for [`CuoraSecrets::get_many`] —
/// `PEREGRIN_CUORA_WORKERS` overrides the default.
pub const WORKERS_ENV: &str = "PEREGRIN_CUORA_WORKERS";

/// Default `get_many` fan-out — the serve wire is strictly
/// one-request-at-a-time per `cuora serve` child, so batch
/// throughput scales with child processes (the CLI transport
/// spawns a process per `get` regardless, so the same count
/// parallelizes it too).
const DEFAULT_WORKERS: usize = 8;
/// Ceiling on `PEREGRIN_CUORA_WORKERS` — each worker is a `cuora
/// serve` child (or a concurrent `cuora get` process under cli).
const MAX_WORKERS: usize = 32;

/// `Secrets` over cuora — CLI or `cuora serve` transport per
/// [`CuoraTransport`]. Thin wrapper over
/// [`hypha_secret_cuora::CuoraSecrets`] pinned to the `peregrin`
/// scope plus the `bupha` fallback scope (T-053 — see the module
/// docs); the env knobs stay peregrin's.
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
    /// Canonical fleet scope — consulted only on a primary miss
    /// (see the module docs' scope-fallback chain).
    fallback: hypha_secret_cuora::CuoraSecrets,
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
            .field("fallback_scope", &FALLBACK_NAMESPACE)
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
                .with_bin(bin.clone())
                .with_transport(transport),
            fallback: hypha_secret_cuora::CuoraSecrets::new(FALLBACK_NAMESPACE)
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

    /// Batch `get` — resolves every name, returning one `Option`
    /// per name in order (`None` = miss, identical to [`Secrets::get`]
    /// per name).
    ///
    /// Sequential `get`s serialize on the wire: under `serve` the
    /// shared child answers one request at a time and each request
    /// costs a vault round-trip, so a listener startup resolving
    /// ~140 `secret_env` names took seconds. `get_many` fans the
    /// batch out over [`Self::workers`] parallel lanes — under
    /// `serve` each lane is its own `cuora serve` child (spawned
    /// lazily, reaped when the batch returns; audit/guardrail/PDP
    /// semantics per request are unchanged since every request is
    /// an ordinary `get`), under `cli` each lane is just a thread
    /// since every `get` is already its own process.
    ///
    /// Secret *values* are never logged; results stay in the
    /// caller's ctx exactly like `get`.
    #[must_use]
    pub fn get_many(&self, names: &[&str]) -> Vec<Option<String>> {
        let workers = Self::workers().min(names.len());
        if workers <= 1 {
            return names.iter().map(|name| self.get(name)).collect();
        }
        let transport = self.inner.transport();
        let bin = self.bin.clone();
        let mut results: Vec<Option<String>> = vec![None; names.len()];
        std::thread::scope(|scope| {
            let handles: Vec<_> = (0..workers)
                .map(|lane| {
                    let bin = bin.clone();
                    scope.spawn(move || {
                        // Round-robin keeps lanes balanced when a
                        // few names are slow (remote vault hits).
                        let mut lane_results = Vec::new();
                        let mut i = lane;
                        // One child per lane under serve — the
                        // shared `inner` child would serialize the
                        // whole batch on its request mutex. The
                        // fallback scope gets its own child per
                        // lane too: refs are per-scope and the
                        // actor label must match the scope.
                        let serve = if transport == CuoraTransport::Serve {
                            Some((
                                CuoraServe::new(bin.clone(), NAMESPACE),
                                CuoraServe::new(bin.clone(), FALLBACK_NAMESPACE),
                            ))
                        } else {
                            None
                        };
                        while i < names.len() {
                            let value = match &serve {
                                Some((primary, fallback)) => {
                                    primary.get(&rref(names[i])).or_else(|| {
                                        fallback.get(&hypha_secret_cuora::rref(
                                            FALLBACK_NAMESPACE,
                                            names[i],
                                        ))
                                    })
                                }
                                None => self.get(names[i]),
                            };
                            lane_results.push((i, value));
                            i += workers;
                        }
                        lane_results
                    })
                })
                .collect();
            for handle in handles {
                if let Ok(pairs) = handle.join() {
                    for (i, value) in pairs {
                        results[i] = value;
                    }
                }
            }
        });
        results
    }

    /// `PEREGRIN_CUORA_WORKERS`, clamped — see [`WORKERS_ENV`].
    fn workers() -> usize {
        std::env::var(WORKERS_ENV)
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .map_or(DEFAULT_WORKERS, |n| n.clamp(1, MAX_WORKERS))
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
    /// Own scope first, then the canonical bupha fleet scope — a
    /// name present under both keeps resolving to the peregrin
    /// value, so legitimately-divergent scopes are preserved.
    fn get(&self, name: &str) -> Option<String> {
        self.inner.get(name).or_else(|| self.fallback.get(name))
    }

    /// Deploy-time migration — `cuora put` reads the value on stdin
    /// (never argv) on the CLI leg; under serve it rides the
    /// request's `value` field, still never argv. Always writes the
    /// *primary* scope — a shared fleet name's canonical write is
    /// `cuora put cuora://bupha/<NAME>`.
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
    fn get_falls_back_to_bupha_scope_on_peregrin_miss() {
        // Scope-aware stub: `cuora://peregrin/*` misses, `cuora://bupha/*`
        // answers — the T-053 dedupe shape.
        let dir = tempfile::tempdir().unwrap();
        let bin = stub(
            dir.path(),
            "#!/bin/sh\n\
             for ref in \"$@\"; do :; done\n\
             case \"$ref\" in\n\
               cuora://bupha/*) printf 'fleet-value\\n' ;;\n\
               *) exit 1 ;;\n\
             esac\n",
        );
        assert_eq!(
            CuoraSecrets::with_bin(bin).get("BUPHA_WH_9AIA_BUPHA"),
            Some("fleet-value".to_string())
        );
    }

    #[test]
    fn get_prefers_own_scope_when_both_scopes_have_the_name() {
        // Divergent scopes (the prod BUPHA_WH_HANDL_LANG_* shape):
        // the peregrin copy must win — fallback is miss-only.
        let dir = tempfile::tempdir().unwrap();
        let bin = stub(
            dir.path(),
            "#!/bin/sh\n\
             for ref in \"$@\"; do :; done\n\
             case \"$ref\" in\n\
               cuora://peregrin/*) printf 'peregrin-value\\n' ;;\n\
               cuora://bupha/*) printf 'bupha-value\\n' ;;\n\
               *) exit 1 ;;\n\
             esac\n",
        );
        assert_eq!(
            CuoraSecrets::with_bin(bin).get("BUPHA_WH_X"),
            Some("peregrin-value".to_string())
        );
    }

    #[test]
    fn set_writes_the_primary_scope_only() {
        let dir = tempfile::tempdir().unwrap();
        let script = format!(
            "#!/bin/sh\nprintf '%s\\n' \"$@\" > '{}'\ncat >/dev/null\n",
            dir.path().join("argv.txt").display()
        );
        let bin = stub(dir.path(), &script);
        CuoraSecrets::with_bin(bin).set("NAME", "v").unwrap();
        let argv = std::fs::read_to_string(dir.path().join("argv.txt")).unwrap();
        assert!(argv.contains("cuora://peregrin/NAME"), "argv was: {argv}");
        assert!(!argv.contains("cuora://bupha/"), "argv was: {argv}");
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

    #[test]
    fn get_many_preserves_order_and_none_semantics() {
        let dir = tempfile::tempdir().unwrap();
        // cli stub answers every get with the same value — the
        // batch must still return one entry per name, in order.
        let bin = stub(dir.path(), "#!/bin/sh\nprintf 's3cret\\n'\n");
        let s = CuoraSecrets::with_bin(bin);
        let names = ["N1", "N2", "N3"];
        let name_refs: Vec<&str> = names.to_vec();
        assert_eq!(
            s.get_many(&name_refs),
            vec![
                Some("s3cret".to_string()),
                Some("s3cret".to_string()),
                Some("s3cret".to_string())
            ]
        );
        assert_eq!(s.get_many(&[]).len(), 0);
    }

    #[test]
    fn get_many_under_serve_fans_out_over_parallel_children() {
        let dir = tempfile::tempdir().unwrap();
        // A `cuora serve` stub in the ndjson wire shape that counts
        // spawns — each get_many lane owns its own child, so >1
        // spawn proves the fan-out (the shared `inner` child would
        // serialize the whole batch on one process).
        let db = dir.path().join("db");
        let script = format!(
            "#!/bin/sh\n\
             echo x >> '{dir}/spawns'\n\
             while IFS= read -r line; do\n\
               op=$(printf '%s' \"$line\" | grep -o '\"op\":\"[a-z]*\"' | cut -d'\"' -f4)\n\
               ref=$(printf '%s' \"$line\" | grep -o '\"ref\":\"[^\"]*\"' | cut -d'\"' -f4)\n\
               val=$(printf '%s' \"$line\" | grep -o '\"value\":\"[^\"]*\"' | cut -d'\"' -f4)\n\
               key=${{ref##*/}}\n\
               if [ \"$op\" = put ]; then printf '%s' \"$val\" > '{db}-'\"$key\"; \\\n                 printf '{{\"id\":1,\"ok\":true,\"result\":{{}}}}\\n';\n\
               elif [ \"$op\" = get ] && [ -f '{db}-'\"$key\" ]; then \\\n                 printf '{{\"id\":1,\"ok\":true,\"result\":{{\"value\":\"%s\"}}}}\\n' \"$(cat '{db}-'\"$key\")\";\n\
               else printf '{{\"id\":1,\"ok\":false,\"error\":{{\"code\":\"not_found\",\"message\":\"x\"}}}}\\n'; fi\n\
             done\n",
            dir = dir.path().display(),
            db = db.display(),
        );
        let bin = stub(dir.path(), &script);
        let s = CuoraSecrets::with_transport(bin, CuoraTransport::Serve);
        // Seed through the single `inner` child (one spawn), then
        // batch-read — misses and hits interleaved.
        for i in 0..6 {
            s.set(&format!("K{i}"), &format!("v{i}")).unwrap();
        }
        let names: Vec<String> = (0..6).map(|i| format!("K{i}")).collect();
        let mut name_refs: Vec<&str> = names.iter().map(String::as_str).collect();
        name_refs.push("MISSING_NAME");
        let out = s.get_many(&name_refs);
        let expected: Vec<Option<String>> = (0..6)
            .map(|i| Some(format!("v{i}")))
            .chain(std::iter::once(None))
            .collect();
        assert_eq!(out, expected);
        let spawns = std::fs::read_to_string(dir.path().join("spawns"))
            .unwrap()
            .lines()
            .count();
        assert!(
            spawns > 1,
            "get_many must fan out over parallel serve children, got {spawns} spawn(s)"
        );
    }
}
