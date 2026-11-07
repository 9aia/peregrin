#![allow(clippy::unwrap_used, clippy::pedantic, clippy::missing_panics_doc)]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Method, Request, StatusCode};
use peregrin_http::entity_plane::EntityPlane;
use peregrin_http::{AppState, app};
use peregrin_node::CampaignEntityPlane;
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

fn mk_app() -> (Router, Arc<CampaignEntityPlane>) {
    let store = Arc::new(MemStore {
        sessions: Mutex::new(HashMap::new()),
    });
    let plane = ControlPlane::new(store, ControlPlaneOptions::default());
    let campaign_plane = Arc::new(CampaignEntityPlane::new());
    let state = AppState::new(Arc::new(LocalPlane(plane, None)))
        .with_kind(Arc::clone(&campaign_plane) as Arc<dyn EntityPlane>)
        .with_held_watch(Duration::ZERO);
    (app(state), campaign_plane)
}

async fn json_body(res: axum::response::Response) -> Value {
    let bytes = to_bytes(res.into_body(), 1024 * 1024).await.unwrap();
    serde_json::from_slice(&bytes).unwrap_or(Value::Null)
}

#[tokio::test]
async fn campaign_entity_create_list_and_get() {
    let (app, _) = mk_app();

    // 1. Create a campaign entity
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/entities")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "kind": "campaign",
                "indexDoc": {
                    "name": "ci-test"
                }
            })
            .to_string(),
        ))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let body = json_body(res).await;
    let id = body["id"].as_str().unwrap().to_string();
    assert_eq!(body["kind"], "campaign");
    assert_eq!(body["indexDoc"]["name"], "ci-test");
    assert_eq!(body["indexDoc"]["state"], "queued");
    assert_eq!(body["live"], false);
    assert_eq!(body["locked"], false);

    // 2. List entities
    let req = Request::builder()
        .method(Method::GET)
        .uri("/api/entities")
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let list = json_body(res).await;
    let arr = list["entities"].as_array().unwrap();
    assert!(arr.iter().any(|e| e["id"] == id));

    // 3. Get entity
    let req = Request::builder()
        .method(Method::GET)
        .uri(format!("/api/entities/{id}"))
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let doc = json_body(res).await;
    assert_eq!(doc["id"], id);
    assert_eq!(doc["indexDoc"]["state"], "queued");
}

#[tokio::test]
async fn campaign_entity_start_and_complete() {
    let (app, _) = mk_app();

    // Create entity
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/entities")
        .header("content-type", "application/json")
        .body(Body::from(json!({ "kind": "campaign" }).to_string()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let body = json_body(res).await;
    let id = body["id"].as_str().unwrap().to_string();

    // Start with quick campaign YAML
    let yaml = r#"
name: quick-run
nodes:
  - id: step1
    driver: noop
    on: [start]
    emits: [step1.done]
"#;
    let req = Request::builder()
        .method(Method::POST)
        .uri(format!("/api/entities/{id}/ops"))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "kind": "campaign.start",
                "payload": {
                    "campaign": { "yaml": yaml }
                }
            })
            .to_string(),
        ))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let op_res = json_body(res).await;
    assert_eq!(op_res["status"], "started");

    // Wait for run completion
    tokio::time::sleep(Duration::from_millis(150)).await;

    // Check entity state
    let req = Request::builder()
        .method(Method::GET)
        .uri(format!("/api/entities/{id}"))
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let doc = json_body(res).await;
    assert_eq!(doc["indexDoc"]["state"], "done");
    assert_eq!(doc["live"], false);
    assert!(doc["indexDoc"]["counts"]["events"].as_u64().unwrap() >= 2);
    assert!(doc["indexDoc"]["finishedAt"].is_string());
}

#[tokio::test]
async fn campaign_entity_cancel_flow() {
    let (app, _) = mk_app();

    // Create entity
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/entities")
        .header("content-type", "application/json")
        .body(Body::from(json!({ "kind": "campaign" }).to_string()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let id = json_body(res).await["id"].as_str().unwrap().to_string();

    // Start with delay
    let yaml = r#"
name: delay-run
nodes:
  - id: delay-step
    driver: delay
    on: [start]
    emits: [delay.done]
    params:
      duration: 5s
"#;
    let req = Request::builder()
        .method(Method::POST)
        .uri(format!("/api/entities/{id}/ops"))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "kind": "campaign.start",
                "payload": {
                    "campaign": { "yaml": yaml }
                }
            })
            .to_string(),
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // Cancel while running
    let req = Request::builder()
        .method(Method::POST)
        .uri(format!("/api/entities/{id}/ops"))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "kind": "campaign.cancel",
                "payload": { "reason": "user cancelled" }
            })
            .to_string(),
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // Wait briefly for cancel to land
    tokio::time::sleep(Duration::from_millis(100)).await;

    let req = Request::builder()
        .method(Method::GET)
        .uri(format!("/api/entities/{id}"))
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let doc = json_body(res).await;
    assert_eq!(doc["indexDoc"]["state"], "cancelled");
}

#[tokio::test]
async fn campaign_entity_signal_flow() {
    let (app, _) = mk_app();

    // Create entity
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/entities")
        .header("content-type", "application/json")
        .body(Body::from(json!({ "kind": "campaign" }).to_string()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let id = json_body(res).await["id"].as_str().unwrap().to_string();

    // Campaign waits for a custom signal event
    let yaml = r#"
name: signal-wait
nodes:
  - id: worker
    driver: noop
    on: [app.proceed]
    emits: [worker.done]
  - id: timer
    driver: delay
    on: [start]
    emits: [timer.done]
    params:
      duration: 5s
"#;
    let req = Request::builder()
        .method(Method::POST)
        .uri(format!("/api/entities/{id}/ops"))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "kind": "campaign.start",
                "payload": {
                    "campaign": { "yaml": yaml }
                }
            })
            .to_string(),
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // Send the signal
    let req = Request::builder()
        .method(Method::POST)
        .uri(format!("/api/entities/{id}/ops"))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "kind": "campaign.signal",
                "payload": {
                    "kind": "app.proceed",
                    "payload": { "token": "xyz" }
                }
            })
            .to_string(),
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // Cancel after signal processed
    tokio::time::sleep(Duration::from_millis(50)).await;
    let req = Request::builder()
        .method(Method::POST)
        .uri(format!("/api/entities/{id}/ops"))
        .header("content-type", "application/json")
        .body(Body::from(json!({ "kind": "campaign.cancel" }).to_string()))
        .unwrap();
    let _ = app.clone().oneshot(req).await.unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Check log contains both the signal and worker's reaction
    let req = Request::builder()
        .method(Method::GET)
        .uri(format!("/api/entities/{id}/snapshot"))
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let snap = json_body(res).await;
    let log = snap["log"].as_array().unwrap();
    assert!(log.iter().any(|e| e["type"] == "app.proceed"));
    assert!(
        log.iter()
            .any(|e| e["type"] == "node.done" && e["source"] == "worker")
    );
}

#[tokio::test]
async fn campaign_entity_snapshot_export_and_import() {
    let (app, _) = mk_app();

    // Create and complete a run
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/entities")
        .header("content-type", "application/json")
        .body(Body::from(json!({ "kind": "campaign" }).to_string()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let id = json_body(res).await["id"].as_str().unwrap().to_string();

    let yaml = r#"
name: export-test
nodes:
  - id: step
    driver: noop
    on: [start]
    emits: [step.done]
"#;
    let req = Request::builder()
        .method(Method::POST)
        .uri(format!("/api/entities/{id}/ops"))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "kind": "campaign.start",
                "payload": { "campaign": { "yaml": yaml } }
            })
            .to_string(),
        ))
        .unwrap();
    let _ = app.clone().oneshot(req).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Export snapshot
    let req = Request::builder()
        .method(Method::GET)
        .uri(format!("/api/entities/{id}/snapshot"))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let mut snapshot = json_body(res).await;
    assert_eq!(snapshot["format"], "campaign-snapshot/v1");
    assert_eq!(snapshot["entity"]["id"], id);
    assert!(!snapshot["log"].as_array().unwrap().is_empty());

    // Import under new id
    let new_id = format!("run-import-{}", uuid::Uuid::new_v4().simple());
    snapshot["entity"]["id"] = json!(new_id);

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/entities/snapshot")
        .header("content-type", "application/json")
        .body(Body::from(snapshot.to_string()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let imported = json_body(res).await;
    assert_eq!(imported["id"], new_id);
    assert_eq!(imported["kind"], "campaign");
    assert_eq!(imported["indexDoc"]["state"], "done");

    // Verify imported entity is accessible
    let req = Request::builder()
        .method(Method::GET)
        .uri(format!("/api/entities/{new_id}"))
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn campaign_entity_delete_flow() {
    let (app, _) = mk_app();

    // Create queued entity
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/entities")
        .header("content-type", "application/json")
        .body(Body::from(json!({ "kind": "campaign" }).to_string()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let id = json_body(res).await["id"].as_str().unwrap().to_string();

    // Delete queued entity
    let req = Request::builder()
        .method(Method::DELETE)
        .uri(format!("/api/entities/{id}"))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // Verify 404
    let req = Request::builder()
        .method(Method::GET)
        .uri(format!("/api/entities/{id}"))
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}
