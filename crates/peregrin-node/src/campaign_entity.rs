//! The campaign entity plane — registers `kind="campaign"` on the
//! node's entity surface (P-001, ADR-0091, PL-004 phase 6).
//!
//! A campaign entity models a *run* (`{id, kind: "campaign", indexDoc}`),
//! created via `POST /api/entities`, driven via `campaign.*` ops
//! (`campaign.start`, `campaign.pause`, `campaign.resume`,
//! `campaign.cancel`, `campaign.signal`), observable in the merged
//! index, and movable across nodes via snapshot export
//! (`campaign-snapshot/v1`) → import.
//!
//! Pause/resume are *checkpoint* semantics, not thread suspension: the
//! engine's runner is synchronous, so `campaign.pause` injects the
//! same `workflow.cancel` a cancel would and parks the entity as
//! `paused` with its event log + pending timers preserved;
//! `campaign.resume` re-runs the campaign with `replayEvents`/
//! `replayTimers` — the P-001 snapshot seam. Nodes that already
//! emitted do not re-fire (`max_fires` accounting survives `seed`),
//! but a node mid-flight when the pause landed re-executes on resume
//! — at-least-once, and driver side effects are not rolled back.
//!
//! Snapshot export of a *running* entity streams the live event log
//! (the run's bus observer feeds it) and reads armed timers through
//! the runner's live mirror (`Runner::with_timer_mirror`) — the same
//! `PendingTimer` snapshot `pending_timers()` returns at finish, so a
//! mid-run snapshot carries them too. Terminal `workflow.*` markers
//! are stripped from
//! replayed logs so a resumed run's history stays honest, and an
//! imported non-terminal log lands as `paused` (a resumable
//! checkpoint), never `running` — no runner thread exists here.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::sync::{Arc, Mutex, RwLock};

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use hypha_http::{EntityContext, EntityPlane};
use peregrin_bus::{LocalBus, Observer};
use peregrin_core::{Campaign, Event};
use peregrin_engine::{Injector, PendingTimer, RunOptions, Runner};
use peregrin_loader::Loader;
use peregrin_loader_yaml::YamlLoader;
use peregrin_store::MemoryStore;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub const CAMPAIGN_KIND: &str = "campaign";

/// `campaign-snapshot/v1` — the export/import payload `format` tag.
pub const SNAPSHOT_FORMAT: &str = "campaign-snapshot/v1";

/// Terminal `workflow.*` markers a previous leg left in the log.
/// They must never replay: a stale `workflow.cancelled` would poison
/// the resumed run's log and mislabel every later export. The pause
/// checkpoint's `workflow.cancel` trigger is consumed by the runner
/// before publish, so the only terminal artifact is the runner's own
/// end event.
fn replayable_log(log: &[Event]) -> Vec<Event> {
    log.iter()
        .filter(|e| {
            !matches!(
                e.kind.as_str(),
                "workflow.done" | "workflow.cancelled" | "workflow.failed" | "workflow.cancel"
            )
        })
        .cloned()
        .collect()
}

/// An in-memory campaign run entity tracked by this plane.
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CampaignEntity {
    pub id: String,
    pub name: String,
    /// `queued` → `running` → `done`|`failed`|`cancelled`, with
    /// `paused` as the checkpoint park between `queued`/`running` and
    /// `campaign.resume`.
    pub state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub campaign: Option<Campaign>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub campaign_ref: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<String>,
    pub events_count: usize,
    pub fires_count: usize,
    pub failed_count: usize,
    /// How many checkpoint resumes this entity has lived through.
    pub resume_count: usize,
    pub log: Vec<Event>,
    pub pending_timers: Vec<PendingTimer>,
    #[serde(skip)]
    pub injector: Option<Injector>,
    /// Set by `campaign.pause` while `running` — the finish handler
    /// parks the entity as `paused` instead of `cancelled` when the
    /// injected cancel is what ended the run.
    #[serde(skip)]
    pub pause_requested: bool,
    /// Mid-run event stream fed by the run's bus observer — keeps
    /// `list`/`get`/`export` honest while the runner thread owns the
    /// authoritative log. `None` before start and after finish.
    #[serde(skip)]
    pub live_log: Option<Arc<Mutex<Vec<Event>>>>,
    /// Mid-run armed-timer mirror — the runner refreshes it after
    /// every heap mutation (`Runner::with_timer_mirror`), so
    /// `export`/`list` see pending timers before the run finishes.
    /// `None` before start and after finish.
    #[serde(skip)]
    pub live_timers: Option<Arc<Mutex<Vec<PendingTimer>>>>,
    pub created_at: String,
    pub updated_at: String,
}

/// The resident `kind="campaign"` plane impl.
#[derive(Clone, Default)]
pub struct CampaignEntityPlane {
    entities: Arc<RwLock<HashMap<String, CampaignEntity>>>,
    /// This node's advertised capability labels — the `runs_on`
    /// matcher's right-hand side (PL-007 C4 fleet scheduling). Fed
    /// from the same list `GET /api/node` reports, so a remote
    /// `campaign.start` lands only on nodes that declared they can
    /// run it.
    labels: Arc<Vec<String>>,
}

impl CampaignEntityPlane {
    #[must_use]
    pub fn new() -> Self {
        Self {
            entities: Arc::new(RwLock::new(HashMap::new())),
            labels: Arc::new(Vec::new()),
        }
    }

    /// Advertise this node's capability labels for `runs_on` matching.
    #[must_use]
    pub fn with_labels(mut self, labels: Vec<String>) -> Self {
        self.labels = Arc::new(labels);
        self
    }

    /// Access the underlying entities map (for diagnostics/tests).
    #[must_use]
    pub fn entities(&self) -> Arc<RwLock<HashMap<String, CampaignEntity>>> {
        Arc::clone(&self.entities)
    }
}

fn json_resp(status: StatusCode, val: Value) -> Response {
    (status, axum::Json(val)).into_response()
}

fn lock_err() -> Response {
    json_resp(
        StatusCode::INTERNAL_SERVER_ERROR,
        json!({ "error": "lock error" }),
    )
}

fn not_found(id: &str) -> Response {
    json_resp(
        StatusCode::NOT_FOUND,
        json!({ "error": format!("campaign entity '{id}' not found") }),
    )
}

/// Mid-run counts read the live stream; finished runs read the
/// counters the runner thread wrote back. Returns
/// `(events, fires, failed, lastEventKind)`.
fn live_counts(entity: &CampaignEntity) -> (usize, usize, usize, Option<String>) {
    let fallback = || {
        (
            entity.events_count,
            entity.fires_count,
            entity.failed_count,
            None,
        )
    };
    let Some(live) = &entity.live_log else {
        return fallback();
    };
    let Ok(log) = live.lock() else {
        return fallback();
    };
    let events = log.len();
    let fires = log.iter().filter(|e| e.source != "__runner__").count();
    let failed = log.iter().filter(|e| e.kind == "node.failed").count();
    let last = log.last().map(|e| e.kind.clone());
    (events, fires, failed, last)
}

/// Armed timers visible right now: a running entity reads the
/// runner's live mirror (refreshed on every heap mutation); any other
/// state reads the `pending_timers` snapshot written back at finish
/// or carried in by `import`.
fn visible_timers(entity: &CampaignEntity) -> Vec<PendingTimer> {
    if entity.state == "running" {
        if let Some(slot) = &entity.live_timers {
            if let Ok(timers) = slot.lock() {
                return timers.clone();
            }
        }
    }
    entity.pending_timers.clone()
}

fn entity_doc(entity: &CampaignEntity) -> Value {
    let running = entity.state == "running";
    let (events, fires, failed, last_event) = if running {
        live_counts(entity)
    } else {
        (
            entity.events_count,
            entity.fires_count,
            entity.failed_count,
            None,
        )
    };
    json!({
        "id": entity.id,
        "kind": CAMPAIGN_KIND,
        "indexDoc": {
            "name": entity.name,
            "state": entity.state,
            "campaignName": entity.campaign.as_ref().map(|c| c.name.as_str()),
            "campaignRef": entity.campaign_ref,
            "startedAt": entity.started_at,
            "finishedAt": entity.finished_at,
            "counts": {
                "events": events,
                "fires": fires,
                "failed": failed,
                "resumes": entity.resume_count,
            },
            "pendingTimers": visible_timers(entity).len(),
            "lastEvent": last_event,
        },
        "live": running,
        "locked": running || entity.state == "paused",
        "updatedAt": entity.updated_at,
    })
}

fn parse_campaign(payload: &Value) -> Result<Campaign, String> {
    let campaign_val = payload.get("campaign").unwrap_or(payload);
    if let Some(yaml) = campaign_val.get("yaml").and_then(Value::as_str) {
        YamlLoader
            .load_str(yaml, Path::new("."))
            .map_err(|e| e.to_string())
    } else if let Some(path) = campaign_val.get("path").and_then(Value::as_str) {
        YamlLoader.load(Path::new(path)).map_err(|e| e.to_string())
    } else if let Some(yaml_str) = campaign_val.as_str() {
        if yaml_str.contains('\n') || yaml_str.starts_with("name:") {
            YamlLoader
                .load_str(yaml_str, Path::new("."))
                .map_err(|e| e.to_string())
        } else {
            YamlLoader
                .load(Path::new(yaml_str))
                .map_err(|e| e.to_string())
        }
    } else {
        serde_json::from_value::<Campaign>(campaign_val.clone()).map_err(|e| e.to_string())
    }
}

/// The campaign a start/resume payload asks for, with `params.config`
/// merged in — `None` when the payload carries no campaign at all
/// (resume falls back to the entity's stored campaign).
fn campaign_from_payload(payload: &Value) -> Result<Option<Campaign>, String> {
    // An object payload with no campaign keys and no top-level `nodes`
    // carries no campaign at all (`{}`, `{"params":…}`, `{"dryRun":…}`)
    // — resume falls back to the stored campaign. Anything else —
    // `{"campaign":…}`, a bare campaign doc, or a non-object payload
    // (yaml/path string) — parses as a campaign.
    let carries_campaign = !payload.is_object()
        || payload.get("campaign").is_some()
        || payload.get("yaml").is_some()
        || payload.get("path").is_some()
        || payload.get("nodes").is_some_and(Value::is_array);
    if !carries_campaign {
        return Ok(None);
    }
    let mut campaign = parse_campaign(payload)?;
    if let Some(config) = payload
        .get("params")
        .and_then(|p| p.get("config"))
        .and_then(Value::as_object)
    {
        for (k, v) in config {
            campaign.config.insert(k.clone(), v.clone());
        }
    }
    Ok(Some(campaign))
}

fn dry_run(payload: &Value) -> bool {
    payload
        .get("dryRun")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

/// Feeds the entity's live event log — the run's bus reports every
/// published event here so `list`/`get`/`export` project mid-run
/// state. `seed` bypasses `publish`, so replayed history never
/// double-counts.
struct LiveObserver {
    log: Arc<Mutex<Vec<Event>>>,
}

impl Observer for LiveObserver {
    fn on_event(&self, event: &Event) {
        if let Ok(mut log) = self.log.lock() {
            log.push(event.clone());
        }
    }
}

#[async_trait::async_trait]
impl EntityPlane for CampaignEntityPlane {
    fn kind(&self) -> &'static str {
        CAMPAIGN_KIND
    }

    fn op_namespace(&self) -> &'static str {
        CAMPAIGN_KIND
    }

    async fn list(&self, _state: &EntityContext, _query: Option<&str>) -> Vec<Value> {
        let Ok(map) = self.entities.read() else {
            return Vec::new();
        };
        map.values().map(entity_doc).collect()
    }

    async fn get(&self, _state: &EntityContext, id: &str, _query: Option<&str>) -> Response {
        let Ok(map) = self.entities.read() else {
            return lock_err();
        };
        map.get(id).map_or_else(
            || not_found(id),
            |entity| json_resp(StatusCode::OK, entity_doc(entity)),
        )
    }

    async fn create(&self, _state: &EntityContext, init: Value) -> Response {
        let id = init.get("id").and_then(Value::as_str).map_or_else(
            || format!("run-{}", uuid::Uuid::new_v4().simple()),
            str::to_string,
        );
        let name = init
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("campaign")
            .to_string();
        let campaign_ref = init.get("campaignRef").cloned();
        let now = chrono::Utc::now().to_rfc3339();

        let entity = CampaignEntity {
            id: id.clone(),
            name,
            state: "queued".to_string(),
            campaign: None,
            campaign_ref,
            started_at: None,
            finished_at: None,
            events_count: 0,
            fires_count: 0,
            failed_count: 0,
            resume_count: 0,
            log: Vec::new(),
            pending_timers: Vec::new(),
            injector: None,
            pause_requested: false,
            live_log: None,
            live_timers: None,
            created_at: now.clone(),
            updated_at: now,
        };

        let Ok(mut map) = self.entities.write() else {
            return lock_err();
        };
        let doc = entity_doc(&entity);
        map.insert(id, entity);
        json_resp(StatusCode::CREATED, doc)
    }

    async fn delete(&self, _state: &EntityContext, id: &str, query: Option<&str>) -> Response {
        let force = query.is_some_and(|q| q.contains("force=true") || q.contains("force=1"));
        let Ok(mut map) = self.entities.write() else {
            return lock_err();
        };
        let Some(entity) = map.get(id) else {
            return not_found(id);
        };

        if entity.state == "running" && !force {
            return json_resp(
                StatusCode::CONFLICT,
                json!({ "error": "campaign is running; use ?force=true to delete" }),
            );
        }

        if let Some(injector) = &entity.injector {
            let _ = injector.inject(Event::cancel("deleted via API"));
        }
        map.remove(id);
        json_resp(StatusCode::OK, json!({ "ok": true }))
    }

    async fn op(
        &self,
        _state: &EntityContext,
        id: &str,
        op: &str,
        payload: Value,
        _query: Option<&str>,
    ) -> Response {
        let verb = op.strip_prefix("campaign.").unwrap_or(op);
        match verb {
            "start" => self.handle_start(id, &payload),
            "pause" => self.handle_pause(id),
            "resume" => self.handle_resume(id, &payload),
            "cancel" => self.handle_cancel(id, &payload),
            "signal" => self.handle_signal(id, &payload),
            _ => json_resp(
                StatusCode::NOT_IMPLEMENTED,
                json!({ "error": format!("unknown campaign op '{op}'") }),
            ),
        }
    }

    async fn export(&self, _state: &EntityContext, id: &str, _query: Option<&str>) -> Response {
        let Ok(map) = self.entities.read() else {
            return lock_err();
        };
        let Some(entity) = map.get(id) else {
            return not_found(id);
        };

        // A running entity's authoritative log lives on the runner
        // thread; the live stream is the freshest honest copy, and
        // the timer mirror carries armed timers mid-run — the same
        // snapshot `pending_timers()` returns at finish.
        let log = if entity.state == "running" {
            entity.live_log.as_ref().map_or_else(
                || entity.log.clone(),
                |live| live.lock().map(|l| l.clone()).unwrap_or_default(),
            )
        } else {
            entity.log.clone()
        };

        let snapshot = json!({
            "format": SNAPSHOT_FORMAT,
            "kind": CAMPAIGN_KIND,
            "entity": {
                "id": entity.id,
                "kind": CAMPAIGN_KIND,
            },
            "state": entity.state,
            "live": entity.state == "running",
            "campaign": entity.campaign,
            "campaignRef": entity.campaign_ref,
            "log": log,
            "pendingTimers": visible_timers(entity),
            "counters": {
                "events": entity.events_count,
                "fires": entity.fires_count,
                "failed": entity.failed_count,
                "resumes": entity.resume_count,
            }
        });
        json_resp(StatusCode::OK, snapshot)
    }

    async fn import(&self, _state: &EntityContext, body: Value) -> Response {
        if body.get("format").and_then(Value::as_str) != Some(SNAPSHOT_FORMAT) {
            return json_resp(
                StatusCode::BAD_REQUEST,
                json!({ "error": format!("expected format '{SNAPSHOT_FORMAT}'") }),
            );
        }

        let id = body
            .get("entity")
            .and_then(|e| e.get("id"))
            .and_then(Value::as_str)
            .map_or_else(
                || format!("run-{}", uuid::Uuid::new_v4().simple()),
                str::to_string,
            );
        let campaign: Option<Campaign> = body
            .get("campaign")
            .and_then(|c| serde_json::from_value(c.clone()).ok());
        let campaign_ref = body.get("campaignRef").cloned();
        let log: Vec<Event> = body
            .get("log")
            .and_then(|l| serde_json::from_value(l.clone()).ok())
            .unwrap_or_default();
        let pending_timers: Vec<PendingTimer> = body
            .get("pendingTimers")
            .and_then(|t| serde_json::from_value(t.clone()).ok())
            .unwrap_or_default();
        let resume_count = body
            .get("counters")
            .and_then(|c| c.get("resumes"))
            .and_then(Value::as_u64)
            .unwrap_or(0) as usize;

        // A non-terminal log is a checkpoint — the imported entity is
        // `paused` (resumable via `campaign.resume`), never `running`:
        // no runner thread exists on this side of the wire.
        let state = if log.iter().any(|e| e.kind == "workflow.cancelled") {
            "cancelled"
        } else if log.iter().any(|e| e.kind == "workflow.failed") {
            "failed"
        } else if log.iter().any(|e| e.kind == "workflow.done") {
            "done"
        } else if !log.is_empty() {
            "paused"
        } else {
            "queued"
        };

        let events_count = log.len();
        let fires_count = log.iter().filter(|e| e.source != "__runner__").count();
        let failed_count = log.iter().filter(|e| e.kind == "node.failed").count();
        let now = chrono::Utc::now().to_rfc3339();

        let entity = CampaignEntity {
            id: id.clone(),
            name: campaign
                .as_ref()
                .map_or("campaign", |c| &c.name)
                .to_string(),
            state: state.to_string(),
            campaign,
            campaign_ref,
            started_at: None,
            finished_at: if matches!(state, "done" | "failed" | "cancelled") {
                Some(now.clone())
            } else {
                None
            },
            events_count,
            fires_count,
            failed_count,
            resume_count,
            log,
            pending_timers,
            injector: None,
            pause_requested: false,
            live_log: None,
            live_timers: None,
            created_at: now.clone(),
            updated_at: now,
        };

        let Ok(mut map) = self.entities.write() else {
            return lock_err();
        };
        let doc = entity_doc(&entity);
        map.insert(id, entity);
        json_resp(StatusCode::CREATED, doc)
    }
}

impl CampaignEntityPlane {
    /// `campaign.start` — requires a campaign in the payload; legal
    /// from `queued` (fresh leg) and `paused` (checkpoint resume with
    /// an optional new campaign definition).
    fn handle_start(&self, id: &str, payload: &Value) -> Response {
        let campaign = match campaign_from_payload(payload) {
            Ok(Some(c)) => c,
            Ok(None) => {
                return json_resp(
                    StatusCode::BAD_REQUEST,
                    json!({ "error": "campaign.start requires a campaign (yaml/path/json)" }),
                );
            }
            Err(e) => {
                return json_resp(
                    StatusCode::BAD_REQUEST,
                    json!({ "error": format!("failed to load campaign: {e}") }),
                );
            }
        };
        self.launch(id, campaign, dry_run(payload))
    }

    /// `campaign.resume` — continues a `paused` checkpoint (or starts
    /// a `queued` entity) from the stored campaign; a campaign in the
    /// payload overrides it.
    fn handle_resume(&self, id: &str, payload: &Value) -> Response {
        let (state, stored) = {
            let Ok(map) = self.entities.read() else {
                return lock_err();
            };
            let Some(entity) = map.get(id) else {
                return not_found(id);
            };
            (entity.state.clone(), entity.campaign.clone())
        };
        if !matches!(state.as_str(), "queued" | "paused") {
            return json_resp(
                StatusCode::CONFLICT,
                json!({ "error": format!("cannot resume campaign in state '{state}'") }),
            );
        }
        let campaign = match campaign_from_payload(payload) {
            Ok(Some(c)) => c,
            Ok(None) => match stored {
                Some(c) => c,
                None => {
                    return json_resp(
                        StatusCode::BAD_REQUEST,
                        json!({ "error": "nothing to resume — entity has no stored campaign; pass one in the payload" }),
                    );
                }
            },
            Err(e) => {
                return json_resp(
                    StatusCode::BAD_REQUEST,
                    json!({ "error": format!("failed to load campaign: {e}") }),
                );
            }
        };
        self.launch(id, campaign, dry_run(payload))
    }

    /// `campaign.pause` — checkpoint semantics. `queued`/`paused`
    /// park directly; `running` gets the pause flag + an injected
    /// `workflow.cancel`, and the finish handler parks the entity as
    /// `paused` (log + timers preserved for `campaign.resume`) instead
    /// of `cancelled`. Terminal states conflict.
    fn handle_pause(&self, id: &str) -> Response {
        let (state, injector) = {
            let Ok(map) = self.entities.read() else {
                return lock_err();
            };
            let Some(entity) = map.get(id) else {
                return not_found(id);
            };
            (entity.state.clone(), entity.injector.clone())
        };

        match state.as_str() {
            "queued" | "paused" => {
                let Ok(mut map) = self.entities.write() else {
                    return lock_err();
                };
                if let Some(entity) = map.get_mut(id) {
                    if entity.state == "queued" {
                        entity.state = "paused".to_string();
                        entity.updated_at = chrono::Utc::now().to_rfc3339();
                    }
                }
                json_resp(StatusCode::OK, json!({ "status": "paused", "id": id }))
            }
            "running" => {
                let Some(injector) = injector else {
                    // The runner already exited; the finish write will
                    // land the real terminal state momentarily.
                    return json_resp(StatusCode::OK, json!({ "status": "paused", "id": id }));
                };
                if let Ok(mut map) = self.entities.write() {
                    if let Some(entity) = map.get_mut(id) {
                        if entity.state == "running" {
                            entity.pause_requested = true;
                            entity.updated_at = chrono::Utc::now().to_rfc3339();
                        }
                    }
                }
                let _ = injector.inject(Event::cancel("paused via campaign.pause"));
                json_resp(StatusCode::OK, json!({ "status": "pausing", "id": id }))
            }
            _ => json_resp(
                StatusCode::CONFLICT,
                json!({ "error": format!("cannot pause campaign in state '{state}'") }),
            ),
        }
    }

    /// The shared start/resume launch — legal from `queued` (fresh
    /// leg, empty replay) and `paused` (checkpoint resume: sanitized
    /// log + preserved timers replay through `RunOptions`). `running`
    /// and terminal states conflict — a second runner on the same
    /// entity would race the first for its state slot.
    fn launch(&self, id: &str, campaign: Campaign, dry_run: bool) -> Response {
        // Fleet gate (PL-007 C4): `runs_on` labels must all be
        // advertised by THIS node — the op was routed here by entity
        // ownership, but eligibility is the campaign's own contract.
        // A mismatched start conflicts rather than queueing forever
        // on a node that can never satisfy it.
        if !campaign.runs_on_matches(&self.labels) {
            return json_resp(
                StatusCode::CONFLICT,
                json!({
                    "error": format!(
                        "campaign requires runner labels {:?} — this node advertises {:?}",
                        campaign.runs_on, self.labels
                    )
                }),
            );
        }
        let (injector, ingress) = peregrin_engine::ingress();
        let now = chrono::Utc::now().to_rfc3339();

        let (replay_events, replay_timers, live_log, live_timers) = {
            let Ok(mut map) = self.entities.write() else {
                return lock_err();
            };
            let Some(entity) = map.get_mut(id) else {
                return not_found(id);
            };

            match entity.state.as_str() {
                "queued" => {}
                "paused" => {
                    entity.resume_count += 1;
                }
                "running" => {
                    return json_resp(
                        StatusCode::CONFLICT,
                        json!({ "error": "campaign is already running" }),
                    );
                }
                state => {
                    return json_resp(
                        StatusCode::CONFLICT,
                        json!({ "error": format!("cannot start campaign in state '{state}'") }),
                    );
                }
            }

            let resuming = entity.state == "paused";
            let replay_events = if resuming {
                replayable_log(&entity.log)
            } else {
                Vec::new()
            };
            let replay_timers = if resuming {
                entity.pending_timers.clone()
            } else {
                Vec::new()
            };

            entity.state = "running".to_string();
            if entity.started_at.is_none() {
                entity.started_at = Some(now.clone());
            }
            entity.finished_at = None;
            entity.pause_requested = false;
            entity.campaign = Some(campaign.clone());
            let live_log = Arc::new(Mutex::new(Vec::new()));
            entity.live_log = Some(Arc::clone(&live_log));
            let live_timers = Arc::new(Mutex::new(Vec::new()));
            entity.live_timers = Some(Arc::clone(&live_timers));
            entity.injector = Some(injector);
            entity.updated_at = now;
            (replay_events, replay_timers, live_log, live_timers)
        };

        let entity_id = id.to_string();
        let entities = Arc::clone(&self.entities);

        std::thread::spawn(move || {
            // The bus's observer feeds the entity's live log so
            // `list`/`get`/`export` see mid-run state; `seed` bypasses
            // `publish`, so replayed history never double-counts.
            let observer: Arc<dyn Observer> = Arc::new(LiveObserver {
                log: Arc::clone(&live_log),
            });
            let bus = LocalBus::new(Box::new(MemoryStore::default()), observer);
            let mut runner = Runner::new(crate::campaign::registry())
                .with_ingress(ingress)
                .with_bus(Box::new(bus))
                .with_timer_mirror(live_timers);
            let options = RunOptions {
                dry_run,
                secrets: BTreeMap::new(),
                secret_stores: crate::campaign::default_secret_stores(),
                replay_events,
                replay_timers,
            };
            let res = runner.run(&campaign, &options);
            let log = runner.log();
            let pending_timers = runner.pending_timers();

            let terminal_state = if log.iter().any(|e| e.kind == "workflow.cancelled") {
                "cancelled"
            } else if res.is_err() || log.iter().any(|e| e.kind == "workflow.failed") {
                "failed"
            } else {
                "done"
            };

            let finish_time = chrono::Utc::now().to_rfc3339();
            if let Ok(mut map) = entities.write() {
                if let Some(ent) = map.get_mut(&entity_id) {
                    let mut state = terminal_state;
                    if ent.pause_requested {
                        ent.pause_requested = false;
                        if state == "cancelled" {
                            // The pause's checkpoint cancel ended the
                            // run — park as resumable, not cancelled.
                            state = "paused";
                        }
                    }
                    ent.state = state.to_string();
                    if state != "paused" {
                        ent.finished_at = Some(finish_time.clone());
                    }
                    ent.events_count = log.len();
                    ent.fires_count = log.iter().filter(|e| e.source != "__runner__").count();
                    ent.failed_count = log.iter().filter(|e| e.kind == "node.failed").count();
                    ent.log = log;
                    ent.pending_timers = pending_timers;
                    ent.injector = None;
                    ent.live_log = None;
                    ent.live_timers = None;
                    ent.updated_at = finish_time;
                }
            }
        });

        json_resp(StatusCode::OK, json!({ "status": "started", "id": id }))
    }

    fn handle_cancel(&self, id: &str, payload: &Value) -> Response {
        let (state, injector) = {
            let Ok(map) = self.entities.read() else {
                return lock_err();
            };
            let Some(entity) = map.get(id) else {
                return not_found(id);
            };
            (entity.state.clone(), entity.injector.clone())
        };

        match state.as_str() {
            "running" => {
                let reason = payload
                    .get("reason")
                    .and_then(Value::as_str)
                    .unwrap_or("cancelled via op");
                if let Some(injector) = injector {
                    let _ = injector.inject(Event::cancel(reason));
                }
                json_resp(StatusCode::OK, json!({ "status": "cancelling", "id": id }))
            }
            // Nothing is running — a queued or checkpointed entity
            // cancels on the spot (its log stays for audit).
            "queued" | "paused" => {
                let now = chrono::Utc::now().to_rfc3339();
                let Ok(mut map) = self.entities.write() else {
                    return lock_err();
                };
                if let Some(entity) = map.get_mut(id) {
                    if matches!(entity.state.as_str(), "queued" | "paused") {
                        entity.state = "cancelled".to_string();
                        entity.finished_at = Some(now.clone());
                        entity.updated_at = now;
                    }
                }
                json_resp(StatusCode::OK, json!({ "status": "cancelled", "id": id }))
            }
            _ => json_resp(StatusCode::OK, json!({ "status": "not_running", "id": id })),
        }
    }

    fn handle_signal(&self, id: &str, payload: &Value) -> Response {
        let Ok(map) = self.entities.read() else {
            return lock_err();
        };
        let Some(entity) = map.get(id) else {
            return not_found(id);
        };

        if entity.state != "running" {
            return json_resp(
                StatusCode::BAD_REQUEST,
                json!({ "error": "cannot signal campaign that is not running" }),
            );
        }

        let event = if let Some(kind) = payload.get("kind").and_then(Value::as_str) {
            let p = payload.get("payload").cloned().unwrap_or_else(|| json!({}));
            Event::new(kind, p, "api")
        } else {
            Event::new("signal", payload.clone(), "api")
        };

        if let Some(injector) = &entity.injector {
            let _ = injector.inject(event);
        }
        json_resp(StatusCode::OK, json!({ "status": "signalled", "id": id }))
    }
}
