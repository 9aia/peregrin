//! `POST /api/events/inject` — the authenticated typed-event ingest
//! into the campaign engine (`GET /api/events` is hypha-http's SSE
//! feed; this is the write half). External producers — the bupha CI
//! worker's `ci.job.completed` emit, forge check runners — POST a
//! `{type, payload}` body; the node's [`EventIngest`] sink fans it
//! out to every *running* `kind="campaign"` entity's ingress channel
//! (or the one named by `campaign`).
//!
//! Wire contract:
//!
//! - `POST /api/events/inject`, `content-type: application/json`,
//!   body ≤ 64 KiB ([`SMALL_JSON_BODY`], the entity-ops cap).
//! - Auth: the shared bearer gate — `Authorization: Bearer
//!   <PEREGRIN_TOKEN>` (or a paired `sepia_…` credential / `sepia_token`
//!   cookie) when a token is configured; open on tokenless loopback
//!   like every other `/api/*` route.
//! - Body: `{"type": "<event kind>", "payload": <any JSON>,
//!   "idempotencyKey"?: "<dedup key>", "source"?: "<label>",
//!   "campaign"?: "<entity id>"}` — `type` is required and non-empty
//!   (`kind` accepted as the `Event` wire alias); `payload` defaults
//!   to `{}`; `source` defaults to `"api"`; `idempotencyKey`
//!   (`idempotency_key` alias) dedups retries — a replayed key
//!   answers `{"ok": true, "deduplicated": true}` without
//!   re-delivering; `campaign` scopes the fan-out to one
//!   `kind="campaign"` entity (unknown id → 404, not running →
//!   `delivered: 0`).
//! - Responses: `200 {"ok": true, "delivered": <runs that accepted>}`
//!   — `delivered: 0` means the event was accepted but no campaign
//!   entity was running to see it; `400` malformed body; `401`
//!   bad/missing credential; `404` unknown scoped campaign (and
//!   wrong-method, per the router's fall-through convention);
//!   `413` over the body cap; `500`/`4xx` from the sink verbatim;
//!   `501` when the node has no campaign plane wired.

use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::{Method, StatusCode};
use axum::response::Response;
use serde_json::{Value, json};

use crate::AppState;
use crate::ingest::{IngestDelivery, IngestOutcome};
use crate::routes::{SMALL_JSON_BODY, json_response, not_found, read_json_body};

/// `POST /api/events/inject` — see the module docs for the contract.
pub async fn inject(State(state): State<AppState>, method: Method, req: Request<Body>) -> Response {
    if method != Method::POST {
        return not_found();
    }
    let Some(ingest) = &state.event_ingest else {
        return json_response(
            json!({ "error": "Event ingest is not configured on this server" }),
            StatusCode::NOT_IMPLEMENTED,
        );
    };
    let body = match read_json_body(req, SMALL_JSON_BODY).await {
        Ok(b) => b,
        Err(res) => return *res,
    };
    let bad_body = || {
        json_response(
            json!({ "error": "Expected a JSON object body" }),
            StatusCode::BAD_REQUEST,
        )
    };
    let Some(body) = body else {
        return bad_body();
    };
    let Some(obj) = body.as_object() else {
        return bad_body();
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
    match ingest.deliver(&delivery) {
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
