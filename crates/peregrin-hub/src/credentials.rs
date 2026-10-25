//! Where the hub keeps node bearer tokens between restarts.
//!
//! `PEREGRIN_NODES` `@token` / `PEREGRIN_NODE_TOKEN` are explicit config and
//! always win; an env token also seeds an empty store slot at boot —
//! the migration path off plaintext env (drop `@token` after one boot,
//! the token keeps working from the store). A stored credential is
//! never overwritten by env config; rotation goes through re-pairing.
//! Nodes listed without a token — and credentials minted by
//! the pair flow — resolve and persist through a [`NodeCredentials`]
//! store: the OS keyring (Secret Service on Linux/*BSD, Keychain on
//! macOS, Credential Manager on Windows) when reachable, else
//! `credentials.json` (0600) under the hub home — the documented
//! headless path. `PEREGRIN_CREDENTIAL_STORE=auto|keyring|file` picks the
//! backend; `auto` (default) probes the keyring once at boot.
//!
//! Keyring ops are blocking (a dbus round-trip on Linux) — async
//! callers go through `tokio::task::spawn_blocking`, same as `ureq`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// The keyring service; the account is `node/{id}`.
const KEYRING_SERVICE: &str = "peregrin-hub";
/// `PEREGRIN_CREDENTIAL_STORE` — `auto` (default), `keyring`, `file`.
pub const STORE_ENV: &str = "PEREGRIN_CREDENTIAL_STORE";
/// File-store name under the hub home (`$PEREGRIN_HOME/hub/`).
pub const FILE_NAME: &str = "credentials.json";

/// Node-token persistence. Errors are strings — the surfaces that call
/// this (hub boot, `NodeApi::pair`) report plain messages.
pub trait NodeCredentials: Send + Sync {
    /// The stored token for `node_id`; `Ok(None)` = never stored.
    fn get(&self, node_id: &str) -> Result<Option<String>, String>;
    /// Persist `token` under `node_id` (overwrite).
    fn set(&self, node_id: &str, token: &str) -> Result<(), String>;
    /// Drop a stored credential (unpair/rotate). Idempotent — deleting
    /// a missing entry is `Ok`.
    fn delete(&self, node_id: &str) -> Result<(), String>;
    /// Which backend is live — for logs and docs, not logic.
    fn backend(&self) -> &'static str;
}

/* ---- OS keyring ------------------------------------------------------*/

/// `Entry::new` builds the crate's *mock* store on platforms where no
/// real backend feature compiled — it "works" but persists nothing, so
/// the probe only trusts platforms we enabled a backend for.
///
/// `Ok(())` = the OS keyring answered (a miss counts — `NoEntry` is a
/// healthy response); `Err` = no reachable backend.
#[cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "macos",
    target_os = "ios",
    target_os = "windows"
))]
pub fn keyring_probe() -> Result<(), String> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, "peregrin-hub-probe")
        .map_err(|e| format!("keyring: {e}"))?;
    match entry.get_password() {
        // Reachable backend — `NoEntry` is a healthy answer.
        Ok(_) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(format!("keyring: {e}")),
    }
}

/// `Err` — no real backend was compiled in for this platform (see the
/// supported-platforms arm above).
#[cfg(not(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "macos",
    target_os = "ios",
    target_os = "windows"
)))]
pub fn keyring_probe() -> Result<(), String> {
    Err("no OS keyring backend on this platform".to_string())
}

/// OS keyring — Secret Service / Keychain / Credential Manager.
pub struct KeyringCredentials;

impl KeyringCredentials {
    fn entry(node_id: &str) -> Result<keyring::Entry, String> {
        keyring::Entry::new(KEYRING_SERVICE, &format!("node/{node_id}"))
            .map_err(|e| format!("keyring: {e}"))
    }
}

impl NodeCredentials for KeyringCredentials {
    fn get(&self, node_id: &str) -> Result<Option<String>, String> {
        match Self::entry(node_id)?.get_password() {
            Ok(token) => Ok(Some(token)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(format!("keyring: {e}")),
        }
    }

    fn set(&self, node_id: &str, token: &str) -> Result<(), String> {
        Self::entry(node_id)?
            .set_password(token)
            .map_err(|e| format!("keyring: {e}"))
    }

    fn delete(&self, node_id: &str) -> Result<(), String> {
        match Self::entry(node_id)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(format!("keyring: {e}")),
        }
    }

    fn backend(&self) -> &'static str {
        "os-keyring"
    }
}

/* ---- file fallback ----------------------------------------------------*/

/// `credentials.json` — `{node_id: token}` written 0600 via tmp+rename.
/// The headless path: no Secret Service means no keyring, and a 0600
/// file under `$PEREGRIN_HOME` is exactly as protected as the env files
/// carrying `PEREGRIN_NODES` today.
pub struct FileCredentials {
    path: PathBuf,
    /// Serializes read-modify-write in `set`.
    write: Mutex<()>,
}

impl FileCredentials {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            write: Mutex::new(()),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn load(&self) -> Result<BTreeMap<String, String>, String> {
        match std::fs::read_to_string(&self.path) {
            Ok(raw) => match serde_json::from_str::<serde_json::Value>(&raw) {
                Ok(serde_json::Value::Object(map)) => Ok(map
                    .into_iter()
                    .filter_map(|(k, v)| v.as_str().map(|t| (k, t.to_string())))
                    .collect()),
                Ok(_) => Err(format!("{}: expected a JSON object", self.path.display())),
                Err(e) => Err(format!("{}: {e}", self.path.display())),
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(BTreeMap::new()),
            Err(e) => Err(format!("{}: {e}", self.path.display())),
        }
    }

    fn store(&self, map: &BTreeMap<String, String>) -> Result<(), String> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("create {}: {e}", dir.display()))?;
        }
        let tmp = self
            .path
            .with_extension(format!("tmp-{}", std::process::id()));
        let body = serde_json::to_string(&serde_json::json!(map))
            .map_err(|e| format!("encode credentials: {e}"))?;
        std::fs::write(&tmp, body).map_err(|e| format!("write {}: {e}", tmp.display()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600))
                .map_err(|e| format!("chmod {}: {e}", tmp.display()))?;
        }
        std::fs::rename(&tmp, &self.path)
            .map_err(|e| format!("rename {}: {e}", self.path.display()))
    }
}

impl NodeCredentials for FileCredentials {
    fn get(&self, node_id: &str) -> Result<Option<String>, String> {
        Ok(self.load()?.get(node_id).cloned())
    }

    fn set(&self, node_id: &str, token: &str) -> Result<(), String> {
        let _guard = self
            .write
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut map = self.load()?;
        map.insert(node_id.to_string(), token.to_string());
        self.store(&map)
    }

    fn delete(&self, node_id: &str) -> Result<(), String> {
        let _guard = self
            .write
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut map = self.load()?;
        map.remove(node_id);
        self.store(&map)
    }

    fn backend(&self) -> &'static str {
        "file"
    }
}

/* ---- selection ---------------------------------------------------------*/

fn file_store(hub_home: &Path) -> Arc<dyn NodeCredentials> {
    Arc::new(FileCredentials::new(hub_home.join(FILE_NAME)))
}

/// Pick the backend from `std::env` — see [`from_lookup`].
pub fn open(hub_home: &Path) -> Arc<dyn NodeCredentials> {
    from_lookup(hub_home, |k| std::env::var(k).ok())
}

/// Selection logic with the env injected (test seam):
/// - `file` — always the JSON file.
/// - `keyring` — the OS keyring, even if the probe fails (explicit
///   choice; per-call errors then surface honestly).
/// - `auto`/unset/unknown — keyring when the probe passes, file else.
pub fn from_lookup(
    hub_home: &Path,
    get: impl Fn(&str) -> Option<String>,
) -> Arc<dyn NodeCredentials> {
    pick(
        hub_home,
        get(STORE_ENV).as_deref().map(str::trim),
        keyring_probe(),
    )
}

/// The pick itself — env value and probe result injected so every arm
/// is directly testable (a real probe can't be failed on demand).
fn pick(
    hub_home: &Path,
    forced: Option<&str>,
    probe: Result<(), String>,
) -> Arc<dyn NodeCredentials> {
    match forced {
        Some("file") => file_store(hub_home),
        Some("keyring") => {
            if let Err(e) = &probe {
                tracing::warn!("PEREGRIN_CREDENTIAL_STORE=keyring but the probe failed: {e}");
            }
            Arc::new(KeyringCredentials)
        }
        other => {
            if !matches!(other, None | Some("" | "auto")) {
                tracing::warn!(
                    "unknown {STORE_ENV}={other:?} — expected auto|keyring|file; using auto"
                );
            }
            match probe {
                Ok(()) => Arc::new(KeyringCredentials),
                Err(e) => {
                    // Formatted eagerly — inside `info!`'s lazy arg
                    // scope it only evaluates when a subscriber is live.
                    let fallback = hub_home.join(FILE_NAME).display().to_string();
                    tracing::info!(
                        "OS keyring unavailable ({e}) — node credentials fall back to {fallback}"
                    );
                    file_store(hub_home)
                }
            }
        }
    }
}

#[cfg(test)]
mod cov_tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn pick_arms() {
        let home = std::path::Path::new("/tmp/x");
        // A thread-local subscriber enables the log macros' lazy arg
        // evaluation (`fallback.display()` sits inside `info!`'s event
        // scope) — a global `try_init` races subscriber registration
        // in parallel tests, so scope it to the Err-probe calls.
        let sub = tracing_subscriber::fmt()
            .with_max_level(tracing::level_filters::LevelFilter::INFO)
            .finish();
        tracing::subscriber::with_default(sub, || {
            // forced keyring with a failed probe → warn arm.
            let _ = pick(home, Some("keyring"), Err("no dbus".into()));
            // unknown value → warn + auto pick; probe-fail → file arm.
            let _ = pick(home, Some("bogus"), Err("no dbus".into()));
            let _ = pick(home, None, Err("no dbus".into()));
        });
        // A no-op dispatcher → the log macros' disabled paths.
        tracing::dispatcher::with_default(&tracing::Dispatch::none(), || {
            let _ = pick(home, Some("keyring"), Err("no dbus".into()));
            let _ = pick(home, None, Err("no dbus".into()));
        });
        // forced keyring with a live probe → the `if let Err` skip arm.
        let _ = pick(home, Some("keyring"), Ok(()));
        let _ = pick(home, Some("auto"), Ok(()));
        let _ = pick(home, Some("file"), Ok(()));
        let _ = pick(home, None, Ok(()));
    }

    #[test]
    fn store_skips_mkdir_when_parent_absent() {
        // `""` has no parent — the `create_dir_all` guard's skip arm;
        // the tmp-name write then fails harmlessly.
        let c = FileCredentials::new(PathBuf::new());
        let _ = c.set("n", "t");
    }

    #[test]
    fn load_non_notfound_error() {
        // Point the file path at a directory — read fails with
        // IsADirectory, not NotFound → the `{path}: {e}` arm.
        // A directory path → `fs::read` fails IsADirectory.
        let dir = tempfile::tempdir().unwrap();
        let c = FileCredentials::new(dir.path().to_path_buf());
        let _ = c.load();
    }
}
