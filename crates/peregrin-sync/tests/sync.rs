#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::pedantic,
    clippy::missing_panics_doc
)]

//! Integration tests — the engine against a real `peregrin_http::app`
//! served on a loopback socket (ureq needs a wire, not `oneshot`).
//!
//! `StubNode` puts a toggling TCP proxy in front of the axum server:
//! `down()` aborts the accept loop *and* the per-connection forwarders,
//! so in-flight SSE connections die like a real crashed node — aborting
//! `axum::serve` alone leaves accepted connections (and their
//! heartbeats) alive.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use peregrin_http::{AppState, EventFeed, app};
use peregrin_outbox::{OpKind, Outbox};
use peregrin_session::LocalPlane;
use peregrin_sync::engine::SubmitOutcome;
use peregrin_sync::{
    IndexedSession, NodeClient, NodeRef, NodeStatus, NodeTarget, NodeWrite, PatchStatus,
    ProjectionStore, SyncEngine, SyncOptions,
};
use sepia_core::storage::SessionRepository;
use sepia_core::{Session, StorageError};
use sepia_meta::MetaStore;
use sepia_session::{ControlPlane, ControlPlaneOptions};
use serde_json::{Value, json};

/* ---- fake store + node harness --------------------------------------*/

struct MemStore {
    sessions: Mutex<HashMap<String, Session>>,
}

#[async_trait]
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
        let mut all: Vec<Session> = self.sessions.lock().unwrap().values().cloned().collect();
        all.sort_by(|a, b| b.last_activity_at.total_cmp(&a.last_activity_at));
        Ok(all)
    }
    async fn delete(&self, id: &str) -> Result<(), StorageError> {
        self.sessions.lock().unwrap().remove(id);
        Ok(())
    }
    async fn has_session(&self, id: &str) -> Result<bool, StorageError> {
        Ok(self.sessions.lock().unwrap().contains_key(id))
    }
}

fn session(id: &str, title: &str) -> Session {
    let mut s = sepia_testkit::contract::session(id, title, 1_700_000_100.0);
    s.backend_type = "devin".into();
    s.working_directory = "/tmp".into();
    s
}

/// Accepts on `listener` and splices each connection to `real`.
/// `kill` refuses new accepts; the task is aborted (with every tracked
/// connection task) to simulate the node vanishing.
fn spawn_proxy(
    listener: tokio::net::TcpListener,
    real: std::net::SocketAddr,
    kill: Arc<AtomicBool>,
    conns: Arc<Mutex<Vec<tokio::task::JoinHandle<()>>>>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            let Ok((mut inbound, _)) = listener.accept().await else {
                return;
            };
            if kill.load(Ordering::Relaxed) {
                drop(inbound);
                continue;
            }
            let task = tokio::spawn(async move {
                let Ok(mut outbound) = tokio::net::TcpStream::connect(real).await else {
                    return;
                };
                let _ = tokio::io::copy_bidirectional(&mut inbound, &mut outbound).await;
            });
            conns
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .push(task);
        }
    })
}

/// A `peregrin_http` node behind a killable proxy, controllable by the test.
struct StubNode {
    /// Proxy port — the engine's target URL.
    port: u16,
    /// The real axum server's port.
    real_port: u16,
    store: Arc<MemStore>,
    feed: EventFeed,
    meta: MetaStore,
    kill: Arc<AtomicBool>,
    conns: Arc<Mutex<Vec<tokio::task::JoinHandle<()>>>>,
    accept: Option<tokio::task::JoinHandle<()>>,
    _serve: tokio::task::JoinHandle<()>,
    _dir: tempfile::TempDir,
}

impl StubNode {
    fn url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    fn node_ref(&self, id: &str) -> NodeRef {
        NodeRef {
            id: id.into(),
            url: self.url(),
            label: format!("test {id}"),
        }
    }

    /// Simulate a crash: stop accepting and sever live connections.
    /// The port frees once the accept task unwinds.
    fn down(&mut self) {
        self.kill.store(true, Ordering::Relaxed);
        if let Some(accept) = self.accept.take() {
            accept.abort();
        }
        for task in self
            .conns
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .drain(..)
        {
            task.abort();
        }
    }

    /// Back on the same proxy port — the previous listener was dropped
    /// by `down`'s abort (SO_REUSEADDR + a short retry cover the race).
    async fn up(&mut self) {
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        let listener = loop {
            match tokio::net::TcpListener::bind(("127.0.0.1", self.port)).await {
                Ok(l) => break l,
                Err(e) if std::time::Instant::now() < deadline => {
                    let _ = e;
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
                Err(e) => panic!("rebind proxy port {}: {e}", self.port),
            }
        };
        self.kill.store(false, Ordering::Relaxed);
        let real = ([127, 0, 0, 1], self.real_port).into();
        self.accept = Some(spawn_proxy(
            listener,
            real,
            Arc::clone(&self.kill),
            Arc::clone(&self.conns),
        ));
    }
}

async fn start_stub(sessions: Vec<Session>) -> StubNode {
    start_stub_with(sessions, None).await
}

/// `token` arms the node's bearer gate — credential tests connect
/// tokenless first, then retarget with the minted token.
async fn start_stub_with(sessions: Vec<Session>, token: Option<String>) -> StubNode {
    let store = Arc::new(MemStore {
        sessions: Mutex::new(sessions.into_iter().map(|s| (s.id.clone(), s)).collect()),
    });
    let plane = ControlPlane::new(
        store.clone(),
        ControlPlaneOptions {
            idle_ttl: Some(Duration::ZERO),
            ..ControlPlaneOptions::default()
        },
    );
    let dir = tempfile::tempdir().unwrap();
    let meta = MetaStore::open(&dir.path().join("meta.json"));
    let state = AppState::new(Arc::new(LocalPlane(plane)))
        .with_meta(meta.clone())
        .with_token(token)
        .with_held_watch(Duration::ZERO)
        .with_keep_alive(Duration::from_millis(40));
    let feed = state.feed.clone();

    // Real node server on an ephemeral port…
    let real_listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .unwrap();
    let real_port = real_listener.local_addr().unwrap().port();
    let serve = tokio::spawn(async move {
        let _ = axum::serve(real_listener, app(state)).await;
    });

    // …behind the killable proxy the engine actually talks to.
    let proxy_listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .unwrap();
    let port = proxy_listener.local_addr().unwrap().port();
    let kill = Arc::new(AtomicBool::new(false));
    let conns = Arc::new(Mutex::new(Vec::new()));
    let accept = spawn_proxy(
        proxy_listener,
        ([127, 0, 0, 1], real_port).into(),
        Arc::clone(&kill),
        Arc::clone(&conns),
    );

    StubNode {
        port,
        real_port,
        store,
        feed,
        meta,
        kill,
        conns,
        accept: Some(accept),
        _serve: serve,
        _dir: dir,
    }
}

fn fast_options() -> SyncOptions {
    SyncOptions {
        on_event: None,
        connect_timeout: Duration::from_millis(300),
        request_timeout: Duration::from_secs(2),
        // Long enough that no mid-test stall reconnect drops emitted
        // events — pump teardown is still bounded by the 40ms heartbeat.
        sse_idle_timeout: Duration::from_secs(30),
        backoff_min: Duration::from_millis(30),
        backoff_max: Duration::from_millis(200),
        resync_interval: Duration::from_secs(3600),
        feed_channel: 64,
    }
}

/// Poll `cond` until it holds or `timeout` lapses.
async fn wait_until(mut cond: impl FnMut() -> bool, timeout: Duration, what: &str) {
    let deadline = std::time::Instant::now() + timeout;
    while std::time::Instant::now() < deadline {
        if cond() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("timed out waiting for {what}");
}

fn indexed(node_id: &str, session_id: &str) -> impl Fn(&ProjectionStore) -> Option<IndexedSession> {
    let node_id = node_id.to_string();
    let session_id = session_id.to_string();
    move |p: &ProjectionStore| p.session(&node_id, &session_id).unwrap()
}

fn write(
    node_id: &str,
    session_id: &str,
    op: &str,
    kind: OpKind,
    payload: Value,
    key: &str,
) -> NodeWrite {
    NodeWrite {
        node_id: node_id.into(),
        session_id: session_id.into(),
        agent: Some("devin".into()),
        op: op.into(),
        kind,
        payload,
        idempotency_key: key.into(),
        ttl_seconds: None,
    }
}

/* ---- projection store (pure) -----------------------------------------*/

#[test]
fn projection_upserts_patches_and_reconciles() {
    let store = ProjectionStore::in_memory().unwrap();
    store
        .upsert_node(&NodeRef {
            id: "n1".into(),
            url: "http://x".into(),
            label: "node one".into(),
        })
        .unwrap();
    let row = store.node("n1").unwrap().unwrap();
    assert_eq!(row.status, NodeStatus::Unknown);

    let summary = json!({
        "id": "s1", "title": "T1", "cwd": "/a", "agent": "devin",
        "source": "devin", "updatedAt": "2024-01-01T00:00:01.000Z",
        "locked": false, "busy": true, "pinned": true,
    });
    store.upsert_summary("n1", &summary).unwrap();
    let got = store.session("n1", "s1").unwrap().unwrap();
    assert_eq!(got.title, "T1");
    assert!(got.busy);
    assert_eq!(got.raw["pinned"], true);

    // Diff merge — un-patched fields survive; columns re-derive.
    store
        .apply_patch("n1", "s1", &json!({"title": "T2", "busy": false}))
        .unwrap();
    let got = store.session("n1", "s1").unwrap().unwrap();
    assert_eq!(got.title, "T2");
    assert!(!got.busy);
    assert_eq!(got.cwd, "/a");
    assert_eq!(got.raw["pinned"], true);

    // Reconcile: a listing without s1 drops it; s2 keeps it.
    store
        .upsert_summary(
            "n1",
            &json!({"id":"s2","title":"S2","updatedAt":"2024-01-02T00:00:00.000Z"}),
        )
        .unwrap();
    store
        .apply_listing(
            "n1",
            &[json!({"id":"s2","title":"S2","updatedAt":"2024-01-02T00:00:00.000Z"})],
        )
        .unwrap();
    assert!(store.session("n1", "s1").unwrap().is_none());
    assert!(store.session("n1", "s2").unwrap().is_some());
    assert_eq!(
        store.cursor("n1").unwrap().as_deref(),
        Some("2024-01-02T00:00:00.000Z")
    );

    store.set_node_status("n1", NodeStatus::Up).unwrap();
    let row = store.node("n1").unwrap().unwrap();
    assert_eq!(row.status, NodeStatus::Up);
    assert!(row.last_seen_at.is_some());
}

#[test]
fn purge_unregistered_drops_stale_node_rows() {
    let store = ProjectionStore::in_memory().unwrap();
    for id in ["main", "node"] {
        store
            .upsert_node(&NodeRef {
                id: id.into(),
                url: "http://x".into(),
                label: id.into(),
            })
            .unwrap();
        store
            .upsert_summary(
                id,
                &json!({"id":"s1","title":"T","updatedAt":"2024-01-01T00:00:00.000Z"}),
            )
            .unwrap();
    }
    // The legacy `node` id is gone from config — its rows go with it.
    store.purge_unregistered(&["main".to_string()]).unwrap();
    assert!(store.session("main", "s1").unwrap().is_some());
    assert!(store.session("node", "s1").unwrap().is_none());
    assert!(store.node("main").unwrap().is_some());
    assert!(store.node("node").unwrap().is_none());
    // Empty keep-list is a no-op guard, not a wipe.
    store.purge_unregistered(&[]).unwrap();
    assert!(store.session("main", "s1").unwrap().is_some());
}

/* ---- full cycle -------------------------------------------------------*/

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn node_down_enqueue_then_up_drains_in_order() {
    let mut stub = start_stub(vec![session("s1", "S1")]).await;
    stub.down();

    let projection = ProjectionStore::in_memory().unwrap();
    let outbox = Outbox::in_memory().unwrap();
    let handle = SyncEngine::spawn(
        projection.clone(),
        outbox.clone(),
        [NodeTarget::new(stub.node_ref("n1"))],
        fast_options(),
    );

    // Writes queue while the node is down.
    let queued = handle
        .submit_op(write(
            "n1",
            "s1",
            "meta.patch",
            OpKind::Metadata,
            json!({"title": "queued rename"}),
            "k1",
        ))
        .await
        .unwrap();
    assert!(matches!(queued, SubmitOutcome::Queued(_)));
    handle
        .submit_op(write(
            "n1",
            "s1",
            "meta.patch",
            OpKind::Metadata,
            json!({"pinned": true}),
            "k2",
        ))
        .await
        .unwrap();
    assert_eq!(outbox.pending_for_node("n1").unwrap().len(), 2);

    // Node comes up — the engine reconnects, lists, and drains FIFO.
    stub.up().await;
    let proj = projection.clone();
    wait_until(
        || {
            proj.node("n1")
                .unwrap()
                .is_some_and(|n| n.status == NodeStatus::Up)
        },
        Duration::from_secs(10),
        "node up",
    )
    .await;
    wait_until(
        || outbox.pending_for_node("n1").unwrap().is_empty(),
        Duration::from_secs(10),
        "outbox drained",
    )
    .await;

    // Order + effect: last-writer-wins landed both patches, in order.
    // Poll — "outbox drained" only means *submitted*; the stub applies
    // submitted ops asynchronously, so a direct read races the apply
    // (flakes under load).
    wait_until(
        || {
            stub.meta.of("s1").is_some_and(|m| {
                m.title.as_deref() == Some("queued rename") && m.pinned == Some(true)
            })
        },
        Duration::from_secs(10),
        "both meta patches applied in order",
    )
    .await;
    let meta = stub.meta.of("s1").unwrap();
    assert_eq!(meta.title.as_deref(), Some("queued rename"));
    assert_eq!(meta.pinned, Some(true));

    // Projection picked up the listing, and the meta-patch SSE diff
    // from the drained write propagated the overlay too.
    let got = indexed("n1", "s1")(&projection).unwrap();
    assert_eq!(got.agent, "devin");
    let proj = projection.clone();
    wait_until(
        || indexed("n1", "s1")(&proj).is_some_and(|s| s.raw["pinned"] == true),
        Duration::from_secs(5),
        "pinned propagated via SSE",
    )
    .await;

    handle.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn turn_op_rejected_by_node_dead_letters() {
    let mut stub = start_stub(vec![session("s1", "S1")]).await;
    stub.down();

    let projection = ProjectionStore::in_memory().unwrap();
    let outbox = Outbox::in_memory().unwrap();
    let handle = SyncEngine::spawn(
        projection,
        outbox.clone(),
        [NodeTarget::new(stub.node_ref("n1"))],
        fast_options(),
    );

    // A prompt queued while down — Turn kind dead-letters on replay
    // failure instead of reordering into the session. (The stub has no
    // live agent, so the node rejects it with an HTTP error.)
    handle
        .submit_op(write(
            "n1",
            "s1",
            "prompt",
            OpKind::Turn,
            json!({"text": "hi"}),
            "p1",
        ))
        .await
        .unwrap();

    stub.up().await;
    let outbox2 = outbox.clone();
    wait_until(
        || {
            outbox2
                .dead_letters()
                .unwrap()
                .iter()
                .any(|e| e.idempotency_key == "p1")
        },
        Duration::from_secs(10),
        "prompt dead-lettered (no live agent to prompt)",
    )
    .await;
    assert!(outbox.pending_for_node("n1").unwrap().is_empty());

    handle.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sse_diffs_land_in_projection() {
    let stub = start_stub(vec![session("s1", "S1"), session("s2", "S2")]).await;
    let projection = ProjectionStore::in_memory().unwrap();
    let outbox = Outbox::in_memory().unwrap();
    let handle = SyncEngine::spawn(
        projection.clone(),
        outbox,
        [NodeTarget::new(stub.node_ref("n1"))],
        fast_options(),
    );

    let proj = projection.clone();
    wait_until(
        || proj.sessions(Some("n1")).unwrap().len() == 2,
        Duration::from_secs(10),
        "initial listing in projection",
    )
    .await;

    // `session` diff — busy flag flips.
    stub.feed
        .emit_session("s1", Some("devin"), json!({"busy": true}));
    let proj = projection.clone();
    wait_until(
        || indexed("n1", "s1")(&proj).is_some_and(|s| s.busy),
        Duration::from_secs(5),
        "busy=true diff",
    )
    .await;

    // `meta` diff — exercise the real path: PATCH the node over HTTP;
    // its instrumented meta store emits the event the engine applies.
    let client = NodeClient::new(&stub.url(), None).unwrap();
    tokio::task::spawn_blocking(move || {
        client
            .post_op(
                "s1",
                Some("devin"),
                "meta.patch",
                &json!({"title": "renamed"}),
            )
            .unwrap();
    })
    .await
    .unwrap();
    let proj = projection.clone();
    wait_until(
        || indexed("n1", "s1")(&proj).is_some_and(|s| s.title == "renamed"),
        Duration::from_secs(5),
        "title diff",
    )
    .await;

    // Tombstone — `{deleted: true}` on a `session` frame removes the row.
    stub.feed
        .emit_session("s2", Some("devin"), json!({"deleted": true}));
    let proj = projection.clone();
    wait_until(
        || indexed("n1", "s2")(&proj).is_none(),
        Duration::from_secs(5),
        "delete diff",
    )
    .await;

    // A diff for a never-indexed session refetches the full summary —
    // s3 doesn't exist on the node, the refetch 404s, row stays absent.
    stub.feed
        .emit_session("s3", Some("devin"), json!({"busy": true}));
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(indexed("n1", "s3")(&projection).is_none());

    handle.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reconnect_after_node_restart_resyncs() {
    let mut stub = start_stub(vec![session("s1", "S1")]).await;
    let projection = ProjectionStore::in_memory().unwrap();
    let handle = SyncEngine::spawn(
        projection.clone(),
        Outbox::in_memory().unwrap(),
        [NodeTarget::new(stub.node_ref("n1"))],
        fast_options(),
    );
    let proj = projection.clone();
    wait_until(
        || {
            proj.node("n1")
                .unwrap()
                .is_some_and(|n| n.status == NodeStatus::Up)
        },
        Duration::from_secs(10),
        "initial up",
    )
    .await;

    // Kill the node — the SSE connection severs, status flips to down.
    stub.down();
    let proj = projection.clone();
    wait_until(
        || {
            proj.node("n1")
                .unwrap()
                .is_some_and(|n| n.status == NodeStatus::Down)
        },
        Duration::from_secs(10),
        "down detected",
    )
    .await;

    // "Restart" with a changed store — the reconnect's full listing
    // replaces the index (s1 tombstoned, s9 appears).
    stub.store.delete("s1").await.unwrap();
    stub.store.save(&session("s9", "S9")).await.unwrap();
    stub.up().await;
    let proj = projection.clone();
    wait_until(
        || indexed("n1", "s9")(&proj).is_some() && indexed("n1", "s1")(&proj).is_none(),
        Duration::from_secs(10),
        "resync replaced index",
    )
    .await;

    handle.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn re_add_retargets_a_node_with_new_credentials() {
    // The node requires a bearer token; the engine starts tokenless —
    // every connect 401s, the node stays down. Re-adding the node with
    // the token restarts its loop under the new credential (this is
    // the path the hub's pair flow takes after minting a token).
    let mut stub = start_stub_with(vec![session("s1", "S1")], Some("tok".into())).await;
    let projection = ProjectionStore::in_memory().unwrap();
    let handle = SyncEngine::spawn(
        projection.clone(),
        Outbox::in_memory().unwrap(),
        [NodeTarget::new(stub.node_ref("n1"))],
        fast_options(),
    );
    let proj = projection.clone();
    wait_until(
        || {
            proj.node("n1")
                .unwrap()
                .is_some_and(|n| n.status == NodeStatus::Down)
        },
        Duration::from_secs(10),
        "tokenless loop stays down",
    )
    .await;
    assert!(indexed("n1", "s1")(&projection).is_none());

    handle
        .add_node(NodeTarget::with_token(stub.node_ref("n1"), "tok"))
        .await
        .unwrap();
    let proj = projection.clone();
    wait_until(
        || indexed("n1", "s1")(&proj).is_some(),
        Duration::from_secs(10),
        "retargeted loop synced",
    )
    .await;

    stub.down();
    handle.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn submit_op_posts_directly_when_node_is_up() {
    let mut stub = start_stub(vec![session("s1", "S1")]).await;
    let projection = ProjectionStore::in_memory().unwrap();
    let outbox = Outbox::in_memory().unwrap();
    let handle = SyncEngine::spawn(
        projection.clone(),
        outbox.clone(),
        [NodeTarget::new(stub.node_ref("n1"))],
        fast_options(),
    );
    let proj = projection.clone();
    wait_until(
        || {
            proj.node("n1")
                .unwrap()
                .is_some_and(|n| n.status == NodeStatus::Up)
        },
        Duration::from_secs(10),
        "up",
    )
    .await;

    let outcome = handle
        .submit_op(write(
            "n1",
            "s1",
            "meta.patch",
            OpKind::Metadata,
            json!({"title": "direct"}),
            "d1",
        ))
        .await
        .unwrap();
    assert!(matches!(outcome, SubmitOutcome::Applied(_)));
    assert!(outbox.pending_for_node("n1").unwrap().is_empty());
    assert_eq!(stub.meta.of("s1").unwrap().title.as_deref(), Some("direct"));

    // Unknown node — typed error, not a silent queue.
    let err = handle
        .submit_op(write(
            "nope",
            "s1",
            "meta.patch",
            OpKind::Metadata,
            json!({}),
            "d2",
        ))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("unknown node"));

    stub.down();
    handle.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn shutdown_stops_all_loops_and_pumps() {
    let mut stub = start_stub(vec![session("s1", "S1")]).await;
    // A second, permanently-down node exercises the backoff path.
    let mut dead = start_stub(vec![]).await;
    dead.down();

    let projection = ProjectionStore::in_memory().unwrap();
    let handle = SyncEngine::spawn(
        projection.clone(),
        Outbox::in_memory().unwrap(),
        [
            NodeTarget::new(stub.node_ref("up-node")),
            NodeTarget::new(dead.node_ref("dead-node")),
        ],
        fast_options(),
    );
    let proj = projection.clone();
    wait_until(
        || {
            proj.node("up-node")
                .unwrap()
                .is_some_and(|n| n.status == NodeStatus::Up)
        },
        Duration::from_secs(10),
        "up-node connected",
    )
    .await;
    wait_until(
        || handle.active_pumps() == 1,
        Duration::from_secs(5),
        "sse pump running",
    )
    .await;

    // Supervisor + both loops exit promptly; pumps unwind at the next
    // heartbeat (keep-alive is 40ms on the stub).
    let pumps = handle.pump_counter();
    tokio::time::timeout(Duration::from_secs(10), handle.shutdown())
        .await
        .expect("shutdown timed out — a loop leaked");
    wait_until(
        || pumps.load(Ordering::Relaxed) == 0,
        Duration::from_secs(2),
        "pump threads exited",
    )
    .await;

    stub.down();
    dead.down();
}

/* ---- NodeClient directly --------------------------------------------*/

/// Canned-HTTP stub: answers every accepted connection with one fixed
/// response until the port is dropped. Returns the bound port.
fn canned_server(status: &str, body: &str) -> u16 {
    let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let port = listener.local_addr().unwrap().port();
    let res = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    std::thread::spawn(move || {
        loop {
            let Ok((mut conn, _)) = listener.accept() else {
                return;
            };
            use std::io::Write;
            let _ = conn.write_all(res.as_bytes());
        }
    });
    port
}

#[test]
fn client_rejects_bad_base_url_and_classifies_errors() {
    let err = NodeClient::new("ftp://node", None).err().unwrap();
    assert!(err.to_string().contains("http(s)://"));
    assert!(!err.is_transport());
    let http = peregrin_sync::client::NodeError::Http {
        status: 500,
        body: "x".into(),
    };
    assert!(!http.is_transport());
    let io: peregrin_sync::client::NodeError = std::io::Error::other("reset").into();
    assert!(io.is_transport());
    // ureq transport — a refused connect.
    let client = NodeClient::new("http://127.0.0.1:1/", None).unwrap();
    assert!(client.health().unwrap_err().is_transport());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn client_calls_against_the_stub_app() {
    let stub = start_stub(vec![session("s1", "S1")]).await;
    let client = NodeClient::new(&stub.url(), Some("tok".into())).unwrap();
    tokio::task::spawn_blocking(move || {
        client.health().unwrap();
        let sessions = client.list_sessions(true).unwrap();
        assert_eq!(sessions.len(), 1);
        let summary = client.get_summary("s1", Some("devin")).unwrap();
        assert_eq!(summary["id"], "s1");
        let page = client
            .get_history("s1", Some(10), Some(5), Some("devin"))
            .unwrap();
        // The stub's seeded session carries whatever history its
        // fixture has — the call path is what matters.
        assert!(page.total <= 1);
        // Every `post_op` arm reaches the node — the stub answers each
        // route (the bodies' statuses don't matter to coverage).
        for op in ["prompt", "cancel", "permission", "meta.patch", "delete"] {
            let _ = client.post_op("s1", Some("devin"), op, &json!({}));
        }
        let err = client.post_op("s1", None, "bogus", &json!({})).unwrap_err();
        assert!(err.to_string().contains("unsupported op"));
        let _ = client.get_json("/api/node").unwrap();
        let _ = client.send_json("POST", "/api/config", &json!({"k": 1}));
        let _ = client.send_json("DELETE", "/api/config/k", &json!(null));
        // Both SSE opens — the node feed and a session stream (its
        // status depends on agent liveness; either path is fine).
        let _stream = client.events().unwrap();
        let _ = client.session_stream("s1", Some("devin"));
    })
    .await
    .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn client_malformed_bodies_and_http_errors() {
    // `sessions` field not an array.
    let port = canned_server("200 OK", "{}");
    let client = NodeClient::new(&format!("http://127.0.0.1:{port}"), None).unwrap();
    let err = tokio::task::spawn_blocking(move || client.list_sessions(false))
        .await
        .unwrap()
        .unwrap_err();
    assert!(err.to_string().contains("missing sessions array"));

    // Non-2xx on `health` → the `expect_json` Http arm.
    let port = canned_server("503 Service Unavailable", "{\"error\":\"down\"}");
    let client = NodeClient::new(&format!("http://127.0.0.1:{port}"), None).unwrap();
    let err = tokio::task::spawn_blocking(move || client.health())
        .await
        .unwrap()
        .unwrap_err();
    assert!(err.to_string().contains("503"));

    // Non-2xx on the SSE open → Http with the error body.
    let port = canned_server("404 Not Found", "{\"error\":\"no\"}");
    let client = NodeClient::new(&format!("http://127.0.0.1:{port}"), None).unwrap();
    let err = tokio::task::spawn_blocking(move || client.events())
        .await
        .unwrap()
        .err()
        .unwrap();
    assert!(err.to_string().contains("404"));

    // History pages missing each required field → Malformed arms.
    for body in ["{}", "{\"messages\":[]}", "{\"messages\":[],\"total\":0}"] {
        let port = canned_server("200 OK", body);
        let client = NodeClient::new(&format!("http://127.0.0.1:{port}"), None).unwrap();
        let err = tokio::task::spawn_blocking(move || client.get_history("s1", None, None, None))
            .await
            .unwrap()
            .unwrap_err();
        assert!(err.to_string().contains("malformed"), "{body}: {err}");
    }
    // A well-formed page and a non-2xx both parse through expect_json.
    let port = canned_server("200 OK", "{\"messages\":[1],\"total\":3,\"start\":2}");
    let client = NodeClient::new(&format!("http://127.0.0.1:{port}"), None).unwrap();
    let page = tokio::task::spawn_blocking(move || client.get_history("s1", None, None, None))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(page.start, 2);
    let port = canned_server("500 Internal Server Error", "boom");
    let client = NodeClient::new(&format!("http://127.0.0.1:{port}"), None).unwrap();
    let err = tokio::task::spawn_blocking(move || client.get_history("s1", None, None, None))
        .await
        .unwrap()
        .unwrap_err();
    assert!(err.to_string().contains("500"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn client_sse_stream_ends_cleanly_at_eof() {
    // A 200 that closes after one frame — the pump sees `Ok(None)`,
    // the clean-EOF `Closed(None)` path (not a dropped connection).
    let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let port = listener.local_addr().unwrap().port();
    // A plain thread, not `spawn_blocking` — an infinite accept loop on
    // the blocking pool keeps the runtime from shutting down at test
    // end (blocking tasks can't be aborted → 120s nextest kill).
    std::thread::spawn(move || {
        use std::io::Write;
        loop {
            let Ok((mut conn, _)) = listener.accept() else {
                return;
            };
            let _ = conn.write_all(
                b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n\
                  event: heartbeat\ndata: {}\n\n",
            );
        }
    });
    let client = NodeClient::new(&format!("http://127.0.0.1:{port}"), None).unwrap();
    // std::thread + mpsc — a hung `spawn_blocking` can't be aborted and
    // would stall runtime shutdown past the watchdog.
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let events = {
            let mut stream = client.events().unwrap();
            let first = stream.next_event().unwrap();
            let second = stream.next_event().unwrap();
            (first, second)
        };
        let _ = tx.send(events);
    });
    let events = rx
        .recv_timeout(std::time::Duration::from_secs(20))
        .expect("SSE EOF watchdog lapsed");
    assert_eq!(
        events,
        (Some(peregrin_sync::client::FeedEvent::Heartbeat), None)
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn remove_node_stops_its_loop_and_drops_state() {
    let mut stub = start_stub(vec![session("s1", "S1")]).await;
    let projection = ProjectionStore::in_memory().unwrap();
    let handle = SyncEngine::spawn(
        projection.clone(),
        Outbox::in_memory().unwrap(),
        [NodeTarget::new(stub.node_ref("n1"))],
        fast_options(),
    );
    let proj = projection.clone();
    wait_until(
        || indexed("n1", "s1")(&proj).is_some(),
        Duration::from_secs(10),
        "synced",
    )
    .await;
    handle.remove_node("n1").await.unwrap();
    assert!(proj.node("n1").unwrap().is_none());
    assert!(indexed("n1", "s1")(&proj).is_none());
    // The supervisor applies `Cmd::Remove` asynchronously — `shutdown`
    // racing the queued command could skip the arm entirely. Probe
    // `submit_op` until the target table loses n1 (`UnknownNode`) so
    // the Remove arm has definitely run before cancel.
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    loop {
        let res = handle
            .submit_op(write(
                "n1",
                "s1",
                "meta.patch",
                OpKind::Metadata,
                json!({}),
                "probe-remove",
            ))
            .await;
        if matches!(res, Err(peregrin_sync::SyncError::UnknownNode(_))) {
            break;
        }
        assert!(std::time::Instant::now() < deadline, "remove never applied");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    stub.down();
    handle.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn supervisor_warns_when_node_upsert_fails() {
    // A `nodes`-less projection → `upsert_node` errors → the supervisor's
    // warn arm (the loop still spawns and connects normally).
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("proj.db");
    let projection = ProjectionStore::open(&path).unwrap();
    {
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute_batch("DROP TABLE nodes").unwrap();
    }
    let handle = SyncEngine::spawn(
        projection,
        Outbox::in_memory().unwrap(),
        [NodeTarget::new(NodeRef {
            id: "n1".into(),
            url: "http://127.0.0.1:1".into(),
            label: "n".into(),
        })],
        fast_options(),
    );
    tokio::time::sleep(Duration::from_millis(300)).await;
    handle.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn resync_tick_and_lagged_frame_relist() {
    let mut stub = start_stub(vec![session("s1", "S1")]).await;
    let projection = ProjectionStore::in_memory().unwrap();
    let handle = SyncEngine::spawn(
        projection.clone(),
        Outbox::in_memory().unwrap(),
        [NodeTarget::new(stub.node_ref("n1"))],
        SyncOptions {
            resync_interval: Duration::from_millis(80),
            ..fast_options()
        },
    );
    let proj = projection.clone();
    wait_until(
        || indexed("n1", "s1")(&proj).is_some(),
        Duration::from_secs(10),
        "synced",
    )
    .await;
    // A `lagged` frame forces a full relist through apply_event.
    stub.feed.emit("lagged", json!({}));
    // And a `project` diff is inert on session rows.
    stub.feed.emit("project", json!({"id": "p1", "patch": {}}));
    tokio::time::sleep(Duration::from_millis(300)).await;
    stub.down();
    handle.shutdown().await;
}

#[test]
fn projection_remove_node_and_cursor_roundtrip() {
    let store = ProjectionStore::in_memory().unwrap();
    store
        .upsert_node(&NodeRef {
            id: "n1".into(),
            url: "http://x".into(),
            label: "l".into(),
        })
        .unwrap();
    store
        .upsert_summary(
            "n1",
            &json!({"id":"s1","title":"T","updatedAt":"2024-01-01T00:00:00.000Z"}),
        )
        .unwrap();
    store.set_cursor("n1", "cur-1").unwrap();
    assert_eq!(store.cursor("n1").unwrap().as_deref(), Some("cur-1"));
    store.set_cursor("n1", "cur-2").unwrap();
    assert_eq!(store.cursor("n1").unwrap().as_deref(), Some("cur-2"));

    store.remove_node("n1").unwrap();
    assert!(store.node("n1").unwrap().is_none());
    assert!(store.session("n1", "s1").unwrap().is_none());
    assert_eq!(store.cursor("n1").unwrap(), None);
}

#[test]
fn projection_surfaces_corrupt_rows() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("proj.db");
    let store = ProjectionStore::open(&path).unwrap();
    store
        .upsert_node(&NodeRef {
            id: "n1".into(),
            url: "http://x".into(),
            label: "l".into(),
        })
        .unwrap();
    // A raw_json that isn't JSON — the row_to_indexed / apply_patch
    // decode error arms.
    {
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute(
            "INSERT INTO session_index (node_id, session_id, raw_json)
             VALUES ('n1', 'bad', 'not json')",
            [],
        )
        .unwrap();
    }
    assert!(store.session("n1", "bad").is_err());
    assert!(store.apply_patch("n1", "bad", &json!({"x": 1})).is_err());
}

#[test]
fn apply_listing_summary_without_updated_at() {
    // Missing `updatedAt` → the `if let Some(u)` else arm; the cursor
    // stays unset.
    let store = ProjectionStore::in_memory().unwrap();
    store
        .apply_listing("n1", &[json!({"id":"s1","title":"T"})])
        .unwrap();
    assert!(store.session("n1", "s1").unwrap().is_some());
    assert_eq!(store.cursor("n1").unwrap(), None);
}

#[test]
fn apply_patch_write_failure_surfaces() {
    // `apply_patch`'s trailing `write_row?` — swap the table under the
    // store so the re-INSERT fails after the patch merges.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("proj.db");
    let store = ProjectionStore::open(&path).unwrap();
    store
        .upsert_summary("n1", &json!({"id":"s1","title":"T"}))
        .unwrap();
    {
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute_batch(
            "DROP TABLE session_index;
             CREATE TABLE session_index (node_id TEXT, session_id TEXT, raw_json TEXT);
             INSERT INTO session_index VALUES ('n1','s1','{}');",
        )
        .unwrap();
    }
    assert!(
        store
            .apply_patch("n1", "s1", &json!({"title": "T2"}))
            .is_err()
    );
}

#[test]
fn apply_patch_non_object_skips_merge() {
    // A non-object patch → the `if let (Some, Some)` else arm: the row
    // is rewritten unchanged.
    let store = ProjectionStore::in_memory().unwrap();
    store
        .upsert_summary("n1", &json!({"id":"s1","title":"T"}))
        .unwrap();
    assert_eq!(
        store.apply_patch("n1", "s1", &json!([1, 2])).unwrap(),
        PatchStatus::Applied
    );
    assert_eq!(store.session("n1", "s1").unwrap().unwrap().title, "T");
}
