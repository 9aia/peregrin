//! `/api/flows` — live flow lifecycle on the `campaign serve`
//! listener (ADR-0093): list, add, delete, and reload hosted flows
//! without restarting the daemon — the tamias-managed lifecycle for
//! `peregrin-bundle` deploys.
//!
//! One [`FlowHost`] owns the hosted-flow registry: every flow is a
//! [`supervise`](crate::supervisor::supervise) loop with its own
//! shutdown flag (routes unregister *before* the flag so no new
//! deliveries land on a dying channel), its own queue store and
//! history file, and — for `github-webhook` nodes — routes stamped
//! with the flow's name as `Route::owner` so
//! `SharedWebhook::unregister` can retract them.
//!
//! Wire contract (same bearer gate as `/api/events/inject` —
//! `PEREGRIN_TOKEN`, unset = open dev loopback):
//!
//! - `GET /api/flows` — `{flows: [{name, state, op?, source, repos}]}`;
//!   `state` is `active` or `draining` (+ `op: delete|reload`).
//! - `POST /api/flows` — `{name, yaml}`: validate + load + spawn
//!   live → `201`; `409` on a duplicate, `400` on a bad
//!   name/unloadable yaml, `422` on `runs_on` mismatch, `503` while
//!   the daemon drains. When the host has a `managed_dir` the posted
//!   yaml is *also* written to `<managed_dir>/<name>.yaml` — the
//!   same persistence `peregrin provision deploy` uses, so a restart
//!   replays it.
//! - `GET /api/flows/{name}` — one flow's info row (`404` unknown).
//! - `DELETE /api/flows/{name}` — unregister demux routes, drop the
//!   pending queue ([`SupervisorCmd::ClearPending`]), set the flow's
//!   shutdown flag; the active run finishes and the entry drops when
//!   the join lands → `202 {draining: true}`.
//! - `POST /api/flows/{name}/reload` — drain-then-swap: unregister
//!   routes, drain the old supervisor (pending jobs *run* — only
//!   delete clears the queue), then respawn from the flow's
//!   persisted source. Deliveries inside the drain window 404 — the
//!   gap is scoped to this one flow, strictly smaller than a service
//!   restart → `202 {reloading: true}`.
//!
//! A DELETE arriving mid-reload wins: the reload reaper re-checks
//! the entry's drain op under the write lock before respawning, so
//! a deleted flow never resurrects.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, PoisonError, RwLock};
use std::thread;
use std::time::Duration;

use axum::body::Bytes;
use axum::extract::{Path as AxumPath, State};
use axum::http::{HeaderMap, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::any;
use axum::{Json, Router};
use peregrin_core::{Campaign, Event};
use peregrin_driver_github_webhook::SharedWebhook;
use peregrin_engine::Injector;
use peregrin_loader::Loader;
use peregrin_loader_yaml::YamlLoader;
use serde_json::{Value, json};

use crate::campaign::{self, CampaignError};
use crate::inject::InjectTargets;
use crate::serve::{HostedCampaign, RunCallback};
use crate::supervisor::{self, SupervisorCmd};

/// A campaign supervisor's join handle — drained run logs or the
/// first run failure.
pub type SupervisorJoin = std::thread::JoinHandle<Result<Vec<Event>, CampaignError>>;

/// Where a hosted flow's yaml comes from — reload's input.
#[derive(Clone, Debug)]
pub enum FlowSource {
    /// A yaml file on disk — reload re-reads it, so `tamias deploy`
    /// rewriting `serve/<name>.yaml` then POSTing reload picks the
    /// new doc up. Boot flows and managed-dir POSTs land here.
    Path(PathBuf),
    /// Posted yaml text with no managed dir to persist it to —
    /// reload re-parses this; `include:`/`snippet:` resolution
    /// anchors at `base_dir`.
    Text { yaml: String, base_dir: PathBuf },
}

impl FlowSource {
    /// Log/display form — the file path or `inline`.
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::Path(path) => path.display().to_string(),
            Self::Text { .. } => "inline".to_string(),
        }
    }
}

/// What a drain is working toward — recorded on the entry so a
/// DELETE arriving mid-reload can still win (the reload reaper
/// re-checks before respawning).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrainOp {
    /// Remove the flow entirely once the supervisor exits.
    Delete,
    /// Respawn the flow from its source once the supervisor exits.
    Reload,
}

/// Lifecycle state reported by `GET /api/flows`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlowState {
    /// Supervising normally.
    Active,
    /// Draining toward `op` — routes already unregistered, shutdown
    /// flag set; the join completes in background.
    Draining(DrainOp),
}

impl FlowState {
    /// The wire spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Draining(_) => "draining",
        }
    }
}

/// One hosted flow — the registry entry `/api/flows` mutates.
struct FlowEntry {
    /// The registry key (also the demux route owner + queue/history
    /// file slug).
    name: String,
    /// Reload's input.
    source: FlowSource,
    state: FlowState,
    /// This flow's shutdown flag — distinct from the daemon-wide one
    /// so DELETE/reload drain one flow without touching the rest.
    shutdown: Arc<AtomicBool>,
    /// The supervisor join — `Mutex<Option>` so a drain reaper can
    /// `take` it while readers (GET, `Served::join`) still see the
    /// entry.
    join: Mutex<Option<SupervisorJoin>>,
    /// Commands into the supervisor loop ([`SupervisorCmd`]).
    control: Sender<SupervisorCmd>,
}

/// What [`FlowHost::spawn_inner`] produced — the pieces a
/// [`FlowEntry`] wraps.
struct Spawned {
    shutdown: Arc<AtomicBool>,
    injector: Injector,
    control: Sender<SupervisorCmd>,
    join: SupervisorJoin,
}

/// The flow registry + spawn context — shared by `serve_campaigns`
/// (boot), the `/api/flows` router, and `Served`'s shutdown/join.
pub struct FlowHost {
    /// `RunOptions.secrets` for every spawned run.
    pub secrets: BTreeMap<String, String>,
    /// Per-campaign durable history dir — `<dir>/<name>.jsonl`.
    pub history_dir: Option<PathBuf>,
    /// Per-campaign durable queue-store dir — `<dir>/<name>.db`.
    pub queue_store_dir: Option<PathBuf>,
    /// This node's runner labels — the `runs_on` gate, re-checked on
    /// every spawn.
    pub labels: Vec<String>,
    /// Per-finished-run callback (the CLI's printer).
    pub on_run: Option<RunCallback>,
    /// The shared webhook demux every flow's `github-webhook` nodes
    /// register into — always present: the listener binds
    /// unconditionally so API-added webhook flows receive
    /// deliveries.
    pub demux: Arc<SharedWebhook>,
    /// `/api/events/inject` fan-out targets — spawn pushes, DELETE
    /// removes.
    pub inject_targets: InjectTargets,
    /// Where POSTed yamls persist — `campaign serve`'s managed dir
    /// (the first directory path arg). `None` keeps API-added flows
    /// in-memory only ([`FlowSource::Text`]).
    pub managed_dir: Option<PathBuf>,
    /// The daemon-wide shutdown flag — `Served::request_shutdown`
    /// sets it (stops the listener) and every flow's own flag.
    /// Spawns after it refuse.
    pub shutdown: Arc<AtomicBool>,
    flows: RwLock<BTreeMap<String, FlowEntry>>,
}

/// A `GET /api/flows` row.
#[derive(Clone, Debug, serde::Serialize)]
pub struct FlowInfo {
    /// The hosted name.
    pub name: String,
    /// `active` | `draining`.
    pub state: &'static str,
    /// `delete` | `reload` while draining.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub op: Option<&'static str>,
    /// The yaml's origin — file path or `inline`.
    pub source: String,
    /// Repos this flow's webhook routes currently watch (empty for
    /// non-webhook flows and for routes not yet registered — the
    /// listener run registers asynchronously).
    pub repos: Vec<String>,
}

/// `DELETE /api/flows/{name}` outcome.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlowDelete {
    /// No flow by that name.
    NotFound,
    /// Already draining (delete or reload in flight) — the response
    /// is the same `202`; nothing new was started.
    AlreadyDraining,
    /// Unregistered + flagged; the join completes in background.
    Started,
}

/// `POST /api/flows/{name}/reload` outcome.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlowReload {
    /// No flow by that name.
    NotFound,
    /// A drain is already in flight.
    Busy,
    /// Drain-then-swap started; the respawn lands when the old
    /// supervisor's join completes.
    Started,
}

/// Flow-name rule — the name becomes a file slug (`<name>.yaml`,
/// `<name>.db`, `<name>.jsonl`) and a demux route owner, so it must
/// be path-safe: `[A-Za-z0-9._-]`, 1–128 chars, not `.`/`..`.
fn valid_flow_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && name != "."
        && name != ".."
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

/// A filename-safe campaign slug — `[A-Za-z0-9._-]` kept, everything
/// else `_`; empty becomes `campaign`. Names the per-campaign
/// history/queue-store files.
fn file_slug(name: &str) -> String {
    let slug: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') {
                c
            } else {
                '_'
            }
        })
        .collect();
    if slug.is_empty() {
        "campaign".to_string()
    } else {
        slug
    }
}

impl FlowHost {
    /// A host over the serve wiring — `flows` starts empty; the
    /// caller spawns boot campaigns through [`Self::spawn`].
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        secrets: BTreeMap<String, String>,
        history_dir: Option<PathBuf>,
        queue_store_dir: Option<PathBuf>,
        labels: Vec<String>,
        on_run: Option<RunCallback>,
        demux: Arc<SharedWebhook>,
        inject_targets: InjectTargets,
        managed_dir: Option<PathBuf>,
        shutdown: Arc<AtomicBool>,
    ) -> Self {
        Self {
            secrets,
            history_dir,
            queue_store_dir,
            labels,
            on_run,
            demux,
            inject_targets,
            managed_dir,
            shutdown,
            flows: RwLock::new(BTreeMap::new()),
        }
    }

    /// Every hosted flow's info row — `GET /api/flows`.
    #[must_use]
    pub fn list(&self) -> Vec<FlowInfo> {
        let flows = self.flows.read().unwrap_or_else(PoisonError::into_inner);
        flows
            .values()
            .map(|entry| FlowInfo {
                name: entry.name.clone(),
                state: entry.state.as_str(),
                op: match entry.state {
                    FlowState::Draining(DrainOp::Delete) => Some("delete"),
                    FlowState::Draining(DrainOp::Reload) => Some("reload"),
                    FlowState::Active => None,
                },
                source: entry.source.describe(),
                repos: self.demux.repos_for_owner(&entry.name),
            })
            .collect()
    }

    /// One flow's info row — `GET /api/flows/{name}`.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<FlowInfo> {
        self.list().into_iter().find(|info| info.name == name)
    }

    /// Whether `name` is hosted (any lifecycle state).
    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.flows
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .contains_key(name)
    }

    /// Spawn `hosted` as a live flow — the boot path and
    /// `POST /api/flows` share it. Returns the flow's trigger
    /// injector (boot collects them for `Served.injectors`; the API
    /// ignores it). Duplicate names, post-shutdown spawns, and
    /// `runs_on`-ineligible campaigns are rejected.
    ///
    /// # Errors
    /// [`CampaignError::Serve`] on shutdown/duplicate/labels;
    /// [`CampaignError::QueueStore`] when the per-flow store can't
    /// open.
    pub fn spawn(self: &Arc<Self>, hosted: HostedCampaign) -> Result<Injector, CampaignError> {
        if self.shutdown.load(Ordering::Relaxed) {
            return Err(CampaignError::Serve("daemon is shutting down".to_string()));
        }
        if !hosted.campaign.runs_on_matches(&self.labels) {
            return Err(CampaignError::Serve(format!(
                "campaign '{}' requires runner labels this node doesn't advertise",
                hosted.name
            )));
        }
        let mut flows = self.flows.write().unwrap_or_else(PoisonError::into_inner);
        if flows.contains_key(&hosted.name) {
            return Err(CampaignError::Serve(format!(
                "duplicate campaign name '{}'",
                hosted.name
            )));
        }
        let name = hosted.name.clone();
        let spawned = self.spawn_inner(&name, &hosted.campaign)?;
        // `/api/events/inject` fan-out — registered before the entry
        // so a request can't see the flow without its injector.
        self.inject_targets
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .push((name.clone(), spawned.injector.clone()));
        flows.insert(
            name.clone(),
            FlowEntry {
                name,
                source: hosted.source,
                state: FlowState::Active,
                shutdown: spawned.shutdown,
                join: Mutex::new(Some(spawned.join)),
                control: spawned.control,
            },
        );
        Ok(spawned.injector)
    }

    /// The supervise half of [`Self::spawn`] — queue store, options,
    /// `supervise`. No registry interaction; callers hold the flows
    /// lock so spawn-vs-delete can't interleave.
    fn spawn_inner(&self, name: &str, campaign: &Campaign) -> Result<Spawned, CampaignError> {
        let queue_store = match &self.queue_store_dir {
            Some(dir) => {
                let path = dir.join(format!("{}.db", file_slug(name)));
                Some(
                    Arc::new(peregrin_queue_sqlite::SqliteQueueStore::open(&path)?)
                        as Arc<dyn peregrin_queue::QueueStore>,
                )
            }
            None => None,
        };
        let on_run: Option<supervisor::OnRunDone> = self.on_run.as_ref().map(|callback| {
            let name = name.to_string();
            let callback = Arc::clone(callback);
            let wrapped: supervisor::OnRunDone =
                Box::new(move |log: &[Event]| callback(&name, log));
            wrapped
        });
        let shutdown = Arc::new(AtomicBool::new(false));
        let options = supervisor::SupervisedOptions {
            secrets: self.secrets.clone(),
            history: self
                .history_dir
                .as_ref()
                .map(|dir| dir.join(format!("{}.jsonl", file_slug(name)))),
            queue_store,
            shared_webhook: Some(Arc::clone(&self.demux)),
            shutdown: Some(Arc::clone(&shutdown)),
            on_run,
            campaign_name: Some(name.to_string()),
            ..supervisor::SupervisedOptions::default()
        };
        let run = supervisor::supervise(campaign, options);
        Ok(Spawned {
            shutdown,
            injector: run.injector,
            control: run.control,
            join: run.join,
        })
    }

    /// Load a flow's persisted source — reload's input.
    ///
    /// # Errors
    /// [`CampaignError::Load`] on read/parse/expand failure.
    fn load_source(name: &str, source: &FlowSource) -> Result<Campaign, CampaignError> {
        let result = match source {
            FlowSource::Path(path) => campaign::load(path),
            FlowSource::Text { yaml, base_dir } => YamlLoader
                .load_str(yaml, base_dir)
                .map_err(CampaignError::from),
        };
        result.map_err(|e| {
            tracing::warn!(flow = name, "flow reload source load failed: {e}");
            e
        })
    }

    /// `DELETE /api/flows/{name}` — stop new deliveries (route
    /// unregister), drop queued triggers, flag the flow's own
    /// shutdown; the active run finishes and a background reaper
    /// drops the registry entry when the join lands.
    pub fn delete(self: &Arc<Self>, name: &str) -> FlowDelete {
        let mut flows = self.flows.write().unwrap_or_else(PoisonError::into_inner);
        let Some(entry) = flows.get_mut(name) else {
            return FlowDelete::NotFound;
        };
        if entry.state != FlowState::Active {
            return FlowDelete::AlreadyDraining;
        }
        entry.state = FlowState::Draining(DrainOp::Delete);
        // Deliveries stop first — a trigger landing between the flag
        // and the drain would queue work the delete just cleared.
        self.demux.unregister(name);
        let _ = entry.control.send(SupervisorCmd::ClearPending);
        entry.shutdown.store(true, Ordering::Relaxed);
        self.inject_targets
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .retain(|(n, _)| n != name);
        let join = entry
            .join
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        let host = Arc::clone(self);
        let name = name.to_string();
        thread::spawn(move || {
            if let Some(join) = join {
                let _ = join.join();
            }
            host.flows
                .write()
                .unwrap_or_else(PoisonError::into_inner)
                .remove(&name);
        });
        FlowDelete::Started
    }

    /// `POST /api/flows/{name}/reload` — drain-then-swap: unregister
    /// routes (deliveries 404 for the drain window — the documented
    /// gap), flag the flow's shutdown *without* clearing pending
    /// (queued triggers still run), then a reaper respawns from the
    /// persisted source once the join lands.
    pub fn reload(self: &Arc<Self>, name: &str) -> FlowReload {
        let mut flows = self.flows.write().unwrap_or_else(PoisonError::into_inner);
        let Some(entry) = flows.get_mut(name) else {
            return FlowReload::NotFound;
        };
        if entry.state != FlowState::Active {
            return FlowReload::Busy;
        }
        entry.state = FlowState::Draining(DrainOp::Reload);
        self.demux.unregister(name);
        entry.shutdown.store(true, Ordering::Relaxed);
        let join = entry
            .join
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        let source = entry.source.clone();
        let host = Arc::clone(self);
        let name = name.to_string();
        thread::spawn(move || {
            if let Some(join) = join {
                let _ = join.join();
            }
            // Load outside the lock — a slow include chain mustn't
            // stall GET/DELETE.
            let Ok(campaign) = FlowHost::load_source(&name, &source) else {
                host.flows
                    .write()
                    .unwrap_or_else(PoisonError::into_inner)
                    .remove(&name);
                return;
            };
            let mut flows = host.flows.write().unwrap_or_else(PoisonError::into_inner);
            // A DELETE during the drain converts the op — it wins:
            // the flow is removed, never respawned.
            if flows.get(&name).map(|e| e.state) != Some(FlowState::Draining(DrainOp::Reload)) {
                flows.remove(&name);
                return;
            }
            match host.spawn_inner(&name, &campaign) {
                Ok(spawned) => {
                    host.inject_targets
                        .write()
                        .unwrap_or_else(PoisonError::into_inner)
                        .push((name.clone(), spawned.injector.clone()));
                    flows.insert(
                        name.clone(),
                        FlowEntry {
                            name,
                            source,
                            state: FlowState::Active,
                            shutdown: spawned.shutdown,
                            join: Mutex::new(Some(spawned.join)),
                            control: spawned.control,
                        },
                    );
                }
                Err(e) => {
                    tracing::error!(flow = %name, "flow reload respawn failed: {e}");
                    flows.remove(&name);
                }
            }
        });
        FlowReload::Started
    }

    /// Set every flow's shutdown flag — `Served::request_shutdown`'s
    /// per-flow half (the daemon-wide flag stops the listener; this
    /// drains the supervisors).
    pub fn shutdown_all(&self) {
        let flows = self.flows.read().unwrap_or_else(PoisonError::into_inner);
        for entry in flows.values() {
            entry.shutdown.store(true, Ordering::Relaxed);
        }
    }

    /// Join every flow's supervisor — `Served::join`'s drain loop.
    /// Entries whose join a delete/reload reaper already took are
    /// skipped (the reaper removes them); entries still holding a
    /// join are joined here and removed. Loops until the registry
    /// empties — reaper-held joins resolve on their own threads.
    ///
    /// Returns the first supervisor error after all loops finish.
    pub fn join_all(&self) -> Result<(), CampaignError> {
        let mut first_error = None;
        loop {
            let pending: Vec<(String, SupervisorJoin)> = {
                let flows = self.flows.read().unwrap_or_else(PoisonError::into_inner);
                flows
                    .values()
                    .filter_map(|entry| {
                        entry
                            .join
                            .lock()
                            .unwrap_or_else(PoisonError::into_inner)
                            .take()
                            .map(|join| (entry.name.clone(), join))
                    })
                    .collect()
            };
            if pending.is_empty() {
                if self
                    .flows
                    .read()
                    .unwrap_or_else(PoisonError::into_inner)
                    .is_empty()
                {
                    return match first_error {
                        Some(e) => Err(e),
                        None => Ok(()),
                    };
                }
                // Remaining entries' joins are held by drain reapers —
                // wait for them to finish and remove the entries.
                thread::sleep(Duration::from_millis(50));
                continue;
            }
            for (name, handle) in pending {
                match handle.join() {
                    Ok(Ok(_)) => {}
                    Ok(Err(e)) => {
                        tracing::error!("campaign '{name}' supervisor failed: {e}");
                        if first_error.is_none() {
                            first_error = Some(e);
                        }
                    }
                    Err(_) => {
                        let e = CampaignError::Run(peregrin_driver::DriverError::new(format!(
                            "campaign '{name}' supervisor thread panicked"
                        )));
                        if first_error.is_none() {
                            first_error = Some(e);
                        }
                    }
                }
                self.flows
                    .write()
                    .unwrap_or_else(PoisonError::into_inner)
                    .remove(&name);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// The axum surface.
// ---------------------------------------------------------------------------

/// Request-body cap — campaign yamls with includes run to tens of
/// KiB; 1 MiB is generous headroom without opening a memory sink.
const MAX_BODY: usize = 1024 * 1024;

/// The router state — the flow host plus the expected bearer
/// (`None` = open, the tokenless-dev convention).
struct ApiState {
    host: Arc<FlowHost>,
    token: Option<String>,
}

/// Bearer gate — shared with `/api/events/inject`'s check.
fn authorized(state: &ApiState, headers: &HeaderMap) -> bool {
    crate::inject::token_ok(state.token.as_deref(), headers)
}

fn json_response(body: Value, status: StatusCode) -> Response {
    (status, Json(body)).into_response()
}

/// `GET|POST /api/flows`.
async fn flows_collection(
    State(state): State<Arc<ApiState>>,
    method: Method,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if !authorized(&state, &headers) {
        return json_response(json!({ "error": "unauthorized" }), StatusCode::UNAUTHORIZED);
    }
    match method {
        Method::GET => {
            let flows = state.host.list();
            json_response(json!({ "flows": flows }), StatusCode::OK)
        }
        Method::POST => create_flow(&state, &body),
        _ => json_response(json!({ "error": "not found" }), StatusCode::NOT_FOUND),
    }
}

/// `POST /api/flows` — validate, optionally persist to the managed
/// dir, load, spawn. The file write precedes the load so a boot
/// replay sees the same doc the API validated; a load failure
/// removes it again.
fn create_flow(state: &ApiState, body: &[u8]) -> Response {
    if body.len() > MAX_BODY {
        return json_response(
            json!({ "error": "request body too large" }),
            StatusCode::PAYLOAD_TOO_LARGE,
        );
    }
    let parsed: Result<Value, _> = serde_json::from_slice(body);
    let Ok(value) = parsed else {
        return json_response(
            json!({ "error": "Expected a JSON object body" }),
            StatusCode::BAD_REQUEST,
        );
    };
    let Some(obj) = value.as_object() else {
        return json_response(
            json!({ "error": "Expected a JSON object body" }),
            StatusCode::BAD_REQUEST,
        );
    };
    let name = obj.get("name").and_then(Value::as_str).unwrap_or("");
    if !valid_flow_name(name) {
        return json_response(
            json!({ "error": "'name' must be [A-Za-z0-9._-], 1-128 chars" }),
            StatusCode::BAD_REQUEST,
        );
    }
    let yaml = obj.get("yaml").and_then(Value::as_str).unwrap_or("");
    if yaml.trim().is_empty() {
        return json_response(
            json!({ "error": "missing 'yaml' — the campaign document" }),
            StatusCode::BAD_REQUEST,
        );
    }
    if state.host.contains(name) {
        return json_response(
            json!({ "error": format!("flow '{name}' already hosted") }),
            StatusCode::CONFLICT,
        );
    }
    // Persist first (managed dir) — reload and next-boot replay both
    // read the file, so it must hold exactly the posted doc.
    let (source, persisted) = if let Some(dir) = &state.host.managed_dir {
        let path = dir.join(format!("{name}.yaml"));
        if let Err(e) = std::fs::create_dir_all(dir).and_then(|()| std::fs::write(&path, yaml)) {
            return json_response(
                json!({ "error": format!("persist {}: {e}", path.display()) }),
                StatusCode::INTERNAL_SERVER_ERROR,
            );
        }
        (FlowSource::Path(path.clone()), Some(path))
    } else {
        let base_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        (
            FlowSource::Text {
                yaml: yaml.to_string(),
                base_dir,
            },
            None,
        )
    };
    let campaign = match &source {
        FlowSource::Path(path) => campaign::load(path),
        FlowSource::Text { yaml, base_dir } => YamlLoader
            .load_str(yaml, base_dir)
            .map_err(CampaignError::from),
    };
    let campaign = match campaign {
        Ok(campaign) => campaign,
        Err(e) => {
            // A doc that can't even load must not stay for the next
            // boot — `campaign serve` fails fast on unloadable yamls.
            if let Some(path) = &persisted {
                let _ = std::fs::remove_file(path);
            }
            return json_response(json!({ "error": e.to_string() }), StatusCode::BAD_REQUEST);
        }
    };
    if !campaign.runs_on_matches(&state.host.labels) {
        // Valid doc, wrong node — leave the file; an eligible node
        // picks it up on boot.
        return json_response(
            json!({ "error": "campaign requires runner labels this node doesn't advertise" }),
            StatusCode::UNPROCESSABLE_ENTITY,
        );
    }
    if state.host.shutdown.load(Ordering::Relaxed) {
        return json_response(
            json!({ "error": "daemon is shutting down" }),
            StatusCode::SERVICE_UNAVAILABLE,
        );
    }
    // `spawn` re-checks duplicate/labels — the pre-checks above give
    // the precise status codes; a leftover error is a race or an
    // internal failure.
    let host = Arc::clone(&state.host);
    match host.spawn(HostedCampaign {
        name: name.to_string(),
        campaign,
        source,
    }) {
        Ok(_) => json_response(
            json!({ "name": name, "state": "active" }),
            StatusCode::CREATED,
        ),
        Err(e) => {
            let msg = e.to_string();
            let status = if msg.contains("duplicate campaign name") {
                StatusCode::CONFLICT
            } else if msg.contains("runner labels") {
                StatusCode::UNPROCESSABLE_ENTITY
            } else if msg.contains("shutting down") {
                StatusCode::SERVICE_UNAVAILABLE
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            json_response(json!({ "error": msg }), status)
        }
    }
}

/// `GET|DELETE /api/flows/{name}`.
async fn flow_item(
    State(state): State<Arc<ApiState>>,
    method: Method,
    headers: HeaderMap,
    AxumPath(name): AxumPath<String>,
) -> Response {
    if !authorized(&state, &headers) {
        return json_response(json!({ "error": "unauthorized" }), StatusCode::UNAUTHORIZED);
    }
    match method {
        Method::GET => match state.host.get(&name) {
            Some(info) => json_response(
                serde_json::to_value(info).unwrap_or(Value::Null),
                StatusCode::OK,
            ),
            None => json_response(
                json!({ "error": format!("flow '{name}' not found") }),
                StatusCode::NOT_FOUND,
            ),
        },
        Method::DELETE => {
            let host = Arc::clone(&state.host);
            let draining = matches!(
                host.delete(&name),
                FlowDelete::Started | FlowDelete::AlreadyDraining
            );
            if draining {
                json_response(
                    json!({ "name": name, "draining": true, "op": "delete" }),
                    StatusCode::ACCEPTED,
                )
            } else {
                json_response(
                    json!({ "error": format!("flow '{name}' not found") }),
                    StatusCode::NOT_FOUND,
                )
            }
        }
        _ => json_response(json!({ "error": "not found" }), StatusCode::NOT_FOUND),
    }
}

/// `POST /api/flows/{name}/reload`.
async fn flow_reload(
    State(state): State<Arc<ApiState>>,
    method: Method,
    headers: HeaderMap,
    AxumPath(name): AxumPath<String>,
) -> Response {
    if !authorized(&state, &headers) {
        return json_response(json!({ "error": "unauthorized" }), StatusCode::UNAUTHORIZED);
    }
    if method != Method::POST {
        return json_response(json!({ "error": "not found" }), StatusCode::NOT_FOUND);
    }
    let host = Arc::clone(&state.host);
    match host.reload(&name) {
        FlowReload::NotFound => json_response(
            json!({ "error": format!("flow '{name}' not found") }),
            StatusCode::NOT_FOUND,
        ),
        FlowReload::Busy => json_response(
            json!({ "error": format!("flow '{name}' is already draining") }),
            StatusCode::CONFLICT,
        ),
        FlowReload::Started => json_response(
            json!({ "name": name, "reloading": true }),
            StatusCode::ACCEPTED,
        ),
    }
}

/// The `/api/flows*` routes as an axum [`Router`] — `.merge()` onto
/// the shared webhook app so flows lifecycle rides the one listener
/// socket. `token` is the expected bearer (`None`/empty = open).
pub fn router(host: Arc<FlowHost>, token: Option<String>) -> Router {
    let state = Arc::new(ApiState { host, token });
    Router::new()
        .route("/api/flows", any(flows_collection))
        .route("/api/flows/{name}", any(flow_item))
        .route("/api/flows/{name}/reload", any(flow_reload))
        .layer(axum::extract::DefaultBodyLimit::max(MAX_BODY))
        .with_state(state)
}
