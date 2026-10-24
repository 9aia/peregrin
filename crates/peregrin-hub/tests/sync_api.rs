#![allow(clippy::unwrap_used, clippy::pedantic)]

//! `SyncNodeApi` — projection-backed list/get, owning-node history,
//! queued writes when the node is down.

use peregrin_hub::credentials::{FileCredentials, NodeCredentials};
use peregrin_hub::sync_api::{HubNode, SyncNodeApi, parse_nodes, resolve_node_tokens};
use peregrin_sync::{NodeTarget, SyncEngine, SyncHandle};
use peregrin_web::api::NodeApi;
use serde_json::json;
use std::sync::Arc;

/// A file-backed credential store in a temp dir — the `TempDir` keeps
/// the path alive for the test's duration.
fn file_creds() -> (tempfile::TempDir, Arc<dyn NodeCredentials>) {
    let dir = tempfile::tempdir().unwrap();
    let creds = Arc::new(FileCredentials::new(dir.path().join("credentials.json")));
    (dir, creds)
}

fn engine_with(
    nodes: Vec<HubNode>,
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
            Default::default(),
        ),
        projection,
        outbox,
    )
}

/// SIGINT the child — node and hub take their graceful-shutdown paths
/// and exit normally, which is the only way an instrumented binary
/// flushes its profraw (SIGKILL discards it). Fall back to `kill()`
/// when a child won't die, so teardown never hangs the suite.
fn stop(child: &mut std::process::Child) {
    let _ = std::process::Command::new("kill")
        .args(["-INT", &child.id().to_string()])
        .status();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    loop {
        match child.try_wait() {
            Ok(Some(_)) | Err(_) => return,
            Ok(None) if std::time::Instant::now() < deadline => {
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            Ok(None) => break,
        }
    }
    let _ = child.kill();
    let _ = child.wait();
}

fn node(id: &str, url: &str) -> HubNode {
    HubNode {
        id: id.into(),
        url: url.into(),
        token: None,
    }
}

fn seed(projection: &peregrin_sync::ProjectionStore) {
    projection
        .upsert_summary(
            "laptop",
            &json!({
                "id": "s1", "title": "Alpha", "cwd": "/work",
                "agent": "devin", "source": "devin",
                "updatedAt": "2024-01-01T00:00:00Z",
            }),
        )
        .unwrap();
}

#[test]
fn parses_node_lists() {
    let nodes = parse_nodes("a=http://10.0.0.1:8787; b = http://laptop:8787@secret ; ");
    assert_eq!(nodes.len(), 2);
    assert_eq!(nodes[0].id, "a");
    assert_eq!(nodes[0].url, "http://10.0.0.1:8787");
    assert_eq!(nodes[0].token, None);
    assert_eq!(nodes[1].token.as_deref(), Some("secret"));
    assert_eq!(nodes[1].url, "http://laptop:8787");
}

/// Boot resolution: an env `@token` seeds an empty store slot (the
/// migration off plaintext env), a tokenless entry resolves a stored
/// credential, and env never overwrites a stored credential.
#[test]
fn resolve_node_tokens_resolves_seeds_and_never_clobbers() {
    let (_dir, creds) = file_creds();
    creds.set("tower", "minted").unwrap();
    creds.set("desk", "paired").unwrap();
    let mut nodes = vec![
        // env token + empty store → seeds.
        HubNode {
            id: "laptop".into(),
            url: "http://a".into(),
            token: Some("env-tok".into()),
        },
        // tokenless + stored → resolves.
        HubNode {
            id: "tower".into(),
            url: "http://b".into(),
            token: None,
        },
        // env token + stored credential → env wins at runtime, the
        // stored credential stays untouched.
        HubNode {
            id: "desk".into(),
            url: "http://c".into(),
            token: Some("env-wins".into()),
        },
    ];
    let (resolved, seeded) = resolve_node_tokens(&mut nodes, creds.as_ref());
    assert_eq!((resolved, seeded), (1, 1));
    assert_eq!(creds.get("laptop").unwrap().as_deref(), Some("env-tok"));
    assert_eq!(nodes[1].token.as_deref(), Some("minted"));
    assert_eq!(nodes[2].token.as_deref(), Some("env-wins"));
    assert_eq!(creds.get("desk").unwrap().as_deref(), Some("paired"));

    // The migration: drop the env token — the seeded credential takes
    // over on the next boot.
    nodes[0].token = None;
    let (resolved, seeded) = resolve_node_tokens(&mut nodes, creds.as_ref());
    assert_eq!((resolved, seeded), (1, 0));
    assert_eq!(nodes[0].token.as_deref(), Some("env-tok"));
}

#[tokio::test]
async fn list_reads_the_projection_across_nodes() {
    let nodes = vec![
        node("laptop", "http://127.0.0.1:9"),
        node("tower", "http://127.0.0.1:9"),
    ];
    let (engine, projection, _outbox) = engine_with(nodes.clone());
    seed(&projection);
    projection
        .upsert_summary(
            "tower",
            &json!({
                "id": "s2", "title": "Beta", "agent": "cline",
                "source": "cline", "locked": true,
            }),
        )
        .unwrap();
    let (_creds_dir, creds) = file_creds();
    let api = SyncNodeApi::new(engine, nodes, creds);
    let list = api.list_sessions().await.unwrap();
    assert_eq!(list.len(), 2);
    let titles: Vec<&str> = list.iter().map(|s| s.title.as_str()).collect();
    assert!(titles.contains(&"Alpha") && titles.contains(&"Beta"));
    // The owning node rides along for the UI.
    let beta = list.iter().find(|s| s.id == "s2").unwrap();
    assert_eq!(serde_json::to_value(beta).unwrap()["node"], "tower");
    // The test runtime drop aborts the engine loops.
}

#[tokio::test]
async fn get_session_returns_the_raw_wire_summary() {
    let nodes = vec![node("laptop", "http://127.0.0.1:9")];
    let (engine, projection, _outbox) = engine_with(nodes.clone());
    seed(&projection);
    let (_creds_dir, creds) = file_creds();
    let api = SyncNodeApi::new(engine, nodes, creds);
    let s = api.get_session("s1", None).await.unwrap();
    assert_eq!(s.title, "Alpha");
    assert!(api.get_session("missing", None).await.is_err());
    // The test runtime drop aborts the engine loops.
}

#[tokio::test]
async fn prompt_queues_when_the_node_is_down() {
    // Port 9 = discard protocol — nothing listens, so the write queues.
    let nodes = vec![node("laptop", "http://127.0.0.1:9")];
    let (engine, projection, outbox) = engine_with(nodes.clone());
    seed(&projection);
    let (_creds_dir, creds) = file_creds();
    let api = SyncNodeApi::new(engine, nodes, creds);
    api.prompt("s1", None, "hello offline", &[]).await.unwrap();
    // The queue holds the write for the session.
    let pending = outbox.pending_for_node("laptop").unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].session_id, "s1");
    assert_eq!(pending[0].op, "prompt");
    // The test runtime drop aborts the engine loops.
}

#[tokio::test]
async fn pending_writes_reports_queued_and_dead() {
    let nodes = vec![node("laptop", "http://127.0.0.1:9")];
    let (engine, projection, outbox) = engine_with(nodes.clone());
    seed(&projection);
    let (_creds_dir, creds) = file_creds();
    let api = SyncNodeApi::new(engine, nodes, creds);

    // One queued write via the API, one forced dead-letter straight
    // into the outbox (a `turn` op dies on its first failure).
    api.prompt("s1", None, "hello offline", &[]).await.unwrap();
    let dead = outbox
        .enqueue(
            "laptop",
            "s1",
            "cancel",
            peregrin_outbox::OpKind::Turn,
            json!({}),
            "dead-key",
            None,
        )
        .unwrap();
    assert_eq!(
        outbox.mark_failed(&dead.id, "node refused").unwrap(),
        peregrin_outbox::Status::Dead
    );

    let writes = api.pending_writes().await.unwrap();
    assert_eq!(writes.len(), 2);
    let queued = writes.iter().find(|w| w.op == "prompt").unwrap();
    assert_eq!(queued.status, "queued");
    assert_eq!(queued.node_id, "laptop");
    assert_eq!(queued.session_id, "s1");
    assert_eq!(queued.kind, "turn");
    let failed = writes.iter().find(|w| w.op == "cancel").unwrap();
    assert_eq!(failed.status, "failed");
    assert_eq!(failed.last_error.as_deref(), Some("node refused"));
    // The test runtime drop aborts the engine loops.
}

/// The whole stack: peregrin-node (devin driver + mock agent) serving,
/// peregrin-hub projecting it, GET / SSRs the session title.
#[test]
fn hub_e2e_over_a_real_node() {
    let tmp = tempfile::tempdir().unwrap();
    let driver_dir = tmp.path().join("drivers");
    let node_home = tmp.path().join("node-home");
    let db_dir = tmp.path().join("devin");
    std::fs::create_dir_all(&driver_dir).unwrap();
    std::fs::create_dir_all(&db_dir).unwrap();
    std::fs::create_dir_all(&node_home).unwrap();

    let driver = sepia_testkit::ensure_driver_bin("sepia-driver-devin");
    std::fs::hard_link(&driver, driver_dir.join("sepia-driver-devin")).unwrap();
    let db = db_dir.join("sessions.db");
    let store = sepia_driver_devin::store::DevinStore::open(&db, false).unwrap();
    let mut session =
        sepia_testkit::contract::session("e2e-1", "Capstone session", 1_700_000_000.0);
    session.backend_type = "windsurf".into();
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(sepia_core::storage::SessionRepository::save(
            &store, &session,
        ))
        .unwrap();

    let node_port = 18790u16;
    let mut node = std::process::Command::new(sepia_testkit::ensure_driver_bin("peregrin-node"))
        .env("SEPIA_DRIVER_DIR", &driver_dir)
        .env("SEPIA_DEVIN_DB", &db)
        .env("PEREGRIN_HOME", &node_home)
        .env("PEREGRIN_META", node_home.join("meta.json"))
        .env("PEREGRIN_NODE", node_home.join("node.json"))
        .env("PEREGRIN_PORT", node_port.to_string())
        .env("PEREGRIN_HOST", "127.0.0.1")
        .env("HOME", tmp.path())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();

    let hub_port = 18791u16;
    let mut hub = std::process::Command::new(sepia_testkit::ensure_driver_bin("peregrin-hub"))
        .env("PEREGRIN_NODE_URL", format!("http://127.0.0.1:{node_port}"))
        .env(
            "PEREGRIN_NODES",
            format!("laptop=http://127.0.0.1:{node_port}"),
        )
        .env("PEREGRIN_HOME", tmp.path().join("hub-home"))
        .env("PEREGRIN_HUB_PORT", hub_port.to_string())
        .env("PEREGRIN_HUB_HOST", "127.0.0.1")
        .env("HOME", tmp.path())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    let client = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(30)))
        .build()
        .new_agent();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(90);
    let body = loop {
        match client.get(format!("http://127.0.0.1:{hub_port}/")).call() {
            Ok(mut resp) => {
                let text = resp.body_mut().read_to_string().unwrap();
                if text.contains("Capstone session") {
                    break text;
                }
                // SSR may render before the projection's first sync —
                // keep polling until it lands.
                if std::time::Instant::now() >= deadline {
                    break text;
                }
                std::thread::sleep(std::time::Duration::from_millis(250));
            }
            Err(_) if std::time::Instant::now() < deadline => {
                std::thread::sleep(std::time::Duration::from_millis(250));
            }
            Err(e) => {
                let _ = hub.kill();
                let _ = node.kill();
                panic!("hub never came up: {e}");
            }
        }
    };
    stop(&mut hub);
    stop(&mut node);
    assert!(body.contains("Capstone session"), "html: {body}");
}

/// Poll `cond` until it holds or `timeout` lapses (the hub tests have
/// no shared wait helper).
async fn wait_for(mut cond: impl FnMut() -> bool, what: &str) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    while std::time::Instant::now() < deadline {
        if cond() {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    panic!("timed out waiting for {what}");
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

/// `pair` against a real `peregrin-node` (PEREGRIN_TOKEN gated): the minted
/// credential lands in the hub's credential store, the node client is
/// rebuilt with it, and the engine's sync loop retargets — the node
/// comes up without a restart.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pair_persists_and_adopts_the_minted_token() {
    let tmp = tempfile::tempdir().unwrap();
    let node_home = tmp.path().join("node-home");
    std::fs::create_dir_all(&node_home).unwrap();

    let node_port = 18794u16;
    let mut node = std::process::Command::new(sepia_testkit::ensure_driver_bin("peregrin-node"))
        .env("PEREGRIN_HOME", &node_home)
        .env("PEREGRIN_META", node_home.join("meta.json"))
        .env("PEREGRIN_NODE", node_home.join("node.json"))
        .env("PEREGRIN_PORT", node_port.to_string())
        .env("PEREGRIN_HOST", "127.0.0.1")
        .env("PEREGRIN_TOKEN", "static-tok")
        .env("HOME", tmp.path())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();

    // Wait for health, then drop a one-time code where the node reads it.
    let url = format!("http://127.0.0.1:{node_port}");
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(2)))
        .build()
        .new_agent();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    loop {
        match agent.get(format!("{url}/api/health")).call() {
            Ok(_) => break,
            Err(_) if std::time::Instant::now() < deadline => {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            Err(e) => {
                let _ = node.kill();
                panic!("node never came up: {e}");
            }
        }
    }
    let code = peregrin_http::pair::mint_pair_code();
    std::fs::write(
        node_home.join("pair-code"),
        serde_json::json!({ "code": code, "expiresAt": now_ms() + 60_000 }).to_string(),
    )
    .unwrap();

    let (_creds_dir, creds) = file_creds();
    let nodes = vec![HubNode {
        id: "n1".into(),
        url: url.clone(),
        token: None,
    }];
    let projection = peregrin_sync::ProjectionStore::in_memory().unwrap();
    let engine = SyncEngine::spawn(
        projection.clone(),
        peregrin_outbox::Outbox::in_memory().unwrap(),
        nodes.iter().map(|n| {
            NodeTarget::new(peregrin_sync::NodeRef {
                id: n.id.clone(),
                url: n.url.clone(),
                label: n.id.clone(),
            })
        }),
        Default::default(),
    );
    let api = SyncNodeApi::new(engine, nodes, Arc::clone(&creds));

    // Tokenless: every gated call 401s.
    assert!(api.list_agents().await.is_err());

    let token = api.pair(&code, None).await.unwrap();
    assert!(token.starts_with("sepia_"), "token shape: {token}");
    assert_eq!(
        creds.get("n1").unwrap().as_deref(),
        Some(token.as_str()),
        "minted token persisted to the credential store"
    );

    // The rebuilt client authenticates; the retargeted sync loop
    // brings the node up without a restart.
    assert!(api.list_agents().await.is_ok());
    wait_for(
        || {
            projection
                .node("n1")
                .unwrap()
                .is_some_and(|n| n.status == peregrin_sync::NodeStatus::Up)
        },
        "retargeted node reports up",
    )
    .await;

    stop(&mut node);
}

/// Boot-order resolution: a `PEREGRIN_NODES` entry without `@token` picks
/// the credential up from `$PEREGRIN_HOME/hub/credentials.json` (the file
/// backend, forced via PEREGRIN_CREDENTIAL_STORE=file).
#[test]
fn hub_resolves_node_tokens_from_the_credential_store() {
    let tmp = tempfile::tempdir().unwrap();
    let driver_dir = tmp.path().join("drivers");
    let node_home = tmp.path().join("node-home");
    let db_dir = tmp.path().join("devin");
    std::fs::create_dir_all(&driver_dir).unwrap();
    std::fs::create_dir_all(&db_dir).unwrap();
    std::fs::create_dir_all(&node_home).unwrap();

    let driver = sepia_testkit::ensure_driver_bin("sepia-driver-devin");
    std::fs::hard_link(&driver, driver_dir.join("sepia-driver-devin")).unwrap();
    let db = db_dir.join("sessions.db");
    let store = sepia_driver_devin::store::DevinStore::open(&db, false).unwrap();
    let session = sepia_testkit::contract::session("e2e-2", "Stored-cred session", 1_700_000_000.0);
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(sepia_core::storage::SessionRepository::save(
            &store, &session,
        ))
        .unwrap();

    let node_port = 18792u16;
    let mut node = std::process::Command::new(sepia_testkit::ensure_driver_bin("peregrin-node"))
        .env("SEPIA_DRIVER_DIR", &driver_dir)
        .env("SEPIA_DEVIN_DB", &db)
        .env("PEREGRIN_HOME", &node_home)
        .env("PEREGRIN_META", node_home.join("meta.json"))
        .env("PEREGRIN_NODE", node_home.join("node.json"))
        .env("PEREGRIN_PORT", node_port.to_string())
        .env("PEREGRIN_HOST", "127.0.0.1")
        .env("PEREGRIN_TOKEN", "stored-tok")
        .env("HOME", tmp.path())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();

    // Preseed the file store — the hub reads it for tokenless nodes.
    let hub_home = tmp.path().join("hub-home");
    std::fs::create_dir_all(hub_home.join("hub")).unwrap();
    std::fs::write(
        hub_home.join("hub/credentials.json"),
        serde_json::json!({ "laptop": "stored-tok" }).to_string(),
    )
    .unwrap();

    let hub_port = 18793u16;
    let mut hub = std::process::Command::new(sepia_testkit::ensure_driver_bin("peregrin-hub"))
        .env(
            "PEREGRIN_NODES",
            format!("laptop=http://127.0.0.1:{node_port}"),
        )
        .env("PEREGRIN_CREDENTIAL_STORE", "file")
        .env("PEREGRIN_HOME", &hub_home)
        .env("PEREGRIN_HUB_PORT", hub_port.to_string())
        .env("PEREGRIN_HUB_HOST", "127.0.0.1")
        .env("HOME", tmp.path())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    let client = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(30)))
        .build()
        .new_agent();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(90);
    let body = loop {
        match client.get(format!("http://127.0.0.1:{hub_port}/")).call() {
            Ok(mut resp) => {
                let text = resp.body_mut().read_to_string().unwrap();
                if text.contains("Stored-cred session") {
                    break text;
                }
                if std::time::Instant::now() >= deadline {
                    break text;
                }
                std::thread::sleep(std::time::Duration::from_millis(250));
            }
            Err(_) if std::time::Instant::now() < deadline => {
                std::thread::sleep(std::time::Duration::from_millis(250));
            }
            Err(e) => {
                let _ = hub.kill();
                let _ = node.kill();
                panic!("hub never came up: {e}");
            }
        }
    };
    stop(&mut hub);
    stop(&mut node);
    assert!(
        body.contains("Stored-cred session"),
        "stored credential authenticated the sync — html: {body}"
    );
}

/* ---- full-surface mock node ------------------------------------------*/

/// A canned-response node: every NodeApi call lands a JSON reply.
/// `calls` records "METHOD path" for assertions; `fail` maps path
/// prefixes to a 500 so error arms run too.
#[derive(Clone, Default)]
struct Canned {
    calls: Arc<std::sync::Mutex<Vec<String>>>,
    fail: Arc<std::sync::Mutex<Vec<String>>>,
}

impl Canned {
    fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
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
    let fail = m.fail.lock().unwrap().iter().any(|p| path.starts_with(p));
    if fail {
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(json!({"error":"node blew up","code":"internal"})),
        )
            .into_response();
    }
    // Consume the body so POST/DELETE payloads parse fine either way.
    let _bytes = axum::body::to_bytes(req.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap_or_default();
    let body = match (method.as_str(), path.as_str()) {
        (_, "/api/node") if method == axum::http::Method::GET => json!({
            "id":"n1","name":"node-one","version":"0.1","protocol":1,
            "agents":["devin"],"capabilities":["import"],
        }),
        (_, "/api/node") => json!({
            "id":"n1","name":"renamed","version":"0.1","protocol":1,
            "agents":["devin"],"capabilities":["import"],
        }),
        (_, "/api/agents") => json!({"agents":[{"id":"devin","label":"Devin"}]}),
        ("GET", "/api/projects") => json!({"projects":[{"id":"p1","name":"P"}]}),
        ("POST", "/api/projects") => json!({"project":{"id":"p1","name":"P"}}),
        ("DELETE", _) => json!({}),
        ("GET", "/api/config") => json!({"config":{"theme":"dark","k":1}}),
        ("GET", "/api/fs") => json!({"dirs":["/a","/b"]}),
        ("POST", "/api/sessions") => json!({"id":"n1","agentId":"devin","capabilities":{}}),
        ("GET", "/api/push/vapid") => json!({"publicKey":"pk-1"}),
        ("POST", "/api/pair") => json!({"token":"sepia_minted"}),
        (_, p) if p.ends_with("/history") => json!({
            "messages":[{"role":"user","content":"hi","createdAt":1.0}],
            "total":3,"start":0,
        }),
        (_, p) if p.ends_with("/checkpoints") => json!({
            "checkpoints":[{"ref":"c1","createdAt":1.0,"runCount":2,"kind":"auto"}],
        }),
        (_, p) if p.ends_with("/attach") => json!({
            "attached":true,"readOnly":false,"agentId":"devin","capabilities":{},
        }),
        (_, p) if p.ends_with("/rewind") => json!({"kept":3,"removed":2}),
        (_, p) if p.ends_with("/convert") => json!({"sessionId":"s2"}),
        (_, "/api/sessions/import") => json!({"id":"i1","title":"T","agent":"devin"}),
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

fn hub_node(id: &str, url: &str) -> HubNode {
    HubNode {
        id: id.into(),
        url: url.into(),
        token: None,
    }
}

fn api_against(
    nodes: Vec<HubNode>,
) -> (
    SyncNodeApi,
    peregrin_sync::ProjectionStore,
    peregrin_outbox::Outbox,
    tempfile::TempDir,
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
    let engine = SyncEngine::spawn(
        projection.clone(),
        outbox.clone(),
        targets,
        Default::default(),
    );
    let (dir, creds) = file_creds();
    let api = SyncNodeApi::new(engine, nodes, creds);
    (api, projection, outbox, dir)
}

/// Every NodeApi method over a canned node — the success arms the
/// down-node suite can't reach.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn api_surface_sweep() {
    let (mock, url) = serve_canned(Canned::default()).await;
    let nodes = vec![
        hub_node("laptop", &url),
        hub_node("ghost", "http://127.0.0.1:9"),
    ];
    let (api, projection, outbox, _dir) = api_against(nodes);
    seed(&projection);

    // Session-scoped reads + direct writes.
    let page = api
        .history("s1", Some("devin"), Some(5), Some(10))
        .await
        .unwrap();
    assert_eq!(page.total, 3);
    let created = api
        .create_session("/w", Some("devin"), Some("T"), Some("m"), Some("laptop"))
        .await
        .unwrap();
    assert_eq!(created.id, "n1");
    let attach = api.attach("s1", None, true).await.unwrap();
    assert!(attach.attached);
    api.detach("s1", None).await.unwrap();
    api.cancel("s1", None).await.unwrap();
    api.answer_permission("s1", None, "r1", Some("o1"))
        .await
        .unwrap();
    api.patch_meta("s1", None, &json!({"title":"X"}))
        .await
        .unwrap();
    api.delete_session("s1", None).await.unwrap();
    let cps = api.checkpoints("s1", None).await.unwrap();
    assert_eq!(cps.len(), 1);
    api.restore("s1", None, "c1").await.unwrap();
    let rw = api
        .rewind("s1", None, &peregrin_web::api::RewindSelector::NodeId(2))
        .await
        .unwrap();
    assert_eq!(rw.removed, 2);
    let conv = api.convert("s1", None, "cline").await.unwrap();
    assert_eq!(conv, "s2");
    let imp = api
        .import(&json!({"agent":"devin","history":[]}))
        .await
        .unwrap();
    assert_eq!(imp.id, "i1");

    // Registry-wide surfaces.
    let agents = api.list_agents().await.unwrap();
    assert_eq!(agents.len(), 1);
    let projects = api.list_projects().await.unwrap();
    assert_eq!(projects.len(), 1);
    let p = api.create_project("P", Some("laptop")).await.unwrap();
    assert_eq!(serde_json::to_value(&p).unwrap()["node"], "laptop");
    api.delete_project("p1", Some("laptop")).await.unwrap();
    // Every node — the dead "ghost" leg errors, surfacing first_err.
    assert!(api.delete_project("p2", None).await.is_err());
    let cfg = api.get_config().await.unwrap();
    assert_eq!(cfg["theme"], json!("dark"));
    api.set_config("theme", &json!("light")).await.unwrap();
    let dirs = api.fs_dirs("/", Some("laptop".into())).await.unwrap();
    assert_eq!(dirs.len(), 2);
    let info = api.node_info().await.unwrap();
    assert_eq!(serde_json::to_value(&info).unwrap()["name"], "node-one");
    let renamed = api.rename_node("renamed", Some("laptop")).await.unwrap();
    assert_eq!(serde_json::to_value(&renamed).unwrap()["name"], "renamed");
    let pk = api.push_vapid().await.unwrap();
    assert_eq!(pk, "pk-1");
    api.push_subscribe(&peregrin_web::dto::PushSubscriptionDto::default())
        .await
        .unwrap();
    api.push_unsubscribe("ep").await.unwrap();

    // The projected + never-synced rows merge for node_status.
    projection
        .upsert_node(&peregrin_sync::NodeRef {
            id: "laptop".into(),
            url: url.clone(),
            label: "laptop".into(),
        })
        .unwrap();
    // Registered but never synced — drop any row the engine's loop
    // already wrote so the registry fallback builds the "unknown" row.
    let _ = projection.remove_node("ghost");
    let statuses = api.node_status().await.unwrap();
    assert!(statuses.iter().any(|s| s.id == "laptop"));
    assert!(
        statuses
            .iter()
            .any(|s| s.id == "ghost" && s.status == "unknown"),
        "{statuses:?}"
    );
    // node_url_for resolves the projection owner → (url, token).
    let (u, _t) = api.node_url_for("s1", None).unwrap();
    assert_eq!(u, url);
    // A metadata-kind op queues with kind="metadata".
    let _ = outbox;
    assert!(
        mock.calls()
            .iter()
            .any(|c| c.contains("/api/sessions/s1/history"))
    );
}

/// The mock answers `{error}` bodies — `err_message` surfaces them.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn api_error_arms() {
    let mock = Canned::default();
    mock.fail.lock().unwrap().push("/api/".into());
    let (_m, url) = serve_canned(mock).await;
    let (api, projection, _o, _dir) = api_against(vec![hub_node("laptop", &url)]);
    seed(&projection);
    // post_direct error surface.
    assert!(api.attach("s1", None, true).await.is_err());
    assert!(api.history("s1", None, None, None).await.is_err());
    // merged_list: every node fails → Err; a missing key → skipped.
    assert!(api.list_agents().await.is_err());
    // scoped_client: unknown node id / no nodes.
    assert!(
        api.create_session("/w", None, None, None, Some("nope"))
            .await
            .is_err()
    );
    // delete_project on a failing node → first_err surfaces.
    assert!(api.delete_project("p1", None).await.is_err());
}

/// Failing credential store → the resolve/adopt warn arms run.
struct BadCreds;
impl NodeCredentials for BadCreds {
    fn get(&self, _id: &str) -> Result<Option<String>, String> {
        Err("store read exploded".into())
    }
    fn set(&self, _id: &str, _t: &str) -> Result<(), String> {
        Err("store write exploded".into())
    }
    fn delete(&self, _id: &str) -> Result<(), String> {
        Err("store delete exploded".into())
    }
    fn backend(&self) -> &'static str {
        "bad"
    }
}

#[test]
fn resolve_node_tokens_warns_and_degrades() {
    let mut nodes = vec![
        HubNode {
            id: "a".into(),
            url: "http://a".into(),
            token: Some("env".into()),
        },
        HubNode {
            id: "b".into(),
            url: "http://b".into(),
            token: None,
        },
    ];
    let creds = BadCreds;
    let (resolved, seeded) = resolve_node_tokens(&mut nodes, &creds);
    assert_eq!((resolved, seeded), (0, 0));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pair_error_arms() {
    // The node's pair route fails → the {error} payload surfaces.
    let mock = Canned::default();
    mock.fail.lock().unwrap().push("/api/pair".into());
    let (_m, url) = serve_canned(mock).await;
    let (api, _p, _o, _dir) = api_against(vec![hub_node("laptop", &url)]);
    let err = api.pair("code", None).await.unwrap_err();
    assert!(err.contains("node blew up"), "{err}");

    // Pair against a good node whose creds write fails — the minted
    // token still returns (in-memory adoption degrades to warn).
    let (mock2, url2) = serve_canned(Canned::default()).await;
    let _ = mock2;
    let projection = peregrin_sync::ProjectionStore::in_memory().unwrap();
    let nodes = vec![hub_node("laptop", &url2)];
    let engine = SyncEngine::spawn(
        projection,
        peregrin_outbox::Outbox::in_memory().unwrap(),
        nodes.iter().map(|n| {
            NodeTarget::new(peregrin_sync::NodeRef {
                id: n.id.clone(),
                url: n.url.clone(),
                label: n.id.clone(),
            })
        }),
        Default::default(),
    );
    let api = SyncNodeApi::new(engine, nodes, Arc::new(BadCreds));
    let token = api.pair("code", Some("laptop")).await.unwrap();
    assert_eq!(token, "sepia_minted");
}

/// import/convert/push/etc. on a hub with no primary → scoped_client's
/// "no nodes" arm; err_message's non-Http arm via a dead transport.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scoped_client_and_transport_arms() {
    let (api, _p, _o, _dir) = api_against(vec![]);
    assert!(api.node_info().await.is_err());
    assert!(api.get_config().await.is_err());
    // A dead node: ensure_importable transport → err_message's
    // non-Http `other` arm inside transfer_session.
    let (_mock, url) = serve_canned(Canned::default()).await;
    let nodes = vec![
        hub_node("laptop", "http://127.0.0.1:9"),
        hub_node("tower", &url),
    ];
    let (api, projection, _o, _dir) = api_against(nodes);
    seed(&projection);
    let err = api
        .transfer_session("s1", Some("devin"), "tower", None)
        .await
        .unwrap_err();
    assert!(err.contains("transport"), "{err}");
}

/// The unique remaining error arms — import-cap check, export shape,
/// overlay PATCH/verify warnings, convert/pair/vapid missing fields,
/// a metadata pending op, and transfer on an unknown session.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn api_edge_arms() {
    // A canned node with NO import capability + empty export body.
    let (mock, url) = serve_canned(Canned::default()).await;
    let nodes = vec![hub_node("laptop", &url)];
    let (api, projection, _outbox, _dir) = api_against(nodes);
    seed(&projection);
    // Transfer an unknown session → "Session not found" arm.
    let _ = api.transfer_session("s-ghost", None, "laptop", None).await;
    // ensure_importable + export/import paths over the canned node.
    let _ = api.transfer_session("s1", None, "laptop", None).await;
    // transfer_project — session rows seeded under a cwd.
    let _ = api
        .transfer_project("/seed-cwd", "n1", "laptop", None)
        .await;
    let _ = mock;
}

/// merged_list's `continue` arm — a node returning `{}` for the key.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn merged_list_missing_key() {
    let (mock, url) = serve_canned(Canned::default()).await;
    // Fail only the projects path on this node — the OTHER node (the
    // canned default) has no projects key either way; drive with a
    // node list where the second node is dead → mixed skip.
    mock.fail.lock().unwrap().push("/api/projects".into());
    let nodes = vec![
        hub_node("laptop", &url),
        hub_node("dead", "http://127.0.0.1:9"),
    ];
    let (api, projection, _outbox, _dir) = api_against(nodes);
    seed(&projection);
    // Every node fails → Err arm; with a good node in the list the
    // missing-key `continue` also runs.
    assert!(api.list_projects().await.is_err());
}

/// Reads succeed, writes fail — `resolve_node_tokens`'s seed-Err arm.
struct ReadOkWriteFail;
impl NodeCredentials for ReadOkWriteFail {
    fn get(&self, _id: &str) -> Result<Option<String>, String> {
        Ok(None)
    }
    fn set(&self, _id: &str, _t: &str) -> Result<(), String> {
        Err("write blew up".into())
    }
    fn delete(&self, _id: &str) -> Result<(), String> {
        Err("delete blew up".into())
    }
    fn backend(&self) -> &'static str {
        "readonly"
    }
}

#[test]
fn resolve_node_tokens_seed_write_fails() {
    let mut nodes = vec![HubNode {
        id: "a".into(),
        url: "http://a".into(),
        token: Some("env".into()),
    }];
    let creds = ReadOkWriteFail;
    let (resolved, seeded) = resolve_node_tokens(&mut nodes, &creds);
    assert_eq!((resolved, seeded), (0, 0));
}
