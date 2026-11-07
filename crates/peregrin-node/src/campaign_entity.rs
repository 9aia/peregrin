//! The campaign entity plane — registers `kind="campaign"` on the
//! node's entity surface (P-001, ADR-0091, PL-004 phase 6).
//!
//! A campaign entity models a *run* (`{id, kind: "campaign", indexDoc}`),
//! created via `POST /api/entities`, driven via `campaign.*` ops
//! (`campaign.start`, `campaign.cancel`, `campaign.signal`), observable
//! in the merged index, and movable across nodes via snapshot
//! export (`campaign-snapshot/v1`) → import.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::sync::{Arc, RwLock};

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use peregrin_core::{Campaign, Event};
use peregrin_engine::{Injector, PendingTimer, RunOptions, Runner};
use peregrin_http::AppState;
use peregrin_http::entity_plane::EntityPlane;
use peregrin_loader::Loader;
use peregrin_loader_yaml::YamlLoader;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub const CAMPAIGN_KIND: &str = "campaign";

/// An in-memory campaign run entity tracked by this plane.
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CampaignEntity {
    pub id: String,
    pub name: String,
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
    pub log: Vec<Event>,
    pub pending_timers: Vec<PendingTimer>,
    #[serde(skip)]
    pub injector: Option<Injector>,
    pub created_at: String,
    pub updated_at: String,
}

/// The resident `kind="campaign"` plane impl.
#[derive(Clone, Default)]
pub struct CampaignEntityPlane {
    entities: Arc<RwLock<HashMap<String, CampaignEntity>>>,
}

impl CampaignEntityPlane {
    #[must_use]
    pub fn new() -> Self {
        Self {
            entities: Arc::new(RwLock::new(HashMap::new())),
        }
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

fn entity_doc(entity: &CampaignEntity) -> Value {
    let is_running = entity.state == "running";
    json!({
        "id": entity.id,
        "kind": CAMPAIGN_KIND,
        "indexDoc": {
            "name": entity.name,
            "state": entity.state,
            "campaignRef": entity.campaign_ref,
            "startedAt": entity.started_at,
            "finishedAt": entity.finished_at,
            "counts": {
                "events": entity.events_count,
                "fires": entity.fires_count,
                "failed": entity.failed_count,
            },
            "pendingTimers": entity.pending_timers.len(),
        },
        "live": is_running,
        "locked": is_running,
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

#[async_trait::async_trait]
impl EntityPlane for CampaignEntityPlane {
    fn kind(&self) -> &'static str {
        CAMPAIGN_KIND
    }

    fn op_namespace(&self) -> &'static str {
        CAMPAIGN_KIND
    }

    async fn list(&self, _state: &AppState, _query: Option<&str>) -> Vec<Value> {
        let Ok(map) = self.entities.read() else {
            return Vec::new();
        };
        map.values().map(entity_doc).collect()
    }

    async fn get(&self, _state: &AppState, id: &str, _query: Option<&str>) -> Response {
        let Ok(map) = self.entities.read() else {
            return json_resp(
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({ "error": "lock error" }),
            );
        };
        map.get(id).map_or_else(
            || {
                json_resp(
                    StatusCode::NOT_FOUND,
                    json!({ "error": format!("campaign entity '{id}' not found") }),
                )
            },
            |entity| json_resp(StatusCode::OK, entity_doc(entity)),
        )
    }

    async fn create(&self, _state: &AppState, init: Value) -> Response {
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
            log: Vec::new(),
            pending_timers: Vec::new(),
            injector: None,
            created_at: now.clone(),
            updated_at: now,
        };

        let Ok(mut map) = self.entities.write() else {
            return json_resp(
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({ "error": "lock error" }),
            );
        };
        let doc = entity_doc(&entity);
        map.insert(id, entity);
        json_resp(StatusCode::CREATED, doc)
    }

    async fn delete(&self, _state: &AppState, id: &str, query: Option<&str>) -> Response {
        let force = query.is_some_and(|q| q.contains("force=true") || q.contains("force=1"));
        let Ok(mut map) = self.entities.write() else {
            return json_resp(
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({ "error": "lock error" }),
            );
        };
        let Some(entity) = map.get(id) else {
            return json_resp(
                StatusCode::NOT_FOUND,
                json!({ "error": format!("campaign entity '{id}' not found") }),
            );
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
        _state: &AppState,
        id: &str,
        op: &str,
        payload: Value,
        _query: Option<&str>,
    ) -> Response {
        let verb = op.strip_prefix("campaign.").unwrap_or(op);
        match verb {
            "start" => self.handle_start(id, &payload),
            "cancel" => self.handle_cancel(id, &payload),
            "signal" => self.handle_signal(id, &payload),
            _ => json_resp(
                StatusCode::NOT_IMPLEMENTED,
                json!({ "error": format!("unknown campaign op '{op}'") }),
            ),
        }
    }

    async fn export(&self, _state: &AppState, id: &str, _query: Option<&str>) -> Response {
        let Ok(map) = self.entities.read() else {
            return json_resp(
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({ "error": "lock error" }),
            );
        };
        let Some(entity) = map.get(id) else {
            return json_resp(
                StatusCode::NOT_FOUND,
                json!({ "error": format!("campaign entity '{id}' not found") }),
            );
        };

        let snapshot = json!({
            "format": "campaign-snapshot/v1",
            "kind": CAMPAIGN_KIND,
            "entity": {
                "id": entity.id,
                "kind": CAMPAIGN_KIND,
            },
            "campaign": entity.campaign,
            "campaignRef": entity.campaign_ref,
            "log": entity.log,
            "pendingTimers": entity.pending_timers,
            "state": {},
            "counters": {
                "events": entity.events_count,
                "fires": entity.fires_count,
                "failed": entity.failed_count,
            }
        });
        json_resp(StatusCode::OK, snapshot)
    }

    async fn import(&self, _state: &AppState, body: Value) -> Response {
        if body.get("format").and_then(Value::as_str) != Some("campaign-snapshot/v1") {
            return json_resp(
                StatusCode::BAD_REQUEST,
                json!({ "error": "expected format 'campaign-snapshot/v1'" }),
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

        let state = if log.iter().any(|e| e.kind == "workflow.cancelled") {
            "cancelled"
        } else if log.iter().any(|e| e.kind == "workflow.failed") {
            "failed"
        } else if log.iter().any(|e| e.kind == "workflow.done") {
            "done"
        } else if !log.is_empty() {
            "running"
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
            log,
            pending_timers,
            injector: None,
            created_at: now.clone(),
            updated_at: now,
        };

        let Ok(mut map) = self.entities.write() else {
            return json_resp(
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({ "error": "lock error" }),
            );
        };
        let doc = entity_doc(&entity);
        map.insert(id, entity);
        json_resp(StatusCode::CREATED, doc)
    }
}

impl CampaignEntityPlane {
    fn handle_start(&self, id: &str, payload: &Value) -> Response {
        let mut campaign = match parse_campaign(payload) {
            Ok(c) => c,
            Err(e) => {
                return json_resp(
                    StatusCode::BAD_REQUEST,
                    json!({ "error": format!("failed to load campaign: {e}") }),
                );
            }
        };

        if let Some(config) = payload
            .get("params")
            .and_then(|p| p.get("config"))
            .and_then(Value::as_object)
        {
            for (k, v) in config {
                campaign.config.insert(k.clone(), v.clone());
            }
        }

        let dry_run = payload
            .get("dryRun")
            .and_then(Value::as_bool)
            .unwrap_or(false);

        let (injector, ingress) = peregrin_engine::ingress();
        let now = chrono::Utc::now().to_rfc3339();

        let (replay_events, replay_timers) = {
            let Ok(mut map) = self.entities.write() else {
                return json_resp(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    json!({ "error": "lock error" }),
                );
            };
            let Some(entity) = map.get_mut(id) else {
                return json_resp(
                    StatusCode::NOT_FOUND,
                    json!({ "error": format!("campaign entity '{id}' not found") }),
                );
            };

            if entity.state != "queued" && entity.state != "running" {
                return json_resp(
                    StatusCode::CONFLICT,
                    json!({ "error": format!("cannot start campaign in state '{}'", entity.state) }),
                );
            }

            entity.state = "running".to_string();
            entity.started_at = Some(now.clone());
            entity.updated_at = now;
            entity.campaign = Some(campaign.clone());
            entity.injector = Some(injector);
            (entity.log.clone(), entity.pending_timers.clone())
        };

        let entity_id = id.to_string();
        let entities = Arc::clone(&self.entities);

        std::thread::spawn(move || {
            let mut runner = Runner::new(crate::campaign::registry()).with_ingress(ingress);
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
                    ent.state = terminal_state.to_string();
                    ent.finished_at = Some(finish_time.clone());
                    ent.events_count = log.len();
                    ent.fires_count = log.iter().filter(|e| e.source != "__runner__").count();
                    ent.failed_count = log.iter().filter(|e| e.kind == "node.failed").count();
                    ent.log = log;
                    ent.pending_timers = pending_timers;
                    ent.injector = None;
                    ent.updated_at = finish_time;
                }
            }
        });

        json_resp(StatusCode::OK, json!({ "status": "started", "id": id }))
    }

    fn handle_cancel(&self, id: &str, payload: &Value) -> Response {
        let Ok(map) = self.entities.read() else {
            return json_resp(
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({ "error": "lock error" }),
            );
        };
        let Some(entity) = map.get(id) else {
            return json_resp(
                StatusCode::NOT_FOUND,
                json!({ "error": format!("campaign entity '{id}' not found") }),
            );
        };

        if entity.state != "running" {
            return json_resp(StatusCode::OK, json!({ "status": "not_running", "id": id }));
        }

        let reason = payload
            .get("reason")
            .and_then(Value::as_str)
            .unwrap_or("cancelled via op");
        if let Some(injector) = &entity.injector {
            let _ = injector.inject(Event::cancel(reason));
        }
        json_resp(StatusCode::OK, json!({ "status": "cancelling", "id": id }))
    }

    fn handle_signal(&self, id: &str, payload: &Value) -> Response {
        let Ok(map) = self.entities.read() else {
            return json_resp(
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({ "error": "lock error" }),
            );
        };
        let Some(entity) = map.get(id) else {
            return json_resp(
                StatusCode::NOT_FOUND,
                json!({ "error": format!("campaign entity '{id}' not found") }),
            );
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
