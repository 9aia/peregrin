//! The entity surface — `/api/entities*` is the substrate-facing view
//! of everything a node hosts. Today the only resident kind is
//! `sepia-session`, so this module wraps the session plane into entity
//! docs (`{id,kind,indexDoc,updatedAt,locked,live}`) and dispatches
//! namespaced ops (`sepia.<verb>`) onto the session verb handlers —
//! the session-domain knowledge the sync client used to carry.
//! INTERIM: when the `sepia-peregrin` integration registers other
//! kinds this becomes a registry lookup; the op dispatch splits
//! per-kind instead of mapping onto `/api/sessions` verbs.

use axum::body::Body;
use axum::extract::{Path, Request, State};
use axum::http::{Method, StatusCode};
use axum::response::Response;
use serde_json::{Value, json};

use crate::AppState;

use super::{
    agent_param, error_response, json_response, not_found, query_param, read_json_body, sessions,
};

/// The only entity kind today's nodes host — the session plane
/// projected onto the entity contract.
const SESSION_KIND: &str = "sepia-session";

fn bad_request(message: &str) -> Response {
    json_response(json!({ "error": message }), StatusCode::BAD_REQUEST)
}

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

/// `GET /api/entities[?withLocks=1]` — entity docs for every resident
/// entity. `POST /api/entities` — create (`{kind, indexDoc}` → the
/// kind's create params; only `sepia-session` exists today).
pub async fn collection(
    State(state): State<AppState>,
    method: Method,
    req: Request<Body>,
) -> Response {
    match method {
        Method::GET => {
            let with_locks = query_param(req.uri().query(), "withLocks").as_deref() == Some("1");
            let res = sessions::list_for_entities(&state, with_locks).await;
            let Ok(rows) = res else {
                return json_response(json!({ "entities": [] }), StatusCode::OK);
            };
            let entities: Vec<Value> = rows.iter().map(entity_doc).collect();
            json_response(json!({ "entities": entities }), StatusCode::OK)
        }
        Method::POST => create(state, req).await,
        _ => not_found(),
    }
}

/// `POST /api/entities` — `{kind, indexDoc}`. INTERIM: `indexDoc` is
/// passed through as the session create body (`cwd`, `agent`, …).
async fn create(state: AppState, req: Request<Body>) -> Response {
    let body = match read_json_body(req, crate::routes::SMALL_JSON_BODY).await {
        Ok(b) => b,
        Err(res) => return *res,
    };
    let Some(body) = body else {
        return bad_request("Expected a JSON object body");
    };
    let kind = body
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or(SESSION_KIND);
    if kind != SESSION_KIND {
        return json_response(
            json!({ "error": format!("entity kind {kind:?} not registered") }),
            StatusCode::NOT_IMPLEMENTED,
        );
    }
    let init = body
        .get("indexDoc")
        .cloned()
        .unwrap_or_else(|| body.clone());
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/sessions")
        .body(Body::from(serde_json::to_vec(&init).unwrap_or_default()))
        .unwrap_or_else(|_| Request::new(Body::empty()));
    sessions::collection(State(state), Method::POST, req).await
}

/// `GET /api/entities/{id}` — one entity doc.
pub async fn item(
    State(state): State<AppState>,
    method: Method,
    Path(id): Path<String>,
    req: Request<Body>,
) -> Response {
    match method {
        Method::GET => {
            let agent = agent_param(&req);
            let res = sessions::summary_for_entities(&state, &id, agent.as_deref()).await;
            match res {
                Ok(summary) => json_response(entity_doc(&summary), StatusCode::OK),
                Err(e) => error_response(&e),
            }
        }
        _ => not_found(),
    }
}

/// `POST /api/entities/{id}/ops` — submit `{kind, payload}`. `sepia.<verb>`
/// dispatches onto the session verb handlers (their own JSON translation
/// and feed emission stay intact); `entity.delete` → the item DELETE.
pub async fn ops(
    State(state): State<AppState>,
    method: Method,
    Path(id): Path<String>,
    req: Request<Body>,
) -> Response {
    if method != Method::POST {
        return not_found();
    }
    let agent = agent_param(&req);
    let body = match read_json_body(req, crate::routes::SMALL_JSON_BODY).await {
        Ok(b) => b,
        Err(res) => return *res,
    };
    let Some(body) = body else {
        return bad_request("Expected a JSON object body");
    };
    let kind = body.get("kind").and_then(Value::as_str).unwrap_or_default();
    let payload = body.get("payload").cloned().unwrap_or_else(|| json!({}));

    // Rebuild the inner verb request — the session handlers own their
    // own JSON translation, lock handling, and feed emission.
    let verb_req = |id: &str, verb_path: &str| {
        use std::fmt::Write as _;
        let mut uri = format!("/api/sessions/{id}{verb_path}");
        if let Some(a) = &agent {
            let _ = write!(uri, "?agent={a}");
        }
        Request::builder()
            .method(Method::POST)
            .uri(uri)
            .body(Body::from(serde_json::to_vec(&payload).unwrap_or_default()))
            .unwrap_or_else(|_| Request::new(Body::empty()))
    };

    // `sepia.` strips; un-namespaced ops (legacy outbox rows) read as
    // session-plane verbs; `entity.*` is the substrate namespace.
    let verb = kind.strip_prefix("sepia.").unwrap_or(kind);
    match verb {
        "meta" | "meta.patch" => {
            sessions::item(
                State(state),
                Method::PATCH,
                Path(id.clone()),
                verb_req(&id, ""),
            )
            .await
        }
        "attach" => {
            sessions::attach(
                State(state),
                Method::POST,
                Path(id.clone()),
                verb_req(&id, "/attach"),
            )
            .await
        }
        "detach" => {
            sessions::detach(
                State(state),
                Method::POST,
                Path(id.clone()),
                verb_req(&id, "/detach"),
            )
            .await
        }
        "prompt" => {
            sessions::prompt(
                State(state),
                Method::POST,
                Path(id.clone()),
                verb_req(&id, "/prompt"),
            )
            .await
        }
        "cancel" => {
            sessions::cancel(
                State(state),
                Method::POST,
                Path(id.clone()),
                verb_req(&id, "/cancel"),
            )
            .await
        }
        "permission" => {
            sessions::permission(
                State(state),
                Method::POST,
                Path(id.clone()),
                verb_req(&id, "/permission"),
            )
            .await
        }
        "restore" => {
            sessions::restore(
                State(state),
                Method::POST,
                Path(id.clone()),
                verb_req(&id, "/restore"),
            )
            .await
        }
        "rewind" => {
            sessions::rewind(
                State(state),
                Method::POST,
                Path(id.clone()),
                verb_req(&id, "/rewind"),
            )
            .await
        }
        "convert" => {
            sessions::convert(
                State(state),
                Method::POST,
                Path(id.clone()),
                verb_req(&id, "/convert"),
            )
            .await
        }
        "entity.delete" | "delete" => {
            sessions::item(
                State(state),
                Method::DELETE,
                Path(id.clone()),
                verb_req(&id, ""),
            )
            .await
        }
        _ => json_response(
            json!({ "error": format!("unknown op {kind:?}") }),
            StatusCode::NOT_IMPLEMENTED,
        ),
    }
}

/// `GET /api/entities/{id}/snapshot` — kind-shaped snapshot bytes the
/// substrate moves opaquely (`sepia-session` → the session export).
pub async fn snapshot_item(
    State(state): State<AppState>,
    method: Method,
    Path(id): Path<String>,
    req: Request<Body>,
) -> Response {
    sessions::export(State(state), method, Path(id), req).await
}

/// `POST /api/entities/snapshot` — import a snapshot on this node.
pub async fn snapshot_collection(
    State(state): State<AppState>,
    method: Method,
    req: Request<Body>,
) -> Response {
    sessions::import_session(State(state), method, req).await
}
