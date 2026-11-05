//! Entity snapshot tests — `snapshot_export` + `snapshot_import`
//! against canned nodes (the session IR routes they map onto today,
//! marked INTERIM until `/api/entities` lands).
#![allow(clippy::unwrap_used)]

use std::sync::Arc;

use peregrin_hub::credentials::{FileCredentials, NodeCredentials};
use peregrin_hub::sync_api::{HubNode, SyncEntityApi};
use peregrin_node_api::api::EntityApi;
use peregrin_sync::{NodeTarget, SyncEngine, SyncHandle, SyncOptions};
use serde_json::json;

fn file_creds() -> (tempfile::TempDir, Arc<dyn NodeCredentials>) {
    let dir = tempfile::tempdir().unwrap();
    let creds = Arc::new(FileCredentials::new(dir.path().join("credentials.json")));
    (dir, creds)
}

/// A canned-response node: records `"METHOD path"` + bodies; `fail`
/// maps path prefixes to a 500.
#[derive(Clone, Default)]
struct Canned {
    calls: Arc<std::sync::Mutex<Vec<String>>>,
    bodies: Arc<std::sync::Mutex<Vec<String>>>,
    fail: Arc<std::sync::Mutex<Vec<String>>>,
}

impl Canned {
    fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }

    fn bodies(&self) -> Vec<String> {
        self.bodies.lock().unwrap().clone()
    }
}

async fn canned(
    axum::extract::State(m): axum::extract::State<Canned>,
    req: axum::extract::Request,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    m.calls.lock().unwrap().push(format!("{method} {path}"));
    let bytes = axum::body::to_bytes(req.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap_or_default();
    let body_text = String::from_utf8_lossy(&bytes).into_owned();
    if !body_text.is_empty() {
        m.bodies
            .lock()
            .unwrap()
            .push(format!("{method} {path} {body_text}"));
    }
    if m.fail
        .lock()
        .unwrap()
        .iter()
        .any(|p| path.starts_with(p.as_str()))
    {
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(json!({"error":"node blew up"})),
        )
            .into_response();
    }
    let body = match (method.as_str(), path.as_str()) {
        ("GET", p) if p.ends_with("/snapshot") => json!({
            "session":{"id":"ir-1","title":"exported","cwd":"/w"}
        }),
        (_, "/api/entities/snapshot") => {
            json!({"id":"i1","kind":"sepia-session","agent":"devin","cwd":"/w"})
        }
        _ => json!({}),
    };
    axum::Json(body).into_response()
}

async fn serve(m: Canned) -> (Canned, String) {
    let app = axum::Router::new()
        .route("/api/{*rest}", axum::routing::any(canned))
        .with_state(m.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    (m, format!("http://{addr}"))
}

fn hub_node(id: &str, url: &str) -> HubNode {
    HubNode {
        id: id.into(),
        url: url.into(),
        token: None,
    }
}

fn api_for(nodes: Vec<HubNode>) -> (SyncEntityApi, peregrin_sync::ProjectionStore) {
    let projection = peregrin_sync::ProjectionStore::in_memory().unwrap();
    let outbox = peregrin_outbox::Outbox::in_memory().unwrap();
    let engine: SyncHandle = SyncEngine::spawn(
        projection.clone(),
        outbox,
        nodes.iter().map(|n| {
            NodeTarget::new(peregrin_sync::NodeRef {
                id: n.id.clone(),
                url: n.url.clone(),
                label: n.id.clone(),
            })
        }),
        SyncOptions::default(),
    );
    let (_dir, creds) = file_creds();
    (SyncEntityApi::new(engine, nodes, creds), projection)
}

/// Seed one `sepia-session` entity in the projection.
fn seed_entity(projection: &peregrin_sync::ProjectionStore, node: &str, id: &str) {
    projection
        .upsert_entity(
            node,
            &json!({
                "id": id, "kind": "sepia-session",
                "indexDoc": {"id": id, "title": "T", "cwd": "/w", "agent": "devin"},
                "updatedAt": "2024-01-01T00:00:00Z",
            }),
        )
        .unwrap();
}

#[tokio::test]
async fn snapshot_export_pulls_the_entity_doc() {
    let (mock, url) = serve(Canned::default()).await;
    let (api, projection) = api_for(vec![hub_node("n1", &url)]);
    seed_entity(&projection, "n1", "s1");
    let doc = api.snapshot_export("s1").await.unwrap();
    assert_eq!(doc["session"]["id"], "ir-1");
    assert!(
        mock.calls()
            .iter()
            .any(|c| c == "GET /api/entities/s1/snapshot"),
        "calls: {:?}",
        mock.calls()
    );
}

#[tokio::test]
async fn snapshot_import_posts_the_snapshot_body() {
    let (mock, url) = serve(Canned::default()).await;
    let (api, _projection) = api_for(vec![hub_node("n1", &url)]);
    let snapshot = json!({"agent":"devin","session":{"id":"ir-1","title":"T"}});
    let entity = api.snapshot_import(&snapshot, Some("n1")).await.unwrap();
    assert_eq!(entity.id, "i1");
    assert_eq!(entity.kind, "sepia-session");
    assert!(
        mock.bodies()
            .iter()
            .any(|b| b.starts_with("POST /api/entities/snapshot") && b.contains("ir-1")),
        "bodies: {:?}",
        mock.bodies()
    );
}

#[tokio::test]
async fn snapshot_export_surfaces_node_errors() {
    let mock = Canned::default();
    mock.fail
        .lock()
        .unwrap()
        .push("/api/entities/s1/snapshot".into());
    let (_mock, url) = serve(mock).await;
    let (api, projection) = api_for(vec![hub_node("n1", &url)]);
    seed_entity(&projection, "n1", "s1");
    let err = api.snapshot_export("s1").await.unwrap_err();
    assert!(err.contains("node blew up"), "{err}");
}

#[tokio::test]
async fn snapshot_export_unknown_entity_errors() {
    let (_mock, url) = serve(Canned::default()).await;
    let (api, _projection) = api_for(vec![hub_node("n1", &url)]);
    let err = api.snapshot_export("ghost").await.unwrap_err();
    assert!(err.contains("Entity not found"), "{err}");
}
