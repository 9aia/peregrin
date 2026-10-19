//! peregrin-session — peregrin's session capability, over the wire.
//! Spawns `sepia serve` and drives the local session plane via ndjson
//! JSON-RPC — the same framing `sepia-host` gives drivers. Peregrin
//! never links session internals: sepia owns mechanism, peregrin owns
//! who/when/where.

use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};

use sepia_driver::rpc::{Notification, RpcError};
use sepia_host::DriverClient;
use serde_json::{Value, json};
use tokio::sync::broadcast;

/// The wire protocol this client speaks — must equal the `protocol`
/// `sepia serve`'s `manifest` reports. Bumping the service's protocol
/// on a breaking change makes old peregrin fail fast here, at spawn.
pub const EXPECTED_PROTOCOL: u64 = 1;

/// How to launch `sepia serve`.
#[derive(Debug, Clone)]
pub struct Spec {
    /// The `sepia` binary (resolved by [`sepia_binary`]).
    pub binary: PathBuf,
    /// Extra env for the service process — `SEPIA_HOME`,
    /// `SEPIA_DRIVER_DIR`, `SEPIA_AGENT_*_COMMAND`, telemetry vars.
    pub env: Vec<(String, String)>,
}

impl Default for Spec {
    fn default() -> Self {
        Self {
            binary: sepia_binary().unwrap_or_else(|| PathBuf::from("sepia")),
            env: Vec::new(),
        }
    }
}

/// Locate the `sepia` binary: `PEREGRIN_SEPIA` env, then sibling dev
/// builds (`../sepia/target/{debug,release}/sepia` — fresher than an
/// installed PATH sepia in a monorepo checkout), then PATH.
#[must_use]
pub fn sepia_binary() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("PEREGRIN_SEPIA") {
        return Some(PathBuf::from(path));
    }
    if let Some(manifest) = std::env::var_os("CARGO_MANIFEST_DIR") {
        let root = PathBuf::from(manifest);
        for profile in ["debug", "release"] {
            let candidate = root.join(format!("../../../sepia/target/{profile}/sepia"));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    which_on_path("sepia")
}

fn which_on_path(name: &str) -> Option<PathBuf> {
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|dir| dir.join(name))
            .find(|p| p.is_file())
    })
}

/// Service manifest, negotiated at spawn — name/version/protocol plus
/// the method list. Typed enough to gate on protocol.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Manifest {
    pub name: String,
    pub version: String,
    pub protocol: u64,
    #[serde(default)]
    pub methods: Vec<String>,
}

/// A managed `sepia serve` subprocess — call/notify plus respawn-once
/// semantics (same contract `DriverRegistry` gives store drivers).
pub struct SepiaService {
    client: Arc<Mutex<DriverClient>>,
    spec: Spec,
    pub manifest: Manifest,
    notifications: broadcast::Sender<Notification>,
}

impl SepiaService {
    /// Spawn `sepia serve`, negotiate `manifest`. Fails fast on a
    /// protocol mismatch so stale sepia binaries are caught at boot.
    ///
    /// # Errors
    /// `RpcError` on spawn failure, manifest call failure, or an
    /// incompatible `protocol` version.
    pub async fn spawn(spec: Spec) -> Result<Self, RpcError> {
        let (notifications, _) = broadcast::channel(256);
        let client = spawn_one(&spec, &notifications).await?;
        let manifest = handshake(&client).await?;
        Ok(Self {
            client: Arc::new(Mutex::new(client)),
            spec,
            manifest,
            notifications,
        })
    }

    /// Respawn-once call: a dead service is replaced transparently,
    /// then the request retries once. Permanent failures surface.
    ///
    /// # Errors
    /// `RpcError` from the service or the transport.
    pub async fn call(&self, method: &str, params: Value) -> Result<Value, RpcError> {
        let client = self.current();
        match client.call(method, params.clone()).await {
            Err(e) if client.is_closed() => {
                tracing::warn!("sepia service died ({e}) — respawning");
                let client = self.respawn().await?;
                client.call(method, params).await
            }
            result => result,
        }
    }

    /// `session.event` (and future) notifications — survives respawns.
    pub fn notifications(&self) -> broadcast::Receiver<Notification> {
        self.notifications.subscribe()
    }

    /// Last stderr lines the service emitted — error context when a
    /// call fails or the child dies.
    pub async fn stderr_tail(&self) -> Vec<String> {
        self.current().stderr_tail().await
    }

    /// Replace the child when the reader marks it dead.
    async fn respawn(&self) -> Result<DriverClient, RpcError> {
        {
            let client = self.client.lock().unwrap_or_else(PoisonError::into_inner);
            if !client.is_closed() {
                return Ok(client.clone());
            }
        }
        let client = spawn_one(&self.spec, &self.notifications).await?;
        let manifest = handshake(&client).await?;
        if manifest.protocol != self.manifest.protocol {
            return Err(RpcError::internal(format!(
                "sepia respawned with protocol {}, expected {}",
                manifest.protocol, self.manifest.protocol
            )));
        }
        *self.client.lock().unwrap_or_else(PoisonError::into_inner) = client.clone();
        Ok(client)
    }

    fn current(&self) -> DriverClient {
        self.client
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /* ---- typed surface over `sepia serve`'s methods ---- */

    /// `agents.list`.
    ///
    /// # Errors
    /// Service or transport failure.
    pub async fn agents(&self) -> Result<Vec<Value>, RpcError> {
        serde_json::from_value(self.call("agents.list", json!({})).await?)
            .map_err(|e| RpcError::internal(e.to_string()))
    }

    /// `sessions.list` — `with_locks` merges live lock state.
    ///
    /// # Errors
    /// Service or transport failure.
    pub async fn sessions(&self, with_locks: bool) -> Result<Value, RpcError> {
        self.call("sessions.list", json!({ "withLocks": with_locks }))
            .await
    }

    /// `sessions.attach` — `takeover` preempts the current holder.
    ///
    /// # Errors
    /// Service or transport failure; a held session without `takeover`
    /// errors `locked`.
    pub async fn attach(
        &self,
        id: &str,
        takeover: bool,
        agent: Option<&str>,
    ) -> Result<Value, RpcError> {
        self.call(
            "sessions.attach",
            json!({ "id": id, "takeover": takeover, "agent": agent }),
        )
        .await
    }

    /// `sessions.prompt` — ACP `PromptPart`s (sepia-proto shapes).
    ///
    /// # Errors
    /// Service or transport failure; `busy`/`locked`/`invalid` codes
    /// surface in `RpcError.data.code`.
    pub async fn prompt(
        &self,
        id: &str,
        parts: Value,
        agent: Option<&str>,
    ) -> Result<(), RpcError> {
        self.call(
            "sessions.prompt",
            json!({ "id": id, "parts": parts, "agent": agent }),
        )
        .await
        .map(|_| ())
    }

    /// `sessions.cancel`.
    ///
    /// # Errors
    /// Service or transport failure.
    pub async fn cancel(&self, id: &str, agent: Option<&str>) -> Result<(), RpcError> {
        self.call("sessions.cancel", json!({ "id": id, "agent": agent }))
            .await
            .map(|_| ())
    }

    /// `sessions.detach`.
    pub async fn detach(&self, id: &str) {
        let _ = self.call("sessions.detach", json!({ "id": id })).await;
    }

    /// `sessions.subscribe` — events arrive on [`Self::notifications`].
    ///
    /// # Errors
    /// Service or transport failure; `not_found` when the session
    /// isn't attached anywhere.
    pub async fn subscribe(&self, id: &str, agent: Option<&str>) -> Result<(), RpcError> {
        self.call("sessions.subscribe", json!({ "id": id, "agent": agent }))
            .await
            .map(|_| ())
    }

    /// `sessions.delete`.
    ///
    /// # Errors
    /// Service or transport failure.
    pub async fn delete(&self, id: &str, agent: Option<&str>) -> Result<(), RpcError> {
        self.call("sessions.delete", json!({ "id": id, "agent": agent }))
            .await
            .map(|_| ())
    }

    /// `shutdown` — graceful close (the plane closes, service exits).
    ///
    /// # Errors
    /// Transport failure only.
    pub async fn shutdown(&self) -> Result<(), RpcError> {
        self.call("shutdown", json!({})).await.map(|_| ())
    }
}

async fn spawn_one(
    spec: &Spec,
    notifications: &broadcast::Sender<Notification>,
) -> Result<DriverClient, RpcError> {
    let client = DriverClient::spawn_with_env(&spec.binary, &["serve"], &spec.env)
        .await
        .map_err(|e| RpcError::internal(format!("spawn {}: {e}", spec.binary.display())))?;
    // Forward child notifications into the service-level broadcast so
    // subscribers survive respawns.
    let mut child_rx = client.notifications();
    let sender = notifications.clone();
    tokio::spawn(async move {
        while let Ok(n) = child_rx.recv().await {
            let _ = sender.send(n);
        }
    });
    Ok(client)
}

async fn handshake(client: &DriverClient) -> Result<Manifest, RpcError> {
    let raw = client.call("manifest", json!({})).await?;
    let manifest: Manifest =
        serde_json::from_value(raw).map_err(|e| RpcError::internal(format!("manifest: {e}")))?;
    if manifest.protocol != EXPECTED_PROTOCOL {
        return Err(RpcError::internal(format!(
            "sepia protocol {} unsupported (client speaks {EXPECTED_PROTOCOL})",
            manifest.protocol
        )));
    }
    Ok(manifest)
}
