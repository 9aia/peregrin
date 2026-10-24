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

/// Locate the `sepia` binary: `PEREGRIN_SEPIA` env, then dev builds —
/// sibling checkout (`../sepia/target/{debug,release}`), the testkit
/// bin dirs (`TESTKIT_BIN_DIRS`, env-split), and the shared cargo
/// target dir (`$HOME/.cargo/shared-target/{debug,release}`, the
/// global `target-dir` on this machine) — all fresher than an
/// installed PATH sepia in a monorepo checkout. PATH is the fallback.
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
    if let Some(dirs) = std::env::var_os("TESTKIT_BIN_DIRS") {
        for dir in std::env::split_paths(&dirs) {
            let candidate = dir.join("sepia");
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    if let Some(home) = std::env::var_os("HOME") {
        for profile in ["debug", "release"] {
            let candidate =
                PathBuf::from(&home).join(format!(".cargo/shared-target/{profile}/sepia"));
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
        if manifest.protocol != EXPECTED_PROTOCOL {
            return Err(RpcError::internal(format!(
                "sepia protocol {} unsupported (client speaks {EXPECTED_PROTOCOL})",
                manifest.protocol
            )));
        }
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

/// Fetch + parse the manifest without gating on protocol — the two
/// callers gate differently: `spawn` demands `EXPECTED_PROTOCOL`,
/// `respawn` demands the protocol the service originally booted with.
async fn handshake(client: &DriverClient) -> Result<Manifest, RpcError> {
    let raw = client.call("manifest", json!({})).await?;
    serde_json::from_value(raw).map_err(|e| RpcError::internal(format!("manifest: {e}")))
}

/* ---- typed surface over `sepia serve`'s methods ---- */

use sepia_acp::types::PromptPart;
use sepia_core::Session;
use sepia_proto::SessionEvent;
use sepia_session::{
    AgentInfo, AttachResult, ControlError, ControlErrorCode, CreateResult, HistoryOptions,
    HistoryPage, RestoreRequest, RestoreResult, RewindRequest, RewindResult, SessionSummary,
    agent_for_backend,
};

fn control(code: ControlErrorCode, e: impl std::fmt::Display) -> ControlError {
    ControlError::new(code, e.to_string())
}

fn to_control(e: RpcError) -> ControlError {
    let (code, cause) = e
        .data
        .as_ref()
        .map_or((ControlErrorCode::Internal, None), |d| {
            let code = d
                .get("code")
                .and_then(|c| serde_json::from_value::<ControlErrorCode>(c.clone()).ok())
                .unwrap_or(ControlErrorCode::Internal);
            let cause = d.get("cause").and_then(|c| c.as_str()).map(str::to_string);
            (code, cause)
        });
    ControlError {
        code,
        message: e.message,
        cause,
    }
}

impl SepiaService {
    async fn decode<T: serde::de::DeserializeOwned>(
        &self,
        method: &str,
        params: Value,
    ) -> Result<T, ControlError> {
        let v = self.call(method, params).await.map_err(to_control)?;
        serde_json::from_value(v).map_err(|e| control(ControlErrorCode::Internal, e))
    }

    fn agent_param(agent: Option<&str>) -> Value {
        agent.map_or_else(|| json!({}), |a| json!({ "agent": a }))
    }

    /// `sessions.list`.
    pub async fn list_sessions_t(
        &self,
        with_locks: bool,
    ) -> Result<Vec<SessionSummary>, ControlError> {
        self.decode("sessions.list", json!({ "withLocks": with_locks }))
            .await
    }

    /// `sessions.get` (full session) / `sessions.summary` (metadata only).
    pub async fn get_session_t(
        &self,
        id: &str,
        agent: Option<&str>,
    ) -> Result<Session, ControlError> {
        let mut p = Self::agent_param(agent);
        p["id"] = json!(id);
        self.decode("sessions.get", p).await
    }

    /// `sessions.summary`.
    pub async fn get_summary_t(
        &self,
        id: &str,
        agent: Option<&str>,
    ) -> Result<Session, ControlError> {
        let mut p = Self::agent_param(agent);
        p["id"] = json!(id);
        self.decode("sessions.summary", p).await
    }

    /// `sessions.history`.
    pub async fn get_history_t(
        &self,
        id: &str,
        options: &HistoryOptions,
        agent: Option<&str>,
    ) -> Result<HistoryPage, ControlError> {
        let mut p = Self::agent_param(agent);
        p["id"] = json!(id);
        if let Some(limit) = options.limit {
            p["limit"] = json!(limit);
        }
        if let Some(before) = options.before {
            p["before"] = json!(before);
        }
        self.decode("sessions.history", p).await
    }

    /// `sessions.attach` — `takeover` preempts a live holder.
    pub async fn attach_t(
        &self,
        id: &str,
        takeover: bool,
        model: Option<String>,
        fallbacks: Option<Vec<String>>,
        agent: Option<String>,
    ) -> Result<AttachResult, ControlError> {
        self.decode(
            "sessions.attach",
            json!({
                "id": id,
                "takeover": takeover,
                "model": model,
                "fallbacks": fallbacks,
                "agent": agent,
            }),
        )
        .await
    }

    /// `sessions.create`.
    pub async fn create_session_t(
        &self,
        cwd: &str,
        agent: Option<String>,
        title: Option<String>,
        model: Option<String>,
        fallbacks: Option<Vec<String>>,
    ) -> Result<CreateResult, ControlError> {
        self.decode(
            "sessions.create",
            json!({
                "cwd": cwd,
                "agent": agent,
                "title": title,
                "model": model,
                "fallbacks": fallbacks,
            }),
        )
        .await
    }

    /// `sessions.detach`.
    pub async fn detach_t(&self, id: &str, agent: Option<&str>) -> Result<(), ControlError> {
        let mut p = Self::agent_param(agent);
        p["id"] = json!(id);
        self.call("sessions.detach", p)
            .await
            .map_err(to_control)
            .map(|_| ())
    }

    /// `sessions.prompt` — ACP `session/prompt` on the holder.
    pub async fn prompt_t(
        &self,
        id: &str,
        parts: Vec<PromptPart>,
        agent: Option<&str>,
    ) -> Result<(), ControlError> {
        let mut p = Self::agent_param(agent);
        p["id"] = json!(id);
        p["parts"] = json!(parts);
        self.call("sessions.prompt", p)
            .await
            .map_err(to_control)
            .map(|_| ())
    }

    /// `sessions.cancel`.
    pub async fn cancel_t(&self, id: &str, agent: Option<&str>) -> Result<(), ControlError> {
        let mut p = Self::agent_param(agent);
        p["id"] = json!(id);
        self.call("sessions.cancel", p)
            .await
            .map_err(to_control)
            .map(|_| ())
    }

    /// `sessions.permission` — resolve a pending permission request.
    pub async fn respond_to_permission_t(
        &self,
        id: &str,
        request_id: &str,
        option: Option<&str>,
        agent: Option<&str>,
    ) -> Result<(), ControlError> {
        let mut p = Self::agent_param(agent);
        p["id"] = json!(id);
        p["requestId"] = json!(request_id);
        if let Some(o) = option {
            p["option"] = json!(o);
        }
        self.call("sessions.permission", p)
            .await
            .map_err(to_control)
            .map(|_| ())
    }

    /// `sessions.delete`.
    pub async fn delete_session_t(
        &self,
        id: &str,
        agent: Option<&str>,
    ) -> Result<(), ControlError> {
        let mut p = Self::agent_param(agent);
        p["id"] = json!(id);
        self.call("sessions.delete", p)
            .await
            .map_err(to_control)
            .map(|_| ())
    }

    /// `sessions.restore` — restore rewindable files to a past state.
    pub async fn restore_t(
        &self,
        id: &str,
        request: &RestoreRequest,
        agent: Option<&str>,
    ) -> Result<RestoreResult, ControlError> {
        let mut p = Self::agent_param(agent);
        p["id"] = json!(id);
        p["request"] = json!(request);
        self.decode("sessions.restore", p).await
    }

    /// `sessions.rewind`.
    pub async fn rewind_t(
        &self,
        id: &str,
        request: &RewindRequest,
        agent: Option<&str>,
    ) -> Result<RewindResult, ControlError> {
        let mut p = Self::agent_param(agent);
        p["id"] = json!(id);
        p["request"] = json!(request);
        self.decode("sessions.rewind", p).await
    }

    /// `agents.list`.
    pub async fn list_agents_t(&self) -> Result<Vec<AgentInfo>, ControlError> {
        self.decode("agents.list", json!({})).await
    }
}

/* ---- the session-plane port ---- */

/// Everything `peregrin-http` can ask of the session plane — the same
/// surface `ControlPlane` exposes, implementable in-process
/// ([`LocalPlane`]) or over `sepia serve` ([`ServicePlane`]).
#[async_trait::async_trait]
pub trait SessionPlane: Send + Sync {
    /// Sessions across every registered store; `with_locks` probes
    /// agents for live lock state.
    async fn list_sessions(&self, with_locks: bool) -> Result<Vec<SessionSummary>, ControlError>;
    /// Full session payload.
    async fn get_session(&self, id: &str, agent: Option<&str>) -> Result<Session, ControlError>;
    /// Metadata-only session (no nodes/prompt history).
    async fn get_summary(&self, id: &str, agent: Option<&str>) -> Result<Session, ControlError>;
    /// Paginated history.
    async fn get_history(
        &self,
        id: &str,
        options: &HistoryOptions,
        agent: Option<&str>,
    ) -> Result<HistoryPage, ControlError>;
    /// Attach to a session — `takeover` preempts a live holder.
    async fn attach(
        &self,
        id: &str,
        takeover: bool,
        model: Option<String>,
        fallbacks: Option<Vec<String>>,
        agent: Option<String>,
    ) -> Result<AttachResult, ControlError>;
    /// Create a session under `cwd` on an agent.
    async fn create_session(
        &self,
        cwd: &str,
        agent: Option<String>,
        title: Option<String>,
        model: Option<String>,
        fallbacks: Option<Vec<String>>,
    ) -> Result<CreateResult, ControlError>;
    /// Detach a held session.
    async fn detach(&self, id: &str);
    /// Forward a prompt to the holder.
    async fn prompt(
        &self,
        id: &str,
        parts: Vec<PromptPart>,
        agent: Option<&str>,
    ) -> Result<(), ControlError>;
    /// Cancel a running turn.
    async fn cancel(&self, id: &str, agent: Option<&str>) -> Result<(), ControlError>;
    /// Resolve a pending permission request.
    async fn respond_to_permission(
        &self,
        id: &str,
        request_id: &str,
        option: Option<&str>,
        agent: Option<&str>,
    ) -> Result<(), ControlError>;
    /// Delete a session from its store.
    async fn delete_session(&self, id: &str, agent: Option<&str>) -> Result<(), ControlError>;
    /// Restore rewindable files.
    async fn restore(
        &self,
        id: &str,
        request: &RestoreRequest,
        agent: Option<&str>,
    ) -> Result<RestoreResult, ControlError>;
    /// Rewind messages/history.
    async fn rewind(
        &self,
        id: &str,
        request: &RewindRequest,
        agent: Option<&str>,
    ) -> Result<RewindResult, ControlError>;
    /// Live event stream for an attached session.
    async fn subscribe(
        &self,
        id: &str,
        agent: Option<&str>,
    ) -> Result<broadcast::Receiver<Vec<SessionEvent>>, ControlError>;
    /// Registered agents + last probed capabilities.
    async fn list_agents(&self) -> Vec<AgentInfo>;
    /// Whether the session's owning agent supports rewind/restore.
    async fn can_rewind(&self, backend_type: &str) -> bool;
    /// Graceful shutdown — detach everything.
    async fn close_all(&self);
}

/// In-process plane — `Arc<ControlPlane>` passthrough. Used by tests and
/// `PEREGRIN_SESSION_PLANE=inprocess`.
pub struct LocalPlane(pub Arc<sepia_session::ControlPlane>);

#[async_trait::async_trait]
impl SessionPlane for LocalPlane {
    async fn list_sessions(&self, with_locks: bool) -> Result<Vec<SessionSummary>, ControlError> {
        self.0.list_sessions(with_locks).await
    }
    async fn get_session(&self, id: &str, agent: Option<&str>) -> Result<Session, ControlError> {
        self.0.get_session(id, agent).await
    }
    async fn get_summary(&self, id: &str, agent: Option<&str>) -> Result<Session, ControlError> {
        self.0.get_summary(id, agent).await
    }
    async fn get_history(
        &self,
        id: &str,
        options: &HistoryOptions,
        agent: Option<&str>,
    ) -> Result<HistoryPage, ControlError> {
        let _ = agent;
        self.0.get_history(id, options).await
    }
    async fn attach(
        &self,
        id: &str,
        takeover: bool,
        model: Option<String>,
        fallbacks: Option<Vec<String>>,
        agent: Option<String>,
    ) -> Result<AttachResult, ControlError> {
        self.0.attach(id, takeover, model, fallbacks, agent).await
    }
    async fn create_session(
        &self,
        cwd: &str,
        agent: Option<String>,
        title: Option<String>,
        model: Option<String>,
        fallbacks: Option<Vec<String>>,
    ) -> Result<CreateResult, ControlError> {
        self.0
            .create_session(cwd, agent, title, model, fallbacks)
            .await
    }
    async fn detach(&self, id: &str) {
        self.0.detach(id).await;
    }
    async fn prompt(
        &self,
        id: &str,
        parts: Vec<PromptPart>,
        agent: Option<&str>,
    ) -> Result<(), ControlError> {
        self.0.prompt(id, &parts, agent).await
    }
    async fn cancel(&self, id: &str, agent: Option<&str>) -> Result<(), ControlError> {
        self.0.cancel(id, agent).await
    }
    async fn respond_to_permission(
        &self,
        id: &str,
        request_id: &str,
        option: Option<&str>,
        agent: Option<&str>,
    ) -> Result<(), ControlError> {
        self.0
            .respond_to_permission(id, request_id, option, agent)
            .await
    }
    async fn delete_session(&self, id: &str, agent: Option<&str>) -> Result<(), ControlError> {
        self.0.delete_session(id, agent).await
    }
    async fn restore(
        &self,
        id: &str,
        request: &RestoreRequest,
        agent: Option<&str>,
    ) -> Result<RestoreResult, ControlError> {
        self.0.restore(id, request, agent).await
    }
    async fn rewind(
        &self,
        id: &str,
        request: &RewindRequest,
        agent: Option<&str>,
    ) -> Result<RewindResult, ControlError> {
        self.0.rewind(id, request, agent).await
    }
    async fn subscribe(
        &self,
        id: &str,
        agent: Option<&str>,
    ) -> Result<broadcast::Receiver<Vec<SessionEvent>>, ControlError> {
        self.0.subscribe(id, agent).await
    }
    async fn list_agents(&self) -> Vec<AgentInfo> {
        self.0.list_agents().await
    }
    async fn can_rewind(&self, backend_type: &str) -> bool {
        self.0.can_rewind(backend_type)
    }
    async fn close_all(&self) {
        self.0.close_all().await;
    }
}

/// `sepia serve` subprocess plane — the production mode.
pub struct ServicePlane(pub SepiaService);

#[async_trait::async_trait]
impl SessionPlane for ServicePlane {
    async fn list_sessions(&self, with_locks: bool) -> Result<Vec<SessionSummary>, ControlError> {
        self.0.list_sessions_t(with_locks).await
    }
    async fn get_session(&self, id: &str, agent: Option<&str>) -> Result<Session, ControlError> {
        self.0.get_session_t(id, agent).await
    }
    async fn get_summary(&self, id: &str, agent: Option<&str>) -> Result<Session, ControlError> {
        self.0.get_summary_t(id, agent).await
    }
    async fn get_history(
        &self,
        id: &str,
        options: &HistoryOptions,
        agent: Option<&str>,
    ) -> Result<HistoryPage, ControlError> {
        self.0.get_history_t(id, options, agent).await
    }
    async fn attach(
        &self,
        id: &str,
        takeover: bool,
        model: Option<String>,
        fallbacks: Option<Vec<String>>,
        agent: Option<String>,
    ) -> Result<AttachResult, ControlError> {
        self.0.attach_t(id, takeover, model, fallbacks, agent).await
    }
    async fn create_session(
        &self,
        cwd: &str,
        agent: Option<String>,
        title: Option<String>,
        model: Option<String>,
        fallbacks: Option<Vec<String>>,
    ) -> Result<CreateResult, ControlError> {
        self.0
            .create_session_t(cwd, agent, title, model, fallbacks)
            .await
    }
    async fn detach(&self, id: &str) {
        let _ = self.0.detach_t(id, None).await;
    }
    async fn prompt(
        &self,
        id: &str,
        parts: Vec<PromptPart>,
        agent: Option<&str>,
    ) -> Result<(), ControlError> {
        self.0.prompt_t(id, parts, agent).await
    }
    async fn cancel(&self, id: &str, agent: Option<&str>) -> Result<(), ControlError> {
        self.0.cancel_t(id, agent).await
    }
    async fn respond_to_permission(
        &self,
        id: &str,
        request_id: &str,
        option: Option<&str>,
        agent: Option<&str>,
    ) -> Result<(), ControlError> {
        self.0
            .respond_to_permission_t(id, request_id, option, agent)
            .await
    }
    async fn delete_session(&self, id: &str, agent: Option<&str>) -> Result<(), ControlError> {
        self.0.delete_session_t(id, agent).await
    }
    async fn restore(
        &self,
        id: &str,
        request: &RestoreRequest,
        agent: Option<&str>,
    ) -> Result<RestoreResult, ControlError> {
        self.0.restore_t(id, request, agent).await
    }
    async fn rewind(
        &self,
        id: &str,
        request: &RewindRequest,
        agent: Option<&str>,
    ) -> Result<RewindResult, ControlError> {
        self.0.rewind_t(id, request, agent).await
    }
    async fn subscribe(
        &self,
        id: &str,
        agent: Option<&str>,
    ) -> Result<broadcast::Receiver<Vec<SessionEvent>>, ControlError> {
        // `sessions.subscribe` replies {"subscribed":true}; `session.event`
        // notifications carry {"sessionId", "events":[...]} — fan them out
        // to a per-subscribe channel filtered by id.
        let session_id = id.to_string();
        let (tx, rx) = broadcast::channel::<Vec<SessionEvent>>(256);
        let mut source = self.0.notifications();
        tokio::spawn(async move {
            while let Ok(note) = source.recv().await {
                if note.method != "session.event"
                    || note.params.get("sessionId").and_then(Value::as_str)
                        != Some(session_id.as_str())
                {
                    continue;
                }
                if let Ok(events) = serde_json::from_value::<Vec<SessionEvent>>(
                    note.params.get("events").cloned().unwrap_or(Value::Null),
                ) {
                    let _ = tx.send(events);
                }
            }
        });
        self.0
            .call("sessions.subscribe", json!({ "id": id, "agent": agent }))
            .await
            .map_err(to_control)?;
        Ok(rx)
    }
    async fn list_agents(&self) -> Vec<AgentInfo> {
        self.0.list_agents_t().await.unwrap_or_default()
    }
    async fn can_rewind(&self, backend_type: &str) -> bool {
        let agent = agent_for_backend(backend_type);
        self.list_agents()
            .await
            .iter()
            .find(|a| a.id == agent)
            .and_then(|a| a.capabilities.as_ref())
            .is_some_and(|c| c.load_session)
    }
    async fn close_all(&self) {
        let _ = self.0.shutdown().await;
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    /// Minimal JSON-RPC "service" as a shell script — enough for
    /// `spawn`'s handshake. `respawn` short-circuits before it would
    /// ever need a second spawn, so no `shutdown` arm is required.
    fn script_spec(dir: &std::path::Path) -> Spec {
        let script = dir.join("stub-service.sh");
        std::fs::write(
            &script,
            "#!/bin/sh\n\
             while IFS= read -r line; do\n\
             id=$(printf '%s' \"$line\" | sed -n 's/.*\"id\":\\([0-9][0-9]*\\).*/\\1/p')\n\
             case \"$line\" in\n\
             *manifest*) printf '{\"jsonrpc\":\"2.0\",\"id\":%s,\"result\":{\"name\":\"stub\",\"version\":\"0\",\"protocol\":1,\"methods\":[]}}\\n' \"${id:-0}\" ;;\n\
             *) printf '{\"jsonrpc\":\"2.0\",\"id\":%s,\"result\":{}}\\n' \"${id:-0}\" ;;\n\
             esac\n\
             done\n",
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        Spec {
            binary: script,
            env: Vec::new(),
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn respawn_alive_client_early_returns() {
        let dir = tempfile::tempdir().unwrap();
        let service = SepiaService::spawn(script_spec(dir.path())).await.unwrap();
        // Client alive → `respawn`'s `!is_closed` early return.
        let client = service.respawn().await.unwrap();
        assert!(!client.is_closed());
        let _ = service.shutdown().await;
    }
}
