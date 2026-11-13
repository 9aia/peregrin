//! `POST /api/events/inject` on the `campaign serve` shared listener —
//! the campaign-serve counterpart of `peregrin serve`'s entity-plane
//! ingest route (`peregrin_http::routes::events`). External producers
//! (the bupha CI worker's `ci.job.completed` emit, forge check
//! runners, ops tooling) POST a `{type, payload}` body; the route
//! fans it out to every hosted campaign's *trigger* injector — the
//! same channel `github-webhook` deliveries and `Served.injectors`
//! pushes use — so an injected event starts (or joins) a supervised
//! run exactly like a webhook trigger does.
//!
//! The route shares the ONE demux socket (`--listen`/`--path` govern
//! `/hook`; this route is fixed at `/api/events/inject`) rather than
//! binding a second port, so the public listener surface stays a
//! single URL. Because that socket is often tunnel-exposed, the
//! route is bearer-gated: `Authorization: Bearer <PEREGRIN_TOKEN>`
//! when the env var is set (the same credential `peregrin serve`
//! nodes and the bupha worker's `node_token_env` resolve); an unset
//! token leaves the route open — the dev-loopback convention every
//! `/api/*` surface follows.
//!
//! Wire contract mirrors `peregrin serve`'s ingest:
//!
//! - `POST /api/events/inject`, `content-type: application/json`,
//!   body ≤ 64 KiB.
//! - Body: `{"type": "<event kind>", "payload": <any JSON>,
//!   "idempotencyKey"?: "<dedup key>", "source"?: "<label>",
//!   "campaign"?: "<hosted campaign name>"}` — `type` required
//!   (`kind` alias accepted); `payload` defaults `{}`; `source`
//!   defaults `"api"`; `idempotencyKey` (`idempotency_key` alias)
//!   dedups retries against a bounded ring; `campaign` scopes the
//!   fan-out to one hosted campaign (unknown name → 404).
//! - Responses: `200 {"ok": true, "delivered": <injectors that
//!   accepted>}` — `0` means accepted but every targeted run
//!   channel was already closed; `400` malformed; `401` bad/missing
//!   credential; `404` unknown scoped campaign; `405` non-POST.
//!
//! The route exists only when the shared listener binds — i.e. at
//! least one hosted campaign declares a `github-webhook` node. A
//! fleet with no webhook campaigns has no socket for producers to
//! reach (the `Served.injectors` handles remain the in-process feed).

use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, PoisonError, RwLock};
use std::thread::JoinHandle;
use std::time::Duration;

use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::any;
use axum::{Json, Router};
use peregrin_core::Event;
use peregrin_engine::Injector;
use peregrin_http::{DeliverError, EventIngest, IngestDelivery, IngestOutcome};
use serde_json::{Value, json};

/// The fixed route path on the shared listener.
pub const INJECT_PATH: &str = "/api/events/inject";

/// Same cap as `peregrin serve`'s ingest route (`SMALL_JSON_BODY`).
const MAX_BODY: usize = 64 * 1024;

/// Hosted campaign name → its trigger injector, populated by
/// [`serve_campaigns`](crate::serve::serve_campaigns) as supervisors
/// spawn. Shared with the inject route's deliver sink — requests
/// arriving before the first supervise lands deliver `0`.
pub type InjectTargets = Arc<RwLock<Vec<(String, Injector)>>>;

/// The axum state the inject route carries — the dedup+fan-out
/// backend plus the expected bearer credential (`None` = open, the
/// tokenless-dev convention).
pub struct InjectState {
    ingest: EventIngest,
    token: Option<String>,
}

/// `token_matches` parity with `peregrin_http::auth` — length check
/// first so a mismatch can't leak the expected token's length, then
/// a constant-time fold.
fn token_matches(provided: &str, expected: &str) -> bool {
    if provided.len() != expected.len() {
        return false;
    }
    provided
        .bytes()
        .zip(expected.bytes())
        .fold(0u8, |acc, (a, b)| acc | (a ^ b))
        == 0
}

/// The bearer gate — `Authorization: Bearer <token>`; no configured
/// token means everything is allowed (the `/api/*` convention).
fn authorized(state: &InjectState, headers: &HeaderMap) -> bool {
    let Some(expected) = state.token.as_deref().filter(|t| !t.is_empty()) else {
        return true;
    };
    let provided = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));
    provided.is_some_and(|p| token_matches(p, expected))
}

fn json_response(body: Value, status: StatusCode) -> Response {
    (status, Json(body)).into_response()
}

/// The inject route's deliver sink — fans one validated delivery
/// out to every registered campaign injector (or the one `campaign`
/// scopes to). Returns the count of injectors that accepted; a
/// scoped name that isn't hosted is a 404 [`DeliverError`], matching
/// `peregrin serve`'s semantics.
fn deliver_to_campaigns(targets: InjectTargets) -> peregrin_http::EventDeliver {
    Arc::new(move |delivery: &IngestDelivery| {
        let map = targets.read().unwrap_or_else(PoisonError::into_inner);
        if let Some(scope) = &delivery.campaign {
            if !map.iter().any(|(name, _)| name == scope) {
                return Err(DeliverError {
                    status: StatusCode::NOT_FOUND,
                    message: format!("campaign '{scope}' not found"),
                });
            }
        }
        let event = Event::new(&delivery.kind, delivery.payload.clone(), &delivery.source);
        let mut delivered = 0usize;
        for (name, injector) in map.iter() {
            if delivery.campaign.as_deref().is_some_and(|s| s != name) {
                continue;
            }
            // A closed channel means that campaign's supervisor
            // already exited — counts as not-delivered, never `Err`
            // (the node ingest's per-run drop accounting).
            if injector.inject(event.clone()).is_ok() {
                delivered += 1;
            }
        }
        Ok(delivered)
    })
}

/// `POST /api/events/inject` — see the module docs for the contract.
async fn inject(
    State(state): State<Arc<InjectState>>,
    method: Method,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if method != Method::POST {
        return json_response(json!({ "error": "not found" }), StatusCode::NOT_FOUND);
    }
    if !authorized(&state, &headers) {
        return json_response(json!({ "error": "unauthorized" }), StatusCode::UNAUTHORIZED);
    }
    if body.len() > MAX_BODY {
        return json_response(
            json!({ "error": "request body too large" }),
            StatusCode::PAYLOAD_TOO_LARGE,
        );
    }
    let parsed: Result<Value, _> = serde_json::from_slice(&body);
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
    let text = |key: &str| {
        obj.get(key)
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
    };
    // `type` is canonical; `kind` is the `Event` wire's alias.
    let Some(kind) = text("type").or_else(|| text("kind")) else {
        return json_response(
            json!({ "error": "missing 'type' — the event kind to inject" }),
            StatusCode::BAD_REQUEST,
        );
    };
    let delivery = IngestDelivery {
        kind: kind.to_string(),
        payload: obj.get("payload").cloned().unwrap_or_else(|| json!({})),
        source: text("source").unwrap_or("api").to_string(),
        campaign: text("campaign").map(str::to_string),
        idempotency_key: text("idempotencyKey")
            .or_else(|| text("idempotency_key"))
            .map(str::to_string),
    };
    match state.ingest.deliver(&delivery) {
        Ok(IngestOutcome::Delivered(delivered)) => json_response(
            json!({ "ok": true, "delivered": delivered }),
            StatusCode::OK,
        ),
        Ok(IngestOutcome::Duplicate) => json_response(
            json!({ "ok": true, "deduplicated": true, "delivered": 0 }),
            StatusCode::OK,
        ),
        Err(error) => json_response(json!({ "error": error.message }), error.status),
    }
}

/// The inject route as an axum [`Router`] — `.merge()` it onto the
/// shared webhook app so both live on the one listener socket.
/// `targets` starts empty; [`serve_campaigns`](crate::serve::serve_campaigns)
/// fills it as supervisors spawn. `token` is the expected bearer —
/// read from `PEREGRIN_TOKEN` by the caller (`None`/empty = open).
pub fn router(targets: InjectTargets, token: Option<String>) -> Router {
    let state = Arc::new(InjectState {
        ingest: EventIngest::new(deliver_to_campaigns(targets)),
        token,
    });
    Router::new()
        .route(INJECT_PATH, any(inject))
        .layer(axum::extract::DefaultBodyLimit::max(MAX_BODY))
        .with_state(state)
}

/// Bind `listen` and spawn the axum serve thread for the merged
/// router — the `campaign serve` counterpart of the demux's private
/// `serve_thread`, minus the route table (the router is already
/// built). Graceful shutdown polls `shutdown` every 100 ms; the
/// returned [`JoinHandle`] lets [`Served::join`](crate::serve::Served::join)
/// collect the thread.
///
/// # Errors
/// Bind/`local_addr` failures as displayable messages (the same
/// surface [`SharedWebhook::serve`](peregrin_driver_github_webhook::SharedWebhook::serve)
/// reports).
pub fn serve_listener(
    listen: &str,
    router: Router,
    shutdown: Arc<AtomicBool>,
) -> Result<(SocketAddr, JoinHandle<()>), String> {
    let listener = std::net::TcpListener::bind(listen)
        .map_err(|e| format!("campaign serve listener failed to bind {listen}: {e}"))?;
    let addr = listener
        .local_addr()
        .map_err(|e| format!("campaign serve listener local_addr: {e}"))?;
    let _ = listener.set_nonblocking(true);
    let thread = std::thread::spawn(move || {
        let runtime = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(e) => {
                eprintln!("campaign serve listener: runtime: {e}");
                return;
            }
        };
        runtime.block_on(async move {
            let listener = match tokio::net::TcpListener::from_std(listener) {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("campaign serve listener: from_std: {e}");
                    return;
                }
            };
            if let Err(e) = axum::serve(listener, router)
                .with_graceful_shutdown(async move {
                    while !shutdown.load(Ordering::Relaxed) {
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }
                })
                .await
            {
                eprintln!("campaign serve listener: {e}");
            }
        });
        runtime.shutdown_background();
    });
    Ok((addr, thread))
}

/// Campaign name → injector registry for tests — a pre-filled
/// [`InjectTargets`].
#[cfg(test)]
pub fn targets_of(entries: Vec<(String, Injector)>) -> InjectTargets {
    Arc::new(RwLock::new(entries))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    fn state_with(targets: InjectTargets, token: Option<&str>) -> Arc<InjectState> {
        Arc::new(InjectState {
            ingest: EventIngest::new(deliver_to_campaigns(targets)),
            token: token.map(str::to_string),
        })
    }

    /// Drain one event off an [`Ingress`] — parks until it lands.
    fn take(ingress: &peregrin_engine::Ingress) -> Event {
        let mut out = Vec::new();
        assert_eq!(
            ingress.wait(None, &mut out),
            peregrin_engine::WaitOutcome::Event
        );
        out.pop().unwrap()
    }

    /// Assert no event is queued within a short window.
    fn assert_quiet(ingress: &peregrin_engine::Ingress) {
        let mut out = Vec::new();
        assert_eq!(
            ingress.wait(Some(Duration::from_millis(50)), &mut out),
            peregrin_engine::WaitOutcome::Timeout
        );
        assert_eq!(out.len(), 0);
    }

    async fn call(
        state: Arc<InjectState>,
        method: Method,
        body: Option<Value>,
        auth: Option<&str>,
    ) -> (StatusCode, Value) {
        let mut req = Request::builder().method(method).uri(INJECT_PATH);
        if let Some(t) = auth {
            req = req.header("authorization", format!("Bearer {t}"));
        }
        let req = match body {
            Some(b) => req
                .header("content-type", "application/json")
                .body(Body::from(b.to_string()))
                .unwrap(),
            None => req.body(Body::empty()).unwrap(),
        };
        let app = Router::new()
            .route(INJECT_PATH, any(inject))
            .with_state(state);
        let res = app.oneshot(req).await.unwrap();
        let status = res.status();
        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap();
        let value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        (status, value)
    }

    #[tokio::test]
    async fn fans_out_to_all_campaigns() {
        let (inj_a, ing_a) = peregrin_engine::ingress();
        let (inj_b, ing_b) = peregrin_engine::ingress();
        let targets = targets_of(vec![("a".into(), inj_a), ("b".into(), inj_b)]);
        let state = state_with(targets, None);
        let (status, body) = call(
            state,
            Method::POST,
            Some(json!({"type": "ci.job.completed", "payload": {"sha": "abc"}})),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["delivered"], 2);
        let ev = take(&ing_a);
        assert_eq!(ev.kind, "ci.job.completed");
        assert_eq!(ev.payload["sha"], "abc");
        assert_eq!(take(&ing_b).kind, "ci.job.completed");
    }

    #[tokio::test]
    async fn campaign_scope_filters_and_404s_unknown() {
        let (inj_a, ing_a) = peregrin_engine::ingress();
        let (inj_b, ing_b) = peregrin_engine::ingress();
        let targets = targets_of(vec![("a".into(), inj_a), ("b".into(), inj_b)]);
        let state = state_with(targets, None);
        let (status, body) = call(
            state.clone(),
            Method::POST,
            Some(json!({"type": "k", "campaign": "a"})),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["delivered"], 1);
        assert_eq!(take(&ing_a).kind, "k");
        assert_quiet(&ing_b);

        let (status, _) = call(
            state,
            Method::POST,
            Some(json!({"type": "k", "campaign": "ghost"})),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn idempotency_key_dedups() {
        let (inj, _ing) = peregrin_engine::ingress();
        let targets = targets_of(vec![("a".into(), inj)]);
        let state = state_with(targets, None);
        let body = json!({"type": "k", "idempotencyKey": "job-1:success"});
        let (s1, b1) = call(state.clone(), Method::POST, Some(body.clone()), None).await;
        let (s2, b2) = call(state, Method::POST, Some(body), None).await;
        assert_eq!(s1, StatusCode::OK);
        assert_eq!(b1["delivered"], 1);
        assert_eq!(s2, StatusCode::OK);
        assert_eq!(b2["deduplicated"], true);
    }

    #[tokio::test]
    async fn bearer_gate() {
        let (inj, _ing) = peregrin_engine::ingress();
        let targets = targets_of(vec![("a".into(), inj)]);
        let state = state_with(targets, Some("sekrit"));
        let body = json!({"type": "k"});
        let (status, _) = call(state.clone(), Method::POST, Some(body.clone()), None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        let (status, _) = call(
            state.clone(),
            Method::POST,
            Some(body.clone()),
            Some("wrong"),
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        let (status, b) = call(state, Method::POST, Some(body), Some("sekrit")).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(b["delivered"], 1);
    }

    #[tokio::test]
    async fn rejects_bad_bodies() {
        let (inj, _ing) = peregrin_engine::ingress();
        let targets = targets_of(vec![("a".into(), inj)]);
        let state = state_with(targets, None);
        let (status, _) = call(
            state.clone(),
            Method::POST,
            Some(json!({"payload": {}})),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        let (status, _) = call(state.clone(), Method::GET, None, None).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        // Non-object JSON.
        let app = Router::new()
            .route(INJECT_PATH, any(inject))
            .with_state(state);
        let req = Request::builder()
            .method(Method::POST)
            .uri(INJECT_PATH)
            .body(Body::from("[1,2]"))
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn token_matches_is_exact() {
        assert!(token_matches("abc", "abc"));
        assert!(!token_matches("abc", "abd"));
        assert!(!token_matches("abc", "abcd"));
        assert!(!token_matches("abcd", "abc"));
    }
}
