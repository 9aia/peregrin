#![cfg(feature = "bupha")]
#![allow(clippy::unwrap_used, clippy::pedantic, clippy::missing_panics_doc)]

//! `kind="ci-job"` end-to-end through the composed entity surface —
//! the plane `peregrin-node` registers under the `bupha` feature
//! (P-002, T-061): create *is* `ci.enqueue`, `ci.claim` CASes
//! `pending→running`, `ci.complete` lands the terminal state.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Method, Request, StatusCode};
use peregrin_http::{AppState, app};
use peregrin_node::ci_job_plane;
use peregrin_session::LocalPlane;
use sepia_core::storage::SessionRepository;
use sepia_core::{Session, StorageError};
use sepia_session::{ControlPlane, ControlPlaneOptions};
use serde_json::{Value, json};
use tower::ServiceExt;

struct MemStore {
    sessions: Mutex<HashMap<String, Session>>,
}

#[async_trait::async_trait]
impl SessionRepository for MemStore {
    async fn save(&self, session: &Session) -> Result<(), StorageError> {
        self.sessions
            .lock()
            .unwrap()
            .insert(session.id.clone(), session.clone());
        Ok(())
    }
    async fn get_by_id(
        &self,
        id: &str,
        _agent: Option<&str>,
    ) -> Result<Option<Session>, StorageError> {
        Ok(self.sessions.lock().unwrap().get(id).cloned())
    }
    async fn list(&self) -> Result<Vec<Session>, StorageError> {
        Ok(self.sessions.lock().unwrap().values().cloned().collect())
    }
    async fn delete(&self, id: &str) -> Result<(), StorageError> {
        self.sessions.lock().unwrap().remove(id);
        Ok(())
    }
    async fn has_session(&self, id: &str) -> Result<bool, StorageError> {
        Ok(self.sessions.lock().unwrap().contains_key(id))
    }
}

fn mk_app() -> Router {
    let store = Arc::new(MemStore {
        sessions: Mutex::new(HashMap::new()),
    });
    let plane = ControlPlane::new(store, ControlPlaneOptions::default());
    let state = AppState::new(Arc::new(LocalPlane(plane, None)))
        .with_kind(ci_job_plane(vec!["arch=arm64".to_string()], vec![]));
    app(state)
}

async fn json_body(res: axum::response::Response) -> Value {
    let bytes = to_bytes(res.into_body(), 1024 * 1024).await.unwrap();
    serde_json::from_slice(&bytes).unwrap_or(Value::Null)
}

/// `hypha-node-api`'s `url_encode` — ci-job ids carry `/`/`:`
/// (`ci:{key}:{sha}:{attempt}`), so entity paths need the segment
/// percent-encoded exactly like `HttpEntityApi` does.
fn url_encode(s: &str) -> String {
    use std::fmt::Write;
    s.chars().fold(String::new(), |mut out, c| {
        if c.is_ascii_alphanumeric() || "-_.~".contains(c) {
            out.push(c);
        } else {
            let _ = write!(out, "%{:02X}", c as u32);
        }
        out
    })
}

async fn op(app: &Router, id: &str, kind: &str, payload: Value) -> (StatusCode, Value) {
    let req = Request::builder()
        .method(Method::POST)
        .uri(format!("/api/entities/{}/ops", url_encode(id)))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({ "kind": kind, "payload": payload }).to_string(),
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    (status, json_body(res).await)
}

#[tokio::test]
async fn ci_job_enqueue_claim_complete_over_http() {
    let app = mk_app();

    // Create IS ci.enqueue — deterministic id `ci:{key}:{sha}:{attempt}`.
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/entities")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "kind": "ci-job",
                "indexDoc": {
                    "key": "9aia/bupha",
                    "sha": "0123abcd4567",
                    "ref": "refs/heads/main",
                    "sender": "luis"
                }
            })
            .to_string(),
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let body = json_body(res).await;
    let id = body["id"].as_str().unwrap().to_string();
    assert_eq!(id, "ci:9aia/bupha:0123abcd4567:1");
    assert_eq!(body["entity"]["kind"], "ci-job");
    assert_eq!(body["entity"]["indexDoc"]["state"], "pending");

    // Replay dedups on the deterministic id — no double-enqueue.
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/entities")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "kind": "ci-job",
                "indexDoc": { "key": "9aia/bupha", "sha": "0123abcd4567" }
            })
            .to_string(),
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(json_body(res).await["duplicate"], true);

    // `ci.claim` CASes pending→running and bumps the fence.
    let (status, body) = op(&app, &id, "ci.claim", json!({ "worker": "node-a/1" })).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["claimed"], true);
    assert_eq!(body["fence"], 1);

    // A second claim conflicts — the job is running.
    let (status, _) = op(&app, &id, "ci.claim", json!({ "worker": "node-b/2" })).await;
    assert_eq!(status, StatusCode::CONFLICT);

    // `ci.complete` with the fence lands the terminal state.
    let (status, body) = op(
        &app,
        &id,
        "ci.complete",
        json!({ "worker": "node-a/1", "fence": 1, "result": "success" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["state"], "success");

    // The entity list filters by kind.
    let req = Request::builder()
        .method(Method::GET)
        .uri("/api/entities?kind=ci-job")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let list = json_body(res).await;
    let arr = list["entities"].as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["id"], id);
    assert_eq!(arr[0]["indexDoc"]["state"], "success");
}

#[tokio::test]
async fn ci_job_runs_on_gate_uses_node_labels() {
    let app = mk_app();

    // runsOn the node doesn't advertise → create succeeds (the gate
    // is at claim time), claim conflicts.
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/entities")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "kind": "ci-job",
                "indexDoc": {
                    "key": "9aia/myco",
                    "sha": "fedcba987654",
                    "runsOn": ["arch=amd64"]
                }
            })
            .to_string(),
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let id = json_body(res).await["id"].as_str().unwrap().to_string();

    let (status, body) = op(&app, &id, "ci.claim", json!({ "worker": "node-a/1" })).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert!(body["error"].as_str().unwrap().contains("arch=amd64"));
}
