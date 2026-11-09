//! The entity surface — `/api/entities*` is the substrate-facing view
//! of everything a node hosts. Every handler here is kind-agnostic:
//! it parses just enough of the request to pick a plane out of the
//! [`EntityContext::kinds`] registry (`body.kind`, the `<ns>.<verb>`
//! op namespace, or owner-resolution across planes for id-scoped
//! calls) and hands over verbatim — payload translation, status
//! codes, and feed emission stay inside each kind's [`EntityPlane`]
//! impl.
//!
//! `sepia-session` is the built-in plane (`SessionEntityPlane`,
//! auto-registered with the session plane); integrations register
//! more at daemon composition. `entity.*` is the substrate's own op
//! namespace: `entity.delete` resolves the owning plane and deletes
//! through it.
//!
//! Handlers still extract [`AppState`] (they share the router with the
//! session surface); the entity-plane contract itself only ever sees
//! the [`EntityContext`] built from it.

use axum::body::Body;
use axum::extract::{Path, Request, State};
use axum::http::{Method, StatusCode};
use axum::response::Response;
use serde_json::{Value, json};
use std::sync::Arc;

use crate::AppState;
use crate::entity_plane::{EntityContext, EntityPlane};

use super::{
    MAX_JSON_BODY, SMALL_JSON_BODY, json_response, not_found, ok, query_param, read_json_body,
};

fn bad_request(message: &str) -> Response {
    json_response(json!({ "error": message }), StatusCode::BAD_REQUEST)
}

/// Unknown kind on create/import — kept at 501 with the long-standing
/// `not registered` wording the hub's `api_err` maps back to 501.
fn kind_not_registered(kind: &str) -> Response {
    json_response(
        json!({ "error": format!("entity kind {kind:?} not registered") }),
        StatusCode::NOT_IMPLEMENTED,
    )
}

fn unknown_op(kind: &str) -> Response {
    json_response(
        json!({ "error": format!("unknown op {kind:?}") }),
        StatusCode::NOT_IMPLEMENTED,
    )
}

/// First plane whose `get` claims `id` — any non-404 answer counts
/// (a 500 means "mine, but broken": more informative than probing on).
async fn owning_plane(
    ctx: &EntityContext,
    id: &str,
    query: Option<&str>,
) -> Option<Arc<dyn EntityPlane>> {
    for plane in ctx.kinds.planes() {
        if plane.get(ctx, id, query).await.status() != StatusCode::NOT_FOUND {
            return Some(Arc::clone(plane));
        }
    }
    None
}

/// `entity.delete` — the substrate verb: find the owning plane and
/// delete through it. Nobody owning the id reads as already-gone —
/// delete stays idempotent (replayed outbox deletes must not
/// dead-letter on a node that already applied them).
async fn delete_entity(ctx: &EntityContext, id: &str, query: Option<&str>) -> Response {
    match owning_plane(ctx, id, query).await {
        Some(plane) => plane.delete(ctx, id, query).await,
        None => ok(),
    }
}

/// `GET /api/entities[?kind=][&withLocks=1]` — every registered
/// plane's entity docs (or just one kind's when `?kind=` filters);
/// `POST /api/entities` — `{kind?, indexDoc}` → the kind's create.
pub async fn collection(
    State(state): State<AppState>,
    method: Method,
    req: Request<Body>,
) -> Response {
    match method {
        Method::GET => {
            let ctx = state.entity_context();
            let query = req.uri().query();
            let kind = query_param(query, "kind");
            let mut entities = Vec::new();
            for plane in ctx.kinds.planes() {
                if kind.as_deref().is_some_and(|k| k != plane.kind()) {
                    continue;
                }
                entities.extend(plane.list(&ctx, query).await);
            }
            json_response(json!({ "entities": entities }), StatusCode::OK)
        }
        Method::POST => create(state, req).await,
        _ => not_found(),
    }
}

/// `POST /api/entities` — `{kind?, indexDoc}`; a missing `kind`
/// targets the default (first-registered) plane, an unregistered one
/// 501s.
async fn create(state: AppState, req: Request<Body>) -> Response {
    let body = match read_json_body(req, SMALL_JSON_BODY).await {
        Ok(b) => b,
        Err(res) => return *res,
    };
    let Some(body) = body else {
        return bad_request("Expected a JSON object body");
    };
    let ctx = state.entity_context();
    let plane = match body.get("kind").and_then(Value::as_str) {
        Some(kind) => match ctx.kinds.plane(kind) {
            Some(plane) => plane,
            None => return kind_not_registered(kind),
        },
        None => match ctx.kinds.default_plane() {
            Some(plane) => plane,
            None => return kind_not_registered(""),
        },
    };
    let init = body.get("indexDoc").cloned().unwrap_or(body);
    plane.create(&ctx, init).await
}

/// `GET /api/entities/{id}` — the owning plane's entity doc (first
/// non-404 wins; all-404 answers the last one). `DELETE` is
/// `entity.delete`.
pub async fn item(
    State(state): State<AppState>,
    method: Method,
    Path(id): Path<String>,
    req: Request<Body>,
) -> Response {
    let ctx = state.entity_context();
    match method {
        Method::GET => {
            let query = req.uri().query();
            let mut last = None;
            for plane in ctx.kinds.planes() {
                let res = plane.get(&ctx, &id, query).await;
                if res.status() != StatusCode::NOT_FOUND {
                    return res;
                }
                last = Some(res);
            }
            last.unwrap_or_else(not_found)
        }
        Method::DELETE => delete_entity(&ctx, &id, req.uri().query()).await,
        _ => not_found(),
    }
}

/// `POST /api/entities/{id}/ops` — submit `{kind, payload}`.
/// `entity.delete` (and the bare `delete` alias) is substrate-generic;
/// `<ns>.<verb>` dispatches to the plane owning `ns`; bare verbs fall
/// back to the default plane — legacy un-namespaced outbox rows.
pub async fn ops(
    State(state): State<AppState>,
    method: Method,
    Path(id): Path<String>,
    req: Request<Body>,
) -> Response {
    if method != Method::POST {
        return not_found();
    }
    let ctx = state.entity_context();
    let query = req.uri().query().map(str::to_string);
    let body = match read_json_body(req, SMALL_JSON_BODY).await {
        Ok(b) => b,
        Err(res) => return *res,
    };
    let Some(body) = body else {
        return bad_request("Expected a JSON object body");
    };
    let kind = body.get("kind").and_then(Value::as_str).unwrap_or_default();
    let payload = body.get("payload").cloned().unwrap_or_else(|| json!({}));

    if kind == "entity.delete" || kind == "delete" {
        return delete_entity(&ctx, &id, query.as_deref()).await;
    }
    // `entity.*` past `delete` is the substrate's own (reserved)
    // namespace — nothing else lives there yet.
    if kind.starts_with("entity.") {
        return unknown_op(kind);
    }
    match ctx.kinds.plane_for_op(kind) {
        Some(plane) => plane.op(&ctx, &id, kind, payload, query.as_deref()).await,
        None => unknown_op(kind),
    }
}

/// `GET /api/entities/{id}/snapshot` — kind-shaped snapshot bytes the
/// substrate moves opaquely; the owning plane produces them.
pub async fn snapshot_item(
    State(state): State<AppState>,
    method: Method,
    Path(id): Path<String>,
    req: Request<Body>,
) -> Response {
    if method != Method::GET {
        return not_found();
    }
    let ctx = state.entity_context();
    let query = req.uri().query();
    let mut last = None;
    for plane in ctx.kinds.planes() {
        let res = plane.export(&ctx, &id, query).await;
        if res.status() != StatusCode::NOT_FOUND {
            return res;
        }
        last = Some(res);
    }
    last.unwrap_or_else(not_found)
}

/// `POST /api/entities/snapshot` — import a snapshot on this node;
/// `body.kind` picks the plane (absent → the default plane).
pub async fn snapshot_collection(
    State(state): State<AppState>,
    method: Method,
    req: Request<Body>,
) -> Response {
    if method != Method::POST {
        return not_found();
    }
    let body = match read_json_body(req, MAX_JSON_BODY).await {
        Ok(b) => b,
        Err(res) => return *res,
    };
    let Some(body) = body else {
        return bad_request("Expected a JSON object body");
    };
    let ctx = state.entity_context();
    let kind_opt = body.get("kind").and_then(Value::as_str).or_else(|| {
        body.get("entity")
            .and_then(|e| e.get("kind"))
            .and_then(Value::as_str)
    });
    let plane = match kind_opt {
        Some(kind) => match ctx.kinds.plane(kind) {
            Some(plane) => plane,
            None => return kind_not_registered(kind),
        },
        None => match ctx.kinds.default_plane() {
            Some(plane) => plane,
            None => return kind_not_registered(""),
        },
    };
    plane.import(&ctx, body).await
}
