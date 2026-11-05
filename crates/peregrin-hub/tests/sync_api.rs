//! `SyncEntityApi` — merged entity index, op submits + outbox, lock
//! arbitration, pairing, node-level verbs. Canned nodes record calls;
//! the projection is seeded in-memory.
#![allow(clippy::unwrap_used)]

use std::collections::BTreeMap;
use std::sync::Arc;

use peregrin_hub::credentials::{FileCredentials, NodeCredentials};
use peregrin_hub::sync_api::{HubNode, SyncEntityApi, parse_nodes, resolve_node_tokens};
use peregrin_node_api::api::EntityApi;
use peregrin_node_api::dto::EntityOp;
use peregrin_sync::{NodeTarget, SyncEngine, SyncHandle, SyncOptions};
use serde_json::json;

fn file_creds() -> (tempfile::TempDir, Arc<dyn NodeCredentials>) {
    let dir = tempfile::tempdir().unwrap();
    let creds = Arc::new(FileCredentials::new(dir.path().join("credentials.json")));
    (dir, creds)
}

fn engine_with(
    nodes: &[HubNode],
) -> (
    SyncHandle,
    peregrin_sync::ProjectionStore,
    peregrin_outbox::Outbox,
) {
    let projection = peregrin_sync::ProjectionStore::in_memory().unwrap();
    let outbox = peregrin_outbox::Outbox::in_memory().unwrap();
    let targets = nodes.iter().map(|n| {
        NodeTarget::new(peregrin_sync::NodeRef {
            id: n.id.clone(),
            url: n.url.clone(),
            label: n.id.clone(),
        })
    });
    (
        SyncEngine::spawn(
            projection.clone(),
            outbox.clone(),
            targets,
            SyncOptions::default(),
        ),
        projection,
        outbox,
    )
}

fn hub_node(id: &str, url: &str) -> HubNode {
    HubNode {
        id: id.into(),
        url: url.into(),
        token: None,
    }
}

/// One `sepia-session` entity doc in the projection.
fn seed_entity(projection: &peregrin_sync::ProjectionStore, node: &str, id: &str, title: &str) {
    projection
        .upsert_entity(
            node,
            &json!({
                "id": id, "kind": "sepia-session",
                "indexDoc": {"id": id, "title": title, "cwd": "/w", "agent": "devin"},
                "updatedAt": "2024-01-01T00:00:00Z",
            }),
        )
        .unwrap();
}

/* ---- full-surface mock node ------------------------------------------*/

/// A canned-response node: records `"METHOD path"` + bodies; `fail`
/// maps path prefixes to a 500, `blank` maps them to `200 {}`.
#[derive(Clone, Default)]
struct Canned {
    calls: Arc<std::sync::Mutex<Vec<String>>>,
    bodies: Arc<std::sync::Mutex<Vec<String>>>,
    fail: Arc<std::sync::Mutex<Vec<String>>>,
    blank: Arc<std::sync::Mutex<Vec<String>>>,
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
    let key = format!("{method} {path}");
    if m.fail
        .lock()
        .unwrap()
        .iter()
        .any(|p| key.starts_with(p.as_str()) || path.starts_with(p.as_str()))
    {
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(json!({"error":"node blew up"})),
        )
            .into_response();
    }
    if m.blank
        .lock()
        .unwrap()
        .iter()
        .any(|p| path.starts_with(p.as_str()))
    {
        return axum::Json(json!({})).into_response();
    }
    let body = match (method.as_str(), path.as_str()) {
        ("GET", "/api/node") => json!({
            "id":"n1","name":"node-one","version":"0.1","protocol":1,
            "capabilities":["import"],
        }),
        ("PATCH", "/api/node") => json!({
            "id":"n1","name":"renamed","version":"0.1","protocol":1,
        }),
        ("GET", "/api/config") => json!({"config":{"theme":"dark","k":1}}),
        ("GET", "/api/fs") => json!({"dirs":["/a","/b"]}),
        ("POST", "/api/sessions") => json!({"id":"new-1","capabilities":{}}),
        ("GET", "/api/push/vapid") => json!({"publicKey":"pk-1"}),
        ("POST", "/api/pair") => json!({"token":"sepia_minted"}),
        ("DELETE", _) => json!({}),
        (_, "/api/sessions/import") => {
            json!({"id":"i1","kind":"sepia-session","agent":"devin"})
        }
        _ => json!({}),
    };
    axum::Json(body).into_response()
}

async fn serve_canned(m: Canned) -> (Canned, String) {
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

fn api_against(nodes: Vec<HubNode>) -> (SyncEntityApi, peregrin_sync::ProjectionStore) {
    let (engine, projection, _outbox) = engine_with(&nodes);
    let (_dir, creds) = file_creds();
    (SyncEntityApi::new(engine, nodes, creds), projection)
}

/* ---- registration / tokens -------------------------------------------*/

#[test]
fn parses_node_lists() {
    let nodes = parse_nodes("tower=http://a:1@tok; edge = http://b:2/ ");
    assert_eq!(nodes.len(), 2);
    assert_eq!(nodes[0].id, "tower");
    assert_eq!(nodes[0].url, "http://a:1");
    assert_eq!(nodes[0].token.as_deref(), Some("tok"));
    assert_eq!(nodes[1].id, "edge");
    assert_eq!(nodes[1].url, "http://b:2");
    assert!(nodes[1].token.is_none());
    assert!(parse_nodes("").is_empty());
    assert!(parse_nodes("garbage").is_empty());
}

#[test]
fn resolve_node_tokens_resolves_seeds_and_never_clobbers() {
    struct FakeCreds {
        map: std::sync::Mutex<BTreeMap<String, String>>,
    }
    impl NodeCredentials for FakeCreds {
        fn get(&self, id: &str) -> Result<Option<String>, String> {
            Ok(self.map.lock().unwrap().get(id).cloned())
        }
        fn set(&self, id: &str, t: &str) -> Result<(), String> {
            self.map.lock().unwrap().insert(id.into(), t.into());
            Ok(())
        }
        fn delete(&self, _id: &str) -> Result<(), String> {
            Ok(())
        }
        fn backend(&self) -> &'static str {
            "fake"
        }
    }
    let creds = FakeCreds {
        map: std::sync::Mutex::new(BTreeMap::from([("stored".into(), "cred-tok".into())])),
    };
    let mut nodes = parse_nodes("stored=http://a; env=http://b@env-tok; none=http://c");
    let (resolved, seeded) = resolve_node_tokens(&mut nodes, &creds);
    assert_eq!((resolved, seeded), (1, 1));
    assert_eq!(nodes[0].token.as_deref(), Some("cred-tok"));
    assert_eq!(nodes[1].token.as_deref(), Some("env-tok"));
    assert!(nodes[2].token.is_none());
    assert_eq!(creds.get("env").unwrap(), Some("env-tok".into()));
}

/* ---- the entity surface ----------------------------------------------*/

#[tokio::test]
async fn list_reads_the_projection_across_nodes() {
    let (api, projection) = api_against(vec![
        hub_node("n1", "http://127.0.0.1:1"),
        hub_node("n2", "http://127.0.0.1:2"),
    ]);
    seed_entity(&projection, "n1", "s1", "Alpha");
    seed_entity(&projection, "n2", "s2", "Beta");
    projection
        .upsert_entity(
            "n2",
            &json!({"id":"c1","kind":"campaign","indexDoc":{"title":"sweep"},"updatedAt":"2024-01-01T00:00:00Z"}),
        )
        .unwrap();
    let all = api.list_entities(None).await.unwrap();
    assert_eq!(all.len(), 3);
    let sessions = api.list_entities(Some("sepia-session")).await.unwrap();
    assert_eq!(sessions.len(), 2);
    assert!(sessions.iter().all(|e| e.kind == "sepia-session"));
    let titles: Vec<&str> = sessions
        .iter()
        .map(|e| e.index_doc["title"].as_str().unwrap_or_default())
        .collect();
    assert!(
        titles.contains(&"Alpha") && titles.contains(&"Beta"),
        "{titles:?}"
    );
    let campaigns = api.list_entities(Some("campaign")).await.unwrap();
    assert_eq!(campaigns.len(), 1);
}

#[tokio::test]
async fn entity_resolves_across_nodes() {
    let (api, projection) = api_against(vec![
        hub_node("n1", "http://127.0.0.1:1"),
        hub_node("n2", "http://127.0.0.1:2"),
    ]);
    seed_entity(&projection, "n2", "s1", "Beta");
    let e = api.entity("s1").await.unwrap();
    assert_eq!(e.node.as_deref(), Some("n2"));
    assert_eq!(e.index_doc["title"], "Beta");
    assert!(
        api.entity("ghost")
            .await
            .unwrap_err()
            .contains("Entity not found")
    );
}

#[tokio::test]
async fn entity_index_groups_by_node() {
    let (api, projection) = api_against(vec![hub_node("n1", "http://127.0.0.1:1")]);
    projection
        .upsert_node(&peregrin_sync::NodeRef {
            id: "n1".into(),
            url: "http://127.0.0.1:1".into(),
            label: "n1".into(),
        })
        .unwrap();
    seed_entity(&projection, "n1", "s1", "Alpha");
    let index = api.entity_index().await.unwrap();
    assert_eq!(index.len(), 1);
    assert_eq!(index[0].node, "n1");
    assert_eq!(index[0].entities.len(), 1);
}

#[tokio::test]
async fn submit_op_goes_direct_when_up_queue_when_down() {
    let (mock, url) = serve_canned(Canned::default()).await;
    let (api, projection) = api_against(vec![hub_node("n1", &url)]);
    seed_entity(&projection, "n1", "s1", "Alpha");
    // Node is registered but has never synced → status "unknown"/down →
    // a `sepia.prompt` op queues instead of posting.
    api.submit_op(
        "s1",
        &EntityOp {
            kind: "sepia.prompt".into(),
            payload: json!({"text":"hi"}),
        },
    )
    .await
    .unwrap();
    let pending = api.pending_ops().await.unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].kind, "sepia.prompt");
    assert_eq!(pending[0].entity_id, "s1");
    assert_eq!(pending[0].status, "queued");
    assert!(
        mock.calls().iter().all(|c| !c.contains("prompt")),
        "queued op must not post — calls: {:?}",
        mock.calls()
    );
}

#[tokio::test]
async fn gated_op_rejects_a_foreign_hold() {
    let (api, projection) = api_against(vec![hub_node("n1", "http://127.0.0.1:1")]);
    // A locked entity — held by someone else.
    projection
        .upsert_entity(
            "n1",
            &json!({
                "id":"s1","kind":"sepia-session","locked":true,
                "indexDoc":{"id":"s1","title":"Held","locked":true,"lockHolderPid":4321},
                "updatedAt":"2024-01-01T00:00:00Z",
            }),
        )
        .unwrap();
    let err = api
        .submit_op(
            "s1",
            &EntityOp {
                kind: "sepia.prompt".into(),
                payload: json!({"text":"hi"}),
            },
        )
        .await
        .unwrap_err();
    assert!(err.contains("held by pid:4321"), "{err}");
    // Meta ops aren't gated — the overlay is last-writer-wins anyway.
    api.submit_op(
        "s1",
        &EntityOp {
            kind: "sepia.meta.patch".into(),
            payload: json!({"pinned":true}),
        },
    )
    .await
    .unwrap();
    let pending = api.pending_ops().await.unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].kind, "sepia.meta.patch");
}

#[tokio::test]
async fn delete_entity_submits_the_delete_op() {
    let (api, projection) = api_against(vec![hub_node("n1", "http://127.0.0.1:1")]);
    seed_entity(&projection, "n1", "s1", "Alpha");
    api.delete_entity("s1").await.unwrap();
    let pending = api.pending_ops().await.unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].kind, "entity.delete");
}

#[tokio::test]
async fn create_entity_posts_to_the_scoped_node() {
    let (mock, url) = serve_canned(Canned::default()).await;
    let (api, _projection) = api_against(vec![hub_node("n1", &url)]);
    let res = api
        .create_entity("sepia-session", &json!({"cwd":"/work"}), Some("n1"))
        .await
        .unwrap();
    assert_eq!(res.id, "new-1");
    assert!(
        mock.bodies()
            .iter()
            .any(|b| b.starts_with("POST /api/sessions") && b.contains("/work")),
        "bodies: {:?}",
        mock.bodies()
    );
    // An unregistered kind errors without posting.
    assert!(
        api.create_entity("k8s-pod", &json!({}), Some("n1"))
            .await
            .unwrap_err()
            .contains("not registered")
    );
}

/* ---- node-level verbs -------------------------------------------------*/

#[tokio::test]
async fn node_verbs_sweep() {
    let (_mock, url) = serve_canned(Canned::default()).await;
    let (api, _projection) = api_against(vec![hub_node("n1", &url)]);
    let info = api.node_info().await.unwrap();
    assert_eq!(info.id, "n1");
    let renamed = api.rename_node("renamed", Some("n1")).await.unwrap();
    assert_eq!(renamed.name, "renamed");
    let config = api.get_config().await.unwrap();
    assert_eq!(config["theme"], "dark");
    api.set_config("theme", &json!("light")).await.unwrap();
    let dirs = api.fs_dirs("/", Some("n1".into())).await.unwrap();
    assert_eq!(dirs, vec!["/a", "/b"]);
    assert_eq!(api.push_vapid().await.unwrap(), "pk-1");
    let status = api.node_status().await.unwrap();
    assert_eq!(status.len(), 1);
    assert_eq!(status[0].id, "n1");
    // Unconfigured: the row exists with status unknown — the node was
    // registered but the (faked) sync loop hasn't marked it.
    let _ = api
        .push_subscribe(&peregrin_node_api::dto::PushSubscriptionDto {
            endpoint: "https://push/e".into(),
            keys: peregrin_node_api::dto::PushKeysDto {
                auth: "a".into(),
                p256dh: "p".into(),
            },
            prefs: None,
        })
        .await;
    api.push_unsubscribe("https://push/e").await.unwrap();
}

#[tokio::test]
async fn pair_persists_the_minted_token() {
    let (_mock, url) = serve_canned(Canned::default()).await;
    let (api, _projection) = api_against(vec![hub_node("n1", &url)]);
    let token = api.pair("code-1", Some("n1")).await.unwrap();
    assert_eq!(token, "sepia_minted");
}

#[tokio::test]
async fn api_error_arms() {
    let mock = Canned::default();
    mock.blank.lock().unwrap().push("/api/sessions".into());
    let (_m, url) = serve_canned(mock).await;
    let (api, _projection) = api_against(vec![hub_node("n1", &url)]);
    // `create` returns `{}` → no `id` → the malformed arm.
    assert!(
        api.create_entity("sepia-session", &json!({}), Some("n1"))
            .await
            .unwrap_err()
            .contains("no entity id")
    );
    // `entity` on a missing row → the "not found" arm.
    assert!(
        api.entity("ghost")
            .await
            .unwrap_err()
            .contains("Entity not found")
    );
    // `snapshot_import` on a missing id → the same arm.
    assert!(
        api.snapshot_import(&json!({}), Some("ghost"))
            .await
            .unwrap_err()
            .contains("not registered")
    );
}

/* ---- boot / state -----------------------------------------------------*/

#[tokio::test]
async fn hub_state_from_boot_arms() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    // A read-only projection.db → `purge_unregistered` fails → warn.
    let hub_home = home.join("hub");
    std::fs::create_dir_all(&hub_home).unwrap();
    let db = hub_home.join("projection.db");
    let _ = peregrin_sync::ProjectionStore::open(&db).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&db, std::fs::Permissions::from_mode(0o444)).unwrap();
    }
    let home_s = home.to_string_lossy().into_owned();
    let cfg = peregrin_hub::HubConfig::default();
    let _ = peregrin_hub::hub_state_from(
        |k| (k == "PEREGRIN_HOME").then(|| std::ffi::OsString::from(home_s.clone())),
        &cfg,
    );
}
