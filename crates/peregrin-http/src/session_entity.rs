//! `SessionEntityPlane` — the `kind="sepia-session"` [`EntityPlane`]
//! impl. It projects the session plane (`/api/sessions*` handlers)
//! onto the entity contract: wire summary rows become entity docs,
//! `sepia.<verb>` ops dispatch onto the session verb handlers, and
//! snapshots are the session export/import pair. Stateless — every
//! method downcasts the [`AppState`] the node host stuffs into the
//! [`EntityContext`] the substrate passes in, so the same instance
//! serves whichever `SessionPlane` the daemon mounted.
//!
//! The `?agent=` disambiguator is this kind's own query param — it
//! scopes the session-store lookup the way `/api/sessions*` does and
//! never leaks into the substrate dispatch.

use std::fmt::Write as _;

use axum::body::Body;
use axum::extract::{Path, Request, State};
use axum::http::{Method, StatusCode};
use axum::response::Response;
use serde_json::{Value, json};

use crate::AppState;
use crate::entity_plane::{EntityContext, EntityPlane};
use crate::routes::{error_response, json_response, query_param, sessions};

/// The entity kind tag sessions project as — registered
/// automatically wherever a `SessionPlane` is mounted.
pub const SESSION_KIND: &str = "sepia-session";

/// The `sepia-session` entity plane — see the module docs.
pub struct SessionEntityPlane;

/// A wire summary row → the entity doc the substrate indexes. The
/// `indexDoc` is opaque to peregrin — kind-specific fields for list
/// views; `locked`/`live`/`updatedAt` lift to substrate columns.
fn entity_doc(summary: &Value) -> Value {
    let get = |k: &str| summary.get(k).cloned().unwrap_or(Value::Null);
    json!({
        "id": get("id"),
        "kind": SESSION_KIND,
        "indexDoc": summary,
        "updatedAt": get("updatedAt"),
        "locked": get("locked"),
        "live": summary.get("live").cloned().unwrap_or_else(|| get("busy")),
    })
}

/// The `?agent=` store disambiguator — empty reads as absent.
fn agent_of(query: Option<&str>) -> Option<String> {
    query_param(query, "agent").filter(|a| !a.is_empty())
}

/// Rebuild a session-plane request carrying `payload` and the `?agent`
/// disambiguator — the session handlers own their JSON translation,
/// lock handling, and feed emission.
fn verb_req(id: &str, verb_path: &str, agent: Option<&str>, payload: &Value) -> Request<Body> {
    let mut uri = format!("/api/sessions/{id}{verb_path}");
    if let Some(a) = agent {
        let _ = write!(uri, "?agent={a}");
    }
    Request::builder()
        .method(Method::POST)
        .uri(uri)
        .body(Body::from(serde_json::to_vec(payload).unwrap_or_default()))
        .unwrap_or_else(|_| Request::new(Body::empty()))
}

/// The `AppState` the node host stuffed into the [`EntityContext`] —
/// this plane can't serve without it (it *is* the session surface).
fn app_state(ctx: &EntityContext) -> Option<&AppState> {
    ctx.host::<AppState>()
}

/// A plane invoked on a host that isn't an `AppState` — can't happen
/// on the peregrin node; a foreign host gets a loud 500, not a panic.
fn host_missing() -> Response {
    json_response(
        json!({ "error": "entity host unavailable" }),
        StatusCode::INTERNAL_SERVER_ERROR,
    )
}

#[async_trait::async_trait]
impl EntityPlane for SessionEntityPlane {
    fn kind(&self) -> &'static str {
        SESSION_KIND
    }

    /// `sepia.*` is the op namespace — `sepia.prompt`, `sepia.meta`, …
    fn op_namespace(&self) -> &'static str {
        "sepia"
    }

    /// `meta.*` is the node-local overlay (`PEREGRIN_META` — pins,
    /// archives, run spans); the op table handles `meta`/`meta.patch`.
    fn extra_op_namespaces(&self) -> &'static [&'static str] {
        &["meta"]
    }

    async fn list(&self, ctx: &EntityContext, query: Option<&str>) -> Vec<Value> {
        let Some(state) = app_state(ctx) else {
            return Vec::new();
        };
        let with_locks = query_param(query, "withLocks").as_deref() == Some("1");
        match sessions::list_for_entities(state, with_locks).await {
            Ok(rows) => rows.iter().map(entity_doc).collect(),
            // Same silent-200 the list route always had.
            Err(_) => Vec::new(),
        }
    }

    async fn get(&self, ctx: &EntityContext, id: &str, query: Option<&str>) -> Response {
        let Some(state) = app_state(ctx) else {
            return host_missing();
        };
        let agent = agent_of(query);
        match sessions::summary_for_entities(state, id, agent.as_deref()).await {
            Ok(summary) => json_response(entity_doc(&summary), StatusCode::OK),
            Err(e) => error_response(&e),
        }
    }

    async fn create(&self, ctx: &EntityContext, init: Value) -> Response {
        let Some(state) = app_state(ctx) else {
            return host_missing();
        };
        // `init` is passed through as the session create body
        // (`cwd`, `agent`, `title`, `model`, `fallbacks`).
        let req = Request::builder()
            .method(Method::POST)
            .uri("/api/sessions")
            .body(Body::from(serde_json::to_vec(&init).unwrap_or_default()))
            .unwrap_or_else(|_| Request::new(Body::empty()));
        sessions::collection(State(state.clone()), Method::POST, req).await
    }

    async fn delete(&self, ctx: &EntityContext, id: &str, query: Option<&str>) -> Response {
        let Some(state) = app_state(ctx) else {
            return host_missing();
        };
        let agent = agent_of(query);
        sessions::item(
            State(state.clone()),
            Method::DELETE,
            Path(id.to_string()),
            verb_req(id, "", agent.as_deref(), &json!({})),
        )
        .await
    }

    async fn op(
        &self,
        ctx: &EntityContext,
        id: &str,
        op: &str,
        payload: Value,
        query: Option<&str>,
    ) -> Response {
        let Some(state) = app_state(ctx) else {
            return host_missing();
        };
        let agent = agent_of(query);
        let req = |verb_path: &str| verb_req(id, verb_path, agent.as_deref(), &payload);
        // `sepia.` strips; un-namespaced ops (legacy outbox rows) read
        // as session-plane verbs.
        let verb = op.strip_prefix("sepia.").unwrap_or(op);
        match verb {
            "meta" | "meta.patch" => {
                sessions::item(
                    State(state.clone()),
                    Method::PATCH,
                    Path(id.to_string()),
                    req(""),
                )
                .await
            }
            "attach" => {
                sessions::attach(
                    State(state.clone()),
                    Method::POST,
                    Path(id.to_string()),
                    req("/attach"),
                )
                .await
            }
            "detach" => {
                sessions::detach(
                    State(state.clone()),
                    Method::POST,
                    Path(id.to_string()),
                    req("/detach"),
                )
                .await
            }
            "prompt" => {
                sessions::prompt(
                    State(state.clone()),
                    Method::POST,
                    Path(id.to_string()),
                    req("/prompt"),
                )
                .await
            }
            "cancel" => {
                sessions::cancel(
                    State(state.clone()),
                    Method::POST,
                    Path(id.to_string()),
                    req("/cancel"),
                )
                .await
            }
            "permission" => {
                sessions::permission(
                    State(state.clone()),
                    Method::POST,
                    Path(id.to_string()),
                    req("/permission"),
                )
                .await
            }
            "restore" => {
                sessions::restore(
                    State(state.clone()),
                    Method::POST,
                    Path(id.to_string()),
                    req("/restore"),
                )
                .await
            }
            "rewind" => {
                sessions::rewind(
                    State(state.clone()),
                    Method::POST,
                    Path(id.to_string()),
                    req("/rewind"),
                )
                .await
            }
            "convert" => {
                sessions::convert(
                    State(state.clone()),
                    Method::POST,
                    Path(id.to_string()),
                    req("/convert"),
                )
                .await
            }
            "entity.delete" | "delete" => {
                sessions::item(
                    State(state.clone()),
                    Method::DELETE,
                    Path(id.to_string()),
                    req(""),
                )
                .await
            }
            _ => json_response(
                json!({ "error": format!("unknown op {op:?}") }),
                StatusCode::NOT_IMPLEMENTED,
            ),
        }
    }

    async fn export(&self, ctx: &EntityContext, id: &str, query: Option<&str>) -> Response {
        let Some(state) = app_state(ctx) else {
            return host_missing();
        };
        let agent = agent_of(query);
        let mut uri = format!("/api/sessions/{id}/export");
        if let Some(a) = agent {
            let _ = write!(uri, "?agent={a}");
        }
        let req = Request::builder()
            .method(Method::GET)
            .uri(uri)
            .body(Body::empty())
            .unwrap_or_else(|_| Request::new(Body::empty()));
        sessions::export(State(state.clone()), Method::GET, Path(id.to_string()), req).await
    }

    async fn import(&self, ctx: &EntityContext, body: Value) -> Response {
        let Some(state) = app_state(ctx) else {
            return host_missing();
        };
        // The import body is the sessions-import shape verbatim —
        // re-hydrate it into the `/api/sessions/import` handler.
        let req = Request::builder()
            .method(Method::POST)
            .uri("/api/sessions/import")
            .body(Body::from(serde_json::to_vec(&body).unwrap_or_default()))
            .unwrap_or_else(|_| Request::new(Body::empty()));
        sessions::import_session(State(state.clone()), Method::POST, req).await
    }
}
