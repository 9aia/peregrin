//! The substrate's HTTP entity surface — `/api/entities*` on the hub.
//! Reads hit the merged projection (every paired node), ops go through
//! `SyncEntityApi::submit_op` so they queue in the durable outbox when
//! the owning node is down, and snapshots route to the owning/target
//! node. These handlers are registered ahead of the `/api/{*rest}`
//! passthrough so the merged surface wins.

use axum::body::Body;
use axum::extract::{Path, Request, State};
use axum::http::{Method, StatusCode};
use axum::response::{IntoResponse, Json, Response};
use peregrin_node_api::dto::EntityOp;
use serde_json::{Value, json};

use crate::HubState;

fn json_response(body: Value, status: StatusCode) -> Response {
    (status, Json(body)).into_response()
}

/// `EntityApi` errors → status codes: not-found 404, unregistered
/// kinds/ops 501, lock contention 409, everything else 500.
fn api_err(e: &str) -> Response {
    let status = if e.contains("not found") || e.contains("Entity not found") {
        StatusCode::NOT_FOUND
    } else if e.contains("not registered") || e.contains("unknown op") {
        StatusCode::NOT_IMPLEMENTED
    } else if e.contains("held by") || e.contains("held on") {
        StatusCode::CONFLICT
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    };
    json_response(json!({ "error": e }), status)
}

fn bad_request(msg: &str) -> Response {
    json_response(json!({ "error": msg }), StatusCode::BAD_REQUEST)
}

fn query_param(req: &Request, key: &str) -> Option<String> {
    req.uri().query().and_then(|q| {
        q.split('&').find_map(|p| {
            p.split_once('=')
                .and_then(|(k, v)| if k == key { Some(v.to_string()) } else { None })
        })
    })
}

/// `GET /api/entities[?kind=][&node=]` — the merged entity index.
/// `POST /api/entities` — create on the named (or primary) node.
pub async fn collection(
    State(state): State<HubState>,
    method: Method,
    req: Request<Body>,
) -> Response {
    match method {
        Method::GET => {
            let kind = query_param(&req, "kind");
            match state.node.list_entities(kind.as_deref()).await {
                Ok(entities) => json_response(json!({ "entities": entities }), StatusCode::OK),
                Err(e) => api_err(&e),
            }
        }
        Method::POST => {
            let node_q = query_param(&req, "node");
            let body = match read_body(req).await {
                Ok(b) => b,
                Err(res) => return *res,
            };
            let kind = body
                .get("kind")
                .and_then(Value::as_str)
                .unwrap_or("sepia-session");
            let init = body
                .get("indexDoc")
                .cloned()
                .unwrap_or_else(|| body.clone());
            let node = body
                .get("node")
                .and_then(Value::as_str)
                .map(str::to_string)
                .or(node_q);
            match state.node.create_entity(kind, &init, node.as_deref()).await {
                Ok(res) => json_response(json!(res), StatusCode::CREATED),
                Err(e) => api_err(&e),
            }
        }
        _ => not_found(),
    }
}

/// `GET /api/entities/{id}` — one entity doc. `DELETE` → `entity.delete`.
pub async fn item(
    State(state): State<HubState>,
    method: Method,
    Path(id): Path<String>,
    _req: Request<Body>,
) -> Response {
    match method {
        Method::GET => match state.node.entity(&id).await {
            Ok(entity) => json_response(json!(entity), StatusCode::OK),
            Err(e) => api_err(&e),
        },
        Method::DELETE => match state.node.delete_entity(&id).await {
            Ok(()) => json_response(json!({ "ok": true }), StatusCode::OK),
            Err(e) => api_err(&e),
        },
        _ => not_found(),
    }
}

/// `POST /api/entities/{id}/ops` — `{kind, payload}` through the outbox.
pub async fn ops(
    State(state): State<HubState>,
    method: Method,
    Path(id): Path<String>,
    req: Request<Body>,
) -> Response {
    if method != Method::POST {
        return not_found();
    }
    let body = match read_body(req).await {
        Ok(b) => b,
        Err(res) => return *res,
    };
    let Some(kind) = body.get("kind").and_then(Value::as_str) else {
        return bad_request("op kind is required");
    };
    let op = EntityOp {
        kind: kind.to_string(),
        payload: body.get("payload").cloned().unwrap_or_else(|| json!({})),
    };
    match state.node.submit_op(&id, &op).await {
        Ok(()) => json_response(json!({ "ok": true }), StatusCode::ACCEPTED),
        Err(e) => api_err(&e),
    }
}

/// `GET /api/entities/{id}/snapshot` — export from the owning node.
pub async fn snapshot_item(
    State(state): State<HubState>,
    method: Method,
    Path(id): Path<String>,
    _req: Request<Body>,
) -> Response {
    if method != Method::GET {
        return not_found();
    }
    match state.node.snapshot_export(&id).await {
        Ok(snapshot) => json_response(snapshot, StatusCode::OK),
        Err(e) => api_err(&e),
    }
}

/// `POST /api/entities/snapshot` — import on `?node=` (or the primary).
pub async fn snapshot_collection(
    State(state): State<HubState>,
    method: Method,
    req: Request<Body>,
) -> Response {
    if method != Method::POST {
        return not_found();
    }
    let node = query_param(&req, "node");
    let body = match read_body(req).await {
        Ok(b) => b,
        Err(res) => return *res,
    };
    match state.node.snapshot_import(&body, node.as_deref()).await {
        Ok(entity) => json_response(json!(entity), StatusCode::CREATED),
        Err(e) => api_err(&e),
    }
}

/// `GET /api/entity-index` — per-node entity buckets (node→entities).
pub async fn entity_index(State(state): State<HubState>, method: Method) -> Response {
    if method != Method::GET {
        return not_found();
    }
    match state.node.entity_index().await {
        Ok(index) => json_response(json!({ "index": index }), StatusCode::OK),
        Err(e) => api_err(&e),
    }
}

/// `GET /api/pending-ops` — outbox depth: queued + dead-letter ops.
pub async fn pending_ops(State(state): State<HubState>, method: Method) -> Response {
    if method != Method::GET {
        return not_found();
    }
    match state.node.pending_ops().await {
        Ok(ops) => json_response(json!({ "ops": ops }), StatusCode::OK),
        Err(e) => api_err(&e),
    }
}

/// Body → JSON object (`{}` when absent/empty); oversized/invalid →
/// the boxed response the handler returns early.
async fn read_body(req: Request<Body>) -> Result<Value, Box<Response>> {
    const CAP: usize = 64 * 1024;
    let bytes = axum::body::to_bytes(req.into_body(), CAP)
        .await
        .map_err(|_| Box::new((StatusCode::PAYLOAD_TOO_LARGE, "body too large").into_response()))?;
    if bytes.is_empty() {
        return Ok(json!({}));
    }
    serde_json::from_slice(&bytes).map_err(|_| Box::new(bad_request("Expected a JSON object body")))
}

fn not_found() -> Response {
    json_response(json!({ "error": "not found" }), StatusCode::NOT_FOUND)
}
