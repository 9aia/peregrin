//! Web Push wiring for the session plane — the `PushBackend`
//! implementation that keeps VAPID keys + subscriptions inside the
//! KV's internal `push` namespace (P-006 phase 2; they used to live in
//! the meta overlay's `config` document) plus the session-event →
//! notification mapping. The push *machinery* (VAPID, subscription
//! store, send) is `hypha_push`; what to notify and when stays
//! Peregrin's.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};

use peregrin_secret_cuora::{BIN_ENV, CuoraServe, CuoraTransport, NAMESPACE, TRANSPORT_ENV};

use hypha_kv::KvStore;
use hypha_push::{Kind, PushStore};
use sepia_proto::SessionEvent;
use serde_json::Value;

/// The node's push persistence — the internal `push` KV namespace
/// (`push/vapid`, `push/pushSubscriptions`), so secrets ride the same
/// store-API-only visibility `/api/kv*` enforces for internal
/// namespaces. A newtype because both `KvStore` and `PushBackend` are
/// foreign to this crate's push surface (orphan rule) — the wrapper
/// is the local type the impl hangs on.
#[derive(Clone)]
pub struct KvPushBackend(pub Arc<dyn KvStore>);

impl hypha_push::PushBackend for KvPushBackend {
    fn config(&self) -> BTreeMap<String, Value> {
        self.0.list(crate::kv::NS_PUSH)
    }

    fn set_config(&self, key: &str, value: Value) {
        self.0.set(crate::kv::NS_PUSH, key, value);
    }
}

/// Cuora ref namespace — push keys live one level under the
/// `peregrin` secret namespace: `cuora://peregrin/push/<key>`.
const CUORA_NAMESPACE: &str = "peregrin/push";

/// The config keys [`PushStore`] persists — hydrated once at open
/// (VAPID + subscriptions are the whole document today).
const KEYS: [&str; 2] = ["vapid", "pushSubscriptions"];

/// `cuora://peregrin/push/<key>` — the ref one push config key
/// resolves to. Values are stored as JSON *text* (the `Value`
/// serialized), so `cuora get` output parses straight back.
fn cuora_ref(key: &str) -> String {
    format!("cuora://{CUORA_NAMESPACE}/{key}")
}

/// The `cuora` binary — `PEREGRIN_CUORA_BIN` when set (tests stub
/// it), else `cuora` on PATH. `CUORA_ENV`/`--env` resolution stays
/// cuora's own: the child inherits the environment.
fn cuora_bin() -> PathBuf {
    std::env::var_os(BIN_ENV).map_or_else(|| PathBuf::from("cuora"), PathBuf::from)
}

/// `cuora get <ref>` → the raw value (one trailing newline stripped),
/// `None` on any failure — missing binary, denied op, absent ref all
/// read as "not here" so the kv fallback takes over. The `cli`
/// transport leg; `serve` goes through [`CuoraServe::get`], which
/// applies the same collapse-to-`None` semantics.
fn cuora_get(bin: &PathBuf, key: &str) -> Option<String> {
    let o = Command::new(bin)
        .args(["--actor", "peregrin", "get", &cuora_ref(key)])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !o.status.success() {
        return None;
    }
    let mut s = String::from_utf8_lossy(&o.stdout).into_owned();
    if let Some(stripped) = s.strip_suffix('\n') {
        s = stripped.strip_suffix('\r').unwrap_or(stripped).to_string();
    }
    (!s.is_empty()).then_some(s)
}

/// `cuora put <ref>` with `value` on stdin (never argv — argv lands
/// in shell history and `/proc`).
fn cuora_put(bin: &PathBuf, key: &str, value: &str) -> std::io::Result<()> {
    let mut c = Command::new(bin)
        .args(["--actor", "peregrin", "put", &cuora_ref(key)])
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
            cuora_ref(key)
        )))
    }
}

/// The cuora-backed [`hypha_push::PushBackend`] — push state
/// (VAPID keys + subscriptions) lives in the secrets broker at
/// `cuora://peregrin/push/<key>` while the KV `push` namespace
/// stays warm as a fallback.
///
/// **Caching is load-bearing.** [`PushStore::open`] *mints* a fresh
/// VAPID pair on a config miss and [`PushBackend::set_config`]
/// returns `()` (write failures are silent), so this backend must
/// never re-read cuora per call — `config()` is hit on every push
/// send. Instead the document hydrates **once** at [`Self::open`]
/// into an in-memory map:
///
/// - per key, read order is cuora → the kv `push` namespace (warm
///   fallback, kept current by the dual-write below) → absent;
/// - `config()` serves only the cache — no per-send subprocess;
/// - `set_config` upserts the cache, `cuora put`s the JSON text,
///   and mirrors the `Value` into the kv namespace. A failed cuora
///   put keeps the in-memory value + kv mirror and warns — the next
///   boot retries cuora, and an unreachable broker can never cause a
///   re-mint as long as kv holds the last write.
///
/// cuora values cap around 64KiB; `pushSubscriptions` is ~500B per
/// subscription, so the array fits comfortably at realistic fleet
/// sizes — revisit if subscription counts grow by orders of
/// magnitude.
pub struct CuoraPushBackend {
    /// The `cuora` binary (`PEREGRIN_CUORA_BIN` or PATH).
    bin: PathBuf,
    /// `cli` per-call shells vs the persistent `cuora serve` child —
    /// `PEREGRIN_CUORA_TRANSPORT`, same knob as `CuoraSecrets`.
    transport: CuoraTransport,
    /// The serve client — lazy spawn, shared by hydrate + every
    /// `set_config` write. Constructed always; only the `serve`
    /// transport ever spawns the child.
    serve: CuoraServe,
    /// The kv `push` namespace — warm-fallback read source and the
    /// write-through mirror target during the migration.
    kv: Arc<dyn KvStore>,
    /// The hydrated config document — the only thing `config()`
    /// ever serves.
    cache: Mutex<BTreeMap<String, Value>>,
}

impl CuoraPushBackend {
    /// Hydrate from cuora (per-key, falling back to the kv `push`
    /// namespace) and return the backend. The `cuora` binary comes
    /// from `PEREGRIN_CUORA_BIN` or PATH; the transport from
    /// `PEREGRIN_CUORA_TRANSPORT`.
    pub fn open(kv: Arc<dyn KvStore>) -> Self {
        Self::open_with_bin(kv, cuora_bin())
    }

    /// [`Self::open`] with an explicit `cuora` binary — the test seam.
    pub fn open_with_bin(kv: Arc<dyn KvStore>, bin: PathBuf) -> Self {
        Self::open_with(kv, bin, CuoraTransport::from_env(TRANSPORT_ENV))
    }

    /// [`Self::open_with_bin`] with an explicit transport — the
    /// serve-transport test seam (env can't be set in tests).
    pub fn open_with(kv: Arc<dyn KvStore>, bin: PathBuf, transport: CuoraTransport) -> Self {
        let serve = CuoraServe::new(bin.clone(), NAMESPACE);
        let mut cache = BTreeMap::new();
        for key in KEYS {
            // cuora first: its value is JSON text — parse it back to
            // the `Value` the kv namespace would have stored. A
            // failed get *or* an unparseable body falls through to
            // the warm kv copy.
            let value = cuora_get_for(&bin, &serve, transport, key)
                .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
                .or_else(|| kv.get(crate::kv::NS_PUSH, key));
            if let Some(value) = value {
                cache.insert(key.to_string(), value);
            }
        }
        Self {
            bin,
            transport,
            serve,
            kv,
            cache: Mutex::new(cache),
        }
    }

    /// `cuora put` for one push key on the configured transport.
    fn put(&self, key: &str, value: &str) -> std::io::Result<()> {
        match self.transport {
            CuoraTransport::Cli => cuora_put(&self.bin, key, value),
            CuoraTransport::Serve => self.serve.put(&cuora_ref(key), value),
        }
    }
}

/// The transport-dispatched `cuora get` — `cli` shells out per call,
/// `serve` reuses the persistent child. Both collapse every failure
/// to `None`.
fn cuora_get_for(
    bin: &PathBuf,
    serve: &CuoraServe,
    transport: CuoraTransport,
    key: &str,
) -> Option<String> {
    match transport {
        CuoraTransport::Cli => cuora_get(bin, key),
        CuoraTransport::Serve => serve.get(&cuora_ref(key)),
    }
}

impl hypha_push::PushBackend for CuoraPushBackend {
    fn config(&self) -> BTreeMap<String, Value> {
        self.cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    fn set_config(&self, key: &str, value: Value) {
        self.cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(key.to_string(), value.clone());
        // cuora is canonical; the kv mirror keeps the warm fallback
        // honest so a broker outage never strands the document.
        match serde_json::to_string(&value) {
            Ok(json) => {
                if let Err(e) = self.put(key, &json) {
                    tracing::warn!(
                        "cuora put {} failed ({e}) — push config kept in \
                         memory + kv mirror; retried on next write",
                        cuora_ref(key)
                    );
                }
            }
            Err(e) => {
                tracing::warn!("push config {key} not JSON-serializable ({e}) — kv mirror only");
            }
        }
        self.kv.set(crate::kv::NS_PUSH, key, value);
    }
}

/// The env-selected push backend — `PEREGRIN_PUSH_BACKEND`
/// (`crate::env::PushBackendKind`) picks between the kv.json `push`
/// namespace and the cuora broker. An enum because `PushStore::open`
/// is generic over the concrete backend and the orphan rule bars a
/// `PushBackend for Box<dyn PushBackend>` impl here.
pub enum NodePushBackend {
    /// [`KvPushBackend`] — `kv.json`'s `push` namespace.
    Kv(KvPushBackend),
    /// [`CuoraPushBackend`] — the cuora broker + kv mirror.
    Cuora(CuoraPushBackend),
}

impl NodePushBackend {
    /// Build the backend `env.push_backend` selects.
    #[must_use]
    pub fn for_kind(kv: Arc<dyn KvStore>, kind: crate::env::PushBackendKind) -> Self {
        match kind {
            crate::env::PushBackendKind::Kv => Self::Kv(KvPushBackend(kv)),
            crate::env::PushBackendKind::Cuora => Self::Cuora(CuoraPushBackend::open(kv)),
        }
    }
}

impl hypha_push::PushBackend for NodePushBackend {
    fn config(&self) -> BTreeMap<String, Value> {
        match self {
            Self::Kv(b) => b.config(),
            Self::Cuora(b) => b.config(),
        }
    }

    fn set_config(&self, key: &str, value: Value) {
        match self {
            Self::Kv(b) => b.set_config(key, value),
            Self::Cuora(b) => b.set_config(key, value),
        }
    }
}

/// Session-event → notification mapping — run completion and permission
/// requests, matching the TS `notifyForEvents`.
pub fn notify_for_events(
    push: &PushStore,
    session_id: &str,
    agent: Option<&str>,
    session_title: &str,
    events: &[SessionEvent],
) {
    let key = agent.map_or_else(|| session_id.to_string(), |a| format!("{a}:{session_id}"));
    let url = format!("/?session={}", urlencoding(&key));
    if events
        .iter()
        .any(|e| matches!(e, SessionEvent::RunFinished { .. }))
    {
        push.send(
            Kind::Done,
            "Session finished",
            &format!("{session_title} finished its run."),
            &url,
        );
    }
    let wants_permission = events.iter().any(
        |e| matches!(e, SessionEvent::Custom { name, .. } if name == "acp:permission_request"),
    );
    if wants_permission {
        push.send(
            Kind::Permission,
            "Approval needed",
            &format!("{session_title} is waiting for you."),
            &url,
        );
    }
}

fn urlencoding(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || "-_.~".contains(c) {
                c.to_string()
            } else {
                format!("%{:02X}", c as u32)
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use hypha_push::PushBackend as _;
    use std::os::unix::fs::PermissionsExt;

    /// A stub `cuora` binary: `get` answers per-ref canned JSON text
    /// from `$dir/get-<key>`, `put` captures stdin to `$dir/put-<key>`,
    /// every call appends to `$dir/calls.log`. `fail` makes every
    /// invocation exit 1 (the unreachable-broker case).
    fn stub_cuora(dir: &std::path::Path, fail: bool) -> PathBuf {
        let script = format!(
            "#!/bin/sh\n\
             echo \"$3 $4\" >> '{dir}/calls.log'\n\
             if [ '{fail}' = '1' ]; then exit 1; fi\n\
             cmd=\"$3\"; ref=\"$4\"; key=\"${{ref##*/}}\"\n\
             if [ \"$cmd\" = get ]; then\n\
               f='{dir}/get-'\"$key\"\n\
               [ -f \"$f\" ] || exit 1\n\
               cat \"$f\"\n\
             elif [ \"$cmd\" = put ]; then\n\
               cat > '{dir}/put-'\"$key\"\n\
             else exit 1; fi\n",
            dir = dir.display(),
            fail = if fail { "1" } else { "0" },
        );
        let bin = dir.join("cuora-stub.sh");
        std::fs::write(&bin, script).unwrap();
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
        bin
    }

    fn kv_store() -> (tempfile::TempDir, Arc<dyn KvStore>) {
        let dir = tempfile::tempdir().unwrap();
        let kv: Arc<dyn KvStore> = Arc::new(hypha_kv_file::JsonKvStore::open(
            &dir.path().join("kv.json"),
        ));
        (dir, kv)
    }

    #[test]
    fn cuora_backend_hydrates_from_cuora() {
        let (dir, kv) = kv_store();
        let vapid = serde_json::json!({"publicKey": "pk", "privateKey": "sk"});
        let subs = serde_json::json!([{"endpoint": "https://push.example/x"}]);
        std::fs::write(
            dir.path().join("get-vapid"),
            serde_json::to_string(&vapid).unwrap(),
        )
        .unwrap();
        std::fs::write(
            dir.path().join("get-pushSubscriptions"),
            serde_json::to_string(&subs).unwrap(),
        )
        .unwrap();
        let bin = stub_cuora(dir.path(), false);
        let backend = CuoraPushBackend::open_with_bin(kv, bin);
        let config = backend.config();
        assert_eq!(config.get("vapid"), Some(&vapid));
        assert_eq!(config.get("pushSubscriptions"), Some(&subs));
    }

    #[test]
    fn cuora_backend_falls_back_to_kv_when_cuora_fails() {
        let (dir, kv) = kv_store();
        let vapid = serde_json::json!({"publicKey": "kv-pk", "privateKey": "kv-sk"});
        kv.set(crate::kv::NS_PUSH, "vapid", vapid.clone());
        let bin = stub_cuora(dir.path(), true);
        let backend = CuoraPushBackend::open_with_bin(kv.clone(), bin);
        assert_eq!(backend.config().get("vapid"), Some(&vapid));
        // …and PushStore must not re-mint over the warm kv copy.
        let store = hypha_push::PushStore::open(backend);
        assert_eq!(store.public_key(), "kv-pk");
    }

    #[test]
    fn cuora_backend_dual_writes_set_config() {
        let (dir, kv) = kv_store();
        let bin = stub_cuora(dir.path(), false);
        let backend = CuoraPushBackend::open_with_bin(kv.clone(), bin);
        let value = serde_json::json!({"publicKey": "new-pk", "privateKey": "new-sk"});
        backend.set_config("vapid", value.clone());
        // cuora got the JSON *text* on stdin…
        let put = std::fs::read_to_string(dir.path().join("put-vapid")).unwrap();
        assert_eq!(serde_json::from_str::<Value>(&put).unwrap(), value);
        // …the kv mirror got the Value…
        assert_eq!(kv.get(crate::kv::NS_PUSH, "vapid"), Some(value.clone()));
        // …and the cache serves it without another subprocess.
        let calls_before = std::fs::read_to_string(dir.path().join("calls.log")).unwrap();
        assert_eq!(backend.config().get("vapid"), Some(&value));
        let calls_after = std::fs::read_to_string(dir.path().join("calls.log")).unwrap();
        assert_eq!(calls_before, calls_after);
    }

    #[test]
    fn cuora_backend_keeps_kv_mirror_when_put_fails() {
        let (dir, kv) = kv_store();
        // get works (empty), put fails — flip the stub mid-test by
        // pointing `put` at a missing capture is overkill; instead
        // use a stub whose put leg exits 1.
        let script = format!(
            "#!/bin/sh\n\
             echo \"$3 $4\" >> '{dir}/calls.log'\n\
             if [ \"$3\" = put ]; then exit 1; fi\n\
             exit 1\n",
            dir = dir.path().display(),
        );
        let bin = dir.path().join("cuora-stub.sh");
        std::fs::write(&bin, script).unwrap();
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
        let backend = CuoraPushBackend::open_with_bin(kv.clone(), bin);
        let value = serde_json::json!({"publicKey": "p", "privateKey": "s"});
        backend.set_config("vapid", value.clone());
        assert_eq!(backend.config().get("vapid"), Some(&value));
        assert_eq!(kv.get(crate::kv::NS_PUSH, "vapid"), Some(value));
    }

    /// A fake `cuora serve` — ndjson on stdin/stdout. Push values
    /// are JSON *text* (quotes and all), so `value` is extracted
    /// with sed out to the `","actor"` boundary and kept
    /// wire-escaped end-to-end: the db file stores the escaped form
    /// and `get` re-emits it inside `"value":"…"`, which parses back
    /// to the original text.
    fn stub_cuora_serve(dir: &std::path::Path) -> PathBuf {
        let script = format!(
            "#!/bin/sh\n\
             while IFS= read -r line; do\n\
               op=$(printf '%s' \"$line\" | grep -o '\"op\":\"[a-z]*\"' | cut -d'\"' -f4)\n\
               ref=$(printf '%s' \"$line\" | grep -o '\"ref\":\"[^\"]*\"' | cut -d'\"' -f4)\n\
               val=$(printf '%s' \"$line\" | sed -n 's/.*\"value\":\"\\(.*\\)\",\"actor\":\"[^\"]*\".*/\\1/p')\n\
               key=${{ref##*/}}\n\
               f='{dir}/db-'\"$key\"\n\
               case \"$op\" in\n\
                 get) if [ -f \"$f\" ]; then \\\n                        v=$(cat \"$f\")\n\
                        printf '{{\"id\":1,\"ok\":true,\"result\":{{\"value\":\"%s\"}}}}\\n' \"$v\"\n\
                      else printf '{{\"id\":1,\"ok\":false,\"error\":{{\"code\":\"not_found\",\"message\":\"x\"}}}}\\n'; fi ;;\n\
                 put) printf '%s' \"$val\" > \"$f\"\n\
                      printf '{{\"id\":1,\"ok\":true,\"result\":{{}}}}\\n' ;;\n\
                 *) printf '{{\"id\":1,\"ok\":false,\"error\":{{\"code\":\"unknown_op\",\"message\":\"x\"}}}}\\n' ;;\n\
               esac\n\
             done\n",
            dir = dir.display(),
        );
        let bin = dir.join("cuora-serve-stub.sh");
        std::fs::write(&bin, script).unwrap();
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
        bin
    }

    #[test]
    fn cuora_backend_serve_transport_roundtrip() {
        let (dir, kv) = kv_store();
        let bin = stub_cuora_serve(dir.path());
        let backend = CuoraPushBackend::open_with(kv.clone(), bin.clone(), CuoraTransport::Serve);
        // Nothing stored anywhere → empty hydrate.
        assert!(backend.config().is_empty());
        // set_config puts over the wire — the db file proves it —
        // while cache + kv mirror behave exactly as under cli.
        let vapid = serde_json::json!({"publicKey": "pk", "privateKey": "sk"});
        backend.set_config("vapid", vapid.clone());
        assert!(dir.path().join("db-vapid").exists());
        assert_eq!(backend.config().get("vapid"), Some(&vapid));
        assert_eq!(kv.get(crate::kv::NS_PUSH, "vapid"), Some(vapid.clone()));
        // A fresh backend hydrates *from cuora* over a new serve
        // child — the stored JSON text parses back to the Value.
        let backend2 = CuoraPushBackend::open_with(kv, bin, CuoraTransport::Serve);
        assert_eq!(backend2.config().get("vapid"), Some(&vapid));
    }

    #[test]
    fn cuora_backend_mints_once_and_mirrors_when_empty_everywhere() {
        let (dir, kv) = kv_store();
        let bin = stub_cuora(dir.path(), true); // cuora unreachable
        let store = hypha_push::PushStore::open(CuoraPushBackend::open_with_bin(kv.clone(), bin));
        // The mint landed in the kv mirror — the next boot's warm
        // fallback — even though cuora refused the put.
        let vapid = kv.get(crate::kv::NS_PUSH, "vapid").unwrap();
        assert_eq!(vapid["publicKey"], store.public_key());
        // A second open over the same kv must reuse it, not re-mint.
        let store2 = hypha_push::PushStore::open(CuoraPushBackend::open_with_bin(
            kv,
            stub_cuora(dir.path(), true),
        ));
        assert_eq!(store2.public_key(), store.public_key());
    }
}
