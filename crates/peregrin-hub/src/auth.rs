//! Client → hub auth — the hub's own token surface. Node bearer tokens
//! stay server-side (`PEREGRIN_NODES` / `PEREGRIN_NODE_TOKEN`); this gate
//! covers everything the hub itself serves.
//!
//! When `PEREGRIN_HUB_TOKEN` is set, every route requires it:
//! `Authorization: Bearer` or a `?token=` query (EventSource can't set
//! headers, so SSE clients ride the query). Failures get the JSON 401 —
//! API-shaped errors, never a redirect.

use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::HubState;

/// `url.searchParams.get(key)` — form-decoded.
fn query_param(query: Option<&str>, key: &str) -> Option<String> {
    query.and_then(|q| {
        form_urlencoded::parse(q.as_bytes())
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.into_owned())
    })
}

fn unauthorized() -> Response {
    let mut res = (
        StatusCode::UNAUTHORIZED,
        axum::Json(serde_json::json!({ "error": "unauthorized" })),
    )
        .into_response();
    res.headers_mut()
        .insert(header::WWW_AUTHENTICATE, HeaderValue::from_static("Bearer"));
    res
}

/// The client gate — middleware over the whole router. No configured
/// token (loopback-only deployments) means everything passes.
pub async fn auth_gate(State(state): State<HubState>, req: Request<Body>, next: Next) -> Response {
    let Some(expected) = state.hub_token.as_deref().filter(|t| !t.is_empty()) else {
        return next.run(req).await;
    };
    let bearer_ok = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .is_some_and(|p| p == expected);
    if bearer_ok || query_param(req.uri().query(), "token").is_some_and(|t| t == expected) {
        return next.run(req).await;
    }
    unauthorized()
}
