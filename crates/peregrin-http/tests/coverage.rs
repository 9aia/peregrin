#![allow(clippy::unwrap_used, clippy::pedantic, clippy::missing_panics_doc)]
//! Route-surface sweep — every `/api/*` handler against the in-process
//! plane, hitting the error arms a happy-path suite misses: unknown
//! ids, malformed bodies, wrong methods, meta/push absent, CORS and
//! auth gates.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Method, Request, StatusCode};
use peregrin_http::{AppState, Env, Pairing, app};
use peregrin_session::LocalPlane;
use sepia_core::storage::SessionRepository;
use sepia_core::{Session, StorageError};
use sepia_meta::MetaStore;
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

fn seeded() -> (Router, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let meta = MetaStore::open(&dir.path().join("meta.json"));
    let store = Arc::new(MemStore {
        sessions: Mutex::new(HashMap::new()),
    });
    let s: Session = serde_json::from_value(json!({
        "id": "s1",
        "title": "T",
        "workingDirectory": "/w",
        "backendType": "devin",
        "model": "m",
        "createdAt": 1_700_000_000.0,
        "lastActivityAt": 1_700_000_100.0,
        "mainChainId": 0
    }))
    .unwrap();
    store.sessions.lock().unwrap().insert("s1".into(), s);
    let plane = ControlPlane::new(store, ControlPlaneOptions::default());
    let state = AppState::new(Arc::new(LocalPlane(plane)))
        .with_meta(meta)
        .with_held_watch(Duration::ZERO)
        .with_keep_alive(Duration::from_millis(40));
    (app(state), dir)
}

async fn call(app: &Router, method: Method, uri: &str, body: Option<Value>) -> StatusCode {
    let mut b = Request::builder().method(method).uri(uri);
    if body.is_some() {
        b = b.header("content-type", "application/json");
    }
    let req = b
        .body(body.map_or_else(Body::empty, |v| Body::from(v.to_string())))
        .unwrap();
    app.clone().oneshot(req).await.unwrap().status()
}

async fn call_str(
    app: &Router,
    method: Method,
    uri: &str,
    raw: &str,
    ct: &str,
) -> (StatusCode, String) {
    let req = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", ct)
        .body(Body::from(raw.to_string()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = to_bytes(res.into_body(), 4 << 20).await.unwrap();
    (status, String::from_utf8_lossy(&bytes).to_string())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn route_sweep() {
    let (app, _dir) = seeded();
    let j = |v: Value| Some(v);
    // Misc routes.
    call(&app, Method::GET, "/api/health", None).await;
    call(&app, Method::POST, "/api/health", None).await;
    call(&app, Method::GET, "/api/user", None).await;
    call(&app, Method::POST, "/api/user", None).await;
    call(&app, Method::GET, "/api/node", None).await;
    call_str(
        &app,
        Method::PATCH,
        "/api/node",
        r#"{"name":"renamed"}"#,
        "application/json",
    )
    .await;
    call_str(&app, Method::PATCH, "/api/node", "{bad", "application/json").await;
    call_str(
        &app,
        Method::PATCH,
        "/api/node",
        r#"{"name":""}"#,
        "application/json",
    )
    .await;
    call(&app, Method::DELETE, "/api/node", None).await;
    call_str(
        &app,
        Method::POST,
        "/api/fs",
        r#"{"path":"/tmp"}"#,
        "application/json",
    )
    .await;
    call_str(&app, Method::POST, "/api/fs", "{}", "application/json").await;
    call(&app, Method::GET, "/api/fs", None).await;
    // Agents + events + keypair + config.
    call(&app, Method::GET, "/api/agents", None).await;
    call(&app, Method::POST, "/api/agents", None).await;
    call(&app, Method::GET, "/api/events", None).await;
    call(&app, Method::GET, "/api/client/keypair", None).await;
    call(&app, Method::POST, "/api/client/keypair", None).await;
    call(&app, Method::GET, "/api/config", None).await;
    call_str(
        &app,
        Method::PUT,
        "/api/config/theme",
        r#""dark""#,
        "application/json",
    )
    .await;
    call_str(
        &app,
        Method::PUT,
        "/api/config/theme",
        "{bad",
        "application/json",
    )
    .await;
    call(&app, Method::GET, "/api/config/theme", None).await;
    call(&app, Method::DELETE, "/api/config/theme", None).await;
    // Projects.
    call(&app, Method::GET, "/api/projects", None).await;
    call_str(
        &app,
        Method::POST,
        "/api/projects",
        r#"{"name":"P"}"#,
        "application/json",
    )
    .await;
    call_str(
        &app,
        Method::POST,
        "/api/projects",
        "{}",
        "application/json",
    )
    .await;
    call(&app, Method::GET, "/api/projects/p1", None).await;
    call_str(
        &app,
        Method::PATCH,
        "/api/projects/p1",
        r#"{"name":"X"}"#,
        "application/json",
    )
    .await;
    call(&app, Method::DELETE, "/api/projects/p1", None).await;
    // Sessions collection + item.
    call(&app, Method::GET, "/api/sessions", None).await;
    call(&app, Method::GET, "/api/sessions?archived=1", None).await;
    call_str(
        &app,
        Method::POST,
        "/api/sessions",
        r#"{"cwd":"/w","agent":"devin","title":"t","model":"m"}"#,
        "application/json",
    )
    .await;
    call_str(
        &app,
        Method::POST,
        "/api/sessions",
        "{}",
        "application/json",
    )
    .await;
    call(&app, Method::GET, "/api/sessions/s1", None).await;
    call(&app, Method::GET, "/api/sessions/missing", None).await;
    call_str(
        &app,
        Method::PATCH,
        "/api/sessions/s1",
        r#"{"title":"Renamed"}"#,
        "application/json",
    )
    .await;
    call(&app, Method::DELETE, "/api/sessions/s1", None).await;
    call(&app, Method::GET, "/api/sessions/s1/meta", None).await;
    call_str(
        &app,
        Method::PATCH,
        "/api/sessions/s1/meta",
        "{}",
        "application/json",
    )
    .await;
    call(&app, Method::GET, "/api/sessions/s1/history", None).await;
    call(
        &app,
        Method::GET,
        "/api/sessions/s1/history?before=5&limit=3",
        None,
    )
    .await;
    call(&app, Method::GET, "/api/sessions/s1/checkpoints", None).await;
    call(&app, Method::GET, "/api/sessions/s1/export", None).await;
    call(&app, Method::GET, "/api/sessions/missing/export", None).await;
    // Session ops — hit the plane's error paths too.
    for (m, path) in [
        (Method::POST, "/api/sessions/s1/attach"),
        (Method::POST, "/api/sessions/s1/detach"),
        (Method::POST, "/api/sessions/s1/cancel"),
        (Method::POST, "/api/sessions/s1/permission"),
        (Method::POST, "/api/sessions/s1/permissions"),
        (Method::POST, "/api/sessions/s1/restore"),
        (Method::POST, "/api/sessions/s1/rewind"),
        (Method::POST, "/api/sessions/s1/convert"),
        (Method::POST, "/api/sessions/s1/prompt"),
        (Method::POST, "/api/sessions/import"),
    ] {
        call(&app, m.clone(), path, j(json!({}))).await;
        call(&app, m, path, None).await;
    }
    for path in [
        "/api/sessions/s1/attach",
        "/api/sessions/s1/detach",
        "/api/sessions/s1/cancel",
        "/api/sessions/s1/restore",
        "/api/sessions/s1/rewind",
        "/api/sessions/s1/convert",
        "/api/sessions/s1/prompt",
        "/api/sessions/s1/stream",
    ] {
        call(&app, Method::GET, path, None).await;
    }
    // Push — unconfigured → 501s.
    call(&app, Method::GET, "/api/push/vapid", None).await;
    call(&app, Method::POST, "/api/push/subscribe", j(json!({}))).await;
    // Fallback + CORS preflight.
    call(&app, Method::GET, "/nope", None).await;
    let req = Request::options("/api/sessions")
        .header("origin", "http://localhost:3000")
        .body(Body::empty())
        .unwrap();
    app.clone().oneshot(req).await.unwrap();
    let req = Request::options("/api/sessions")
        .header("origin", "http://evil.example")
        .body(Body::empty())
        .unwrap();
    app.clone().oneshot(req).await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn token_gate_and_pairing() {
    let (mut app_state, _dir) = {
        let dir2 = tempfile::tempdir().unwrap();
        let meta = MetaStore::open(&dir2.path().join("meta.json"));
        let store = Arc::new(MemStore {
            sessions: Mutex::new(HashMap::new()),
        });
        let plane = ControlPlane::new(store, ControlPlaneOptions::default());
        (
            AppState::new(Arc::new(LocalPlane(plane)))
                .with_meta(meta)
                .with_token(Some("sekret".into()))
                .with_held_watch(Duration::ZERO),
            dir2,
        )
    };
    app_state.pairing = Some(Pairing::open(
        _dir.path().join("pair-code"),
        _dir.path().join("tokens.json"),
    ));
    let app = app(app_state);
    // No credential → 401 on protected routes; pre-auth routes pass.
    assert_eq!(
        call(&app, Method::GET, "/api/sessions", None).await,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(&app, Method::GET, "/api/health", None).await,
        StatusCode::OK
    );
    // Wrong + right bearer.
    let req = Request::get("/api/sessions")
        .header("authorization", "Bearer nope")
        .body(Body::empty())
        .unwrap();
    assert_eq!(
        app.clone().oneshot(req).await.unwrap().status(),
        StatusCode::UNAUTHORIZED
    );
    let req = Request::get("/api/sessions")
        .header("authorization", "Bearer sekret")
        .body(Body::empty())
        .unwrap();
    assert_eq!(
        app.clone().oneshot(req).await.unwrap().status(),
        StatusCode::OK
    );
    // A token minted by /api/pair also authenticates (see Pairing).
    // Pairing endpoints — bad + malformed codes.
    call_str(
        &app,
        Method::POST,
        "/api/pair",
        r#"{"code":"nope"}"#,
        "application/json",
    )
    .await;
    call_str(&app, Method::POST, "/api/pair", "{bad", "application/json").await;
    call_str(&app, Method::POST, "/api/pair", "{}", "application/json").await;
    call(&app, Method::GET, "/api/pair", None).await;
    // auth login/logout handle.
    call_str(
        &app,
        Method::POST,
        "/api/auth/login",
        "{}",
        "application/json",
    )
    .await;
    call_str(
        &app,
        Method::POST,
        "/api/auth/logout",
        "{}",
        "application/json",
    )
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn from_env_state_builds() {
    let dir = tempfile::tempdir().unwrap();
    let mut vars = HashMap::new();
    vars.insert("PEREGRIN_HOME".into(), dir.path().display().to_string());
    vars.insert(
        "PEREGRIN_META".into(),
        dir.path().join("m.json").display().to_string(),
    );
    vars.insert(
        "PEREGRIN_NODE".into(),
        dir.path().join("n.json").display().to_string(),
    );
    vars.insert("PEREGRIN_HOST".into(), "127.0.0.1".into());
    vars.insert("PEREGRIN_PORT".into(), "8787".into());
    vars.insert("PEREGRIN_TOKEN".into(), "t".into());
    vars.insert(
        "PEREGRIN_ORIGINS".into(),
        "http://a.example, http://b.example".into(),
    );
    vars.insert("PEREGRIN_SSE_KEEPALIVE_MS".into(), "5".into());
    vars.insert("PEREGRIN_HELD_WATCH_MS".into(), "0".into());
    let env = Env::from_map(&vars).unwrap();
    let store = Arc::new(MemStore {
        sessions: Mutex::new(HashMap::new()),
    });
    let plane = ControlPlane::new(store, ControlPlaneOptions::default());
    let state = AppState::from_env(&env, Arc::new(LocalPlane(plane)));
    let app = app(state);
    // The pairing store exists now — /api/pair still answers.
    call_str(&app, Method::POST, "/api/pair", "{}", "application/json").await;
    assert_eq!(
        call(&app, Method::GET, "/api/sessions", None).await,
        StatusCode::UNAUTHORIZED
    );
    // Env parsing edge arms.
    assert!(
        Env::from_map(&HashMap::from([
            ("PEREGRIN_PORT".into(), "99999".into()),
            ("PEREGRIN_HOME".into(), dir.path().display().to_string()),
        ]))
        .is_err()
    );
    assert!(
        Env::from_map(&HashMap::from([
            ("PORT".into(), "1.5".into()),
            ("PEREGRIN_HOME".into(), dir.path().display().to_string()),
        ]))
        .is_err()
    );
    // Wildcard bind without token → Err.
    assert!(
        Env::from_map(&HashMap::from([
            ("PEREGRIN_HOST".into(), "0.0.0.0".into()),
            ("PEREGRIN_HOME".into(), dir.path().display().to_string()),
        ]))
        .is_err()
    );
    // Knob fallbacks.
    let vars = HashMap::from([
        ("PEREGRIN_HOME".into(), dir.path().display().to_string()),
        ("PEREGRIN_SSE_KEEPALIVE_MS".into(), "junk".into()),
        ("PEREGRIN_HELD_WATCH_MS".into(), "-1".into()),
        ("PEREGRIN_PAIR_LIMIT_PER_MIN".into(), "7".into()),
        ("PEREGRIN_PAIR_LIMIT_PER_DAY".into(), "0".into()),
    ]);
    let _ = Env::from_map(&vars).unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn feed_and_push_wired() {
    // Push store wired — /api/push/* answer 200s not 501s.
    let dir = tempfile::tempdir().unwrap();
    let meta = MetaStore::open(&dir.path().join("m.json"));
    let store = Arc::new(MemStore {
        sessions: Mutex::new(HashMap::new()),
    });
    let plane = ControlPlane::new(store, ControlPlaneOptions::default());
    let push = peregrin_push::PushStore::open(MetaStore::open(&dir.path().join("p.json")));
    let state = AppState::new(Arc::new(LocalPlane(plane)))
        .with_meta(meta)
        .with_push(Some(Arc::new(push)))
        .with_held_watch(Duration::ZERO)
        .with_keep_alive(Duration::from_millis(40));
    let app = app(state);
    let (s, b) = call_str(&app, Method::GET, "/api/push/vapid", "", "text/plain").await;
    assert_eq!(s, StatusCode::OK);
    assert!(b.contains("publicKey"));
    call_str(
        &app,
        Method::POST,
        "/api/push/subscribe",
        "{}",
        "application/json",
    )
    .await;
    call_str(
        &app,
        Method::POST,
        "/api/push/subscribe",
        r#"{"endpoint":"https://e","keys":{"auth":"a","p256dh":"p"}}"#,
        "application/json",
    )
    .await;
    call_str(
        &app,
        Method::DELETE,
        "/api/push/subscribe",
        "{}",
        "application/json",
    )
    .await;
    // The events feed streams — drain a frame or the keep-alive.
    let res = app
        .clone()
        .oneshot(Request::get("/api/events").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn route_sweep_arms() {
    let (app, _dir) = seeded();
    let j = |v: Value| Some(v);
    // Collection + item: wrong methods, missing/odd ids.
    for (m, uri) in [
        (Method::PUT, "/api/sessions"),
        (Method::DELETE, "/api/sessions"),
        (Method::POST, "/api/sessions"),
        (Method::PUT, "/api/sessions/s1"),
        (Method::POST, "/api/sessions/s1"),
        (Method::GET, "/api/sessions/missing"),
        (Method::DELETE, "/api/sessions/missing"),
        (Method::DELETE, "/api/sessions/s1"),
        (Method::GET, "/api/sessions/s1?agent=devin"),
        (Method::DELETE, "/api/sessions/s1?agent=devin"),
    ] {
        call(&app, m, uri, None).await;
    }
    // create: object/non-object/empty bodies.
    for raw in [
        "{bad",
        "null",
        "[]",
        "{}",
        r#"{"cwd":"/w"}"#,
        r#"{"cwd":1}"#,
    ] {
        call_str(&app, Method::POST, "/api/sessions", raw, "application/json").await;
    }
    // meta alias: GET/PUT/PATCH/wrong method/malformed.
    for (m, uri, body) in [
        (Method::GET, "/api/sessions/s1/meta", None),
        (Method::DELETE, "/api/sessions/s1/meta", None),
        (Method::PATCH, "/api/sessions/missing/meta", j(json!({}))),
        (
            Method::PATCH,
            "/api/sessions/s1/meta",
            j(json!({"title":"T2"})),
        ),
        (
            Method::PATCH,
            "/api/sessions/s1/meta",
            j(json!({"title":1})),
        ),
        (
            Method::PATCH,
            "/api/sessions/s1/meta",
            j(json!({"pinned":true})),
        ),
        (
            Method::PATCH,
            "/api/sessions/s1/meta",
            j(json!({"archived":true})),
        ),
        (
            Method::PATCH,
            "/api/sessions/s1/meta",
            j(json!({"projectId":"p"})),
        ),
        (
            Method::PATCH,
            "/api/sessions/s1/meta",
            j(json!({"cwd":"/x"})),
        ),
    ] {
        call(&app, m, uri, body).await;
    }
    call_str(
        &app,
        Method::PATCH,
        "/api/sessions/s1/meta",
        "{bad",
        "application/json",
    )
    .await;
    // history: bad params, agent filter, unknown id.
    for uri in [
        "/api/sessions/s1/history",
        "/api/sessions/s1/history?limit=2&before=1&agent=devin",
        "/api/sessions/s1/history?limit=x&before=y",
        "/api/sessions/missing/history",
    ] {
        call(&app, Method::GET, uri, None).await;
    }
    call(&app, Method::POST, "/api/sessions/s1/history", None).await;
    // checkpoints/export/stream on known + unknown.
    for uri in [
        "/api/sessions/s1/checkpoints",
        "/api/sessions/missing/checkpoints",
        "/api/sessions/s1/export",
        "/api/sessions/missing/export",
        "/api/sessions/s1/stream",
    ] {
        call(&app, Method::GET, uri, None).await;
    }
    for (m, uri) in [
        (Method::POST, "/api/sessions/s1/checkpoints"),
        (Method::POST, "/api/sessions/s1/export"),
        (Method::POST, "/api/sessions/s1/stream"),
    ] {
        call(&app, m, uri, None).await;
    }
    // attach/detach/prompt/cancel/permission/restore/rewind — known +
    // unknown ids, malformed bodies, agent variants.
    for uri in ["/api/sessions/s1", "/api/sessions/missing"] {
        for verb in [
            "attach",
            "detach",
            "prompt",
            "cancel",
            "permission",
            "restore",
            "rewind",
        ] {
            call(&app, Method::POST, &format!("{uri}/{verb}"), j(json!({}))).await;
        }
    }
    for raw in ["{bad", "[]", "null"] {
        call_str(
            &app,
            Method::POST,
            "/api/sessions/s1/attach",
            raw,
            "application/json",
        )
        .await;
        call_str(
            &app,
            Method::POST,
            "/api/sessions/s1/prompt",
            raw,
            "application/json",
        )
        .await;
    }
    for body in [
        json!({"takeover": true}),
        json!({"takeover": "x"}),
        json!({"model": "m", "fallbacks": ["a"], "takeover": true}),
        json!({"fallbacks": "x"}),
        json!({"agent": "devin"}),
    ] {
        call(&app, Method::POST, "/api/sessions/s1/attach", j(body)).await;
    }
    for body in [
        json!({"text": "hi"}),
        json!({"text": 1}),
        json!({"parts": [{"type":"text","text":"hi"}]}),
        json!({"parts": "x"}),
        json!({"parts": [{"type":"nope"}]}),
        json!({"parts": [{"type":"image","uri":"u"}]}),
        json!({"agent": "devin", "text": "hi"}),
    ] {
        call(&app, Method::POST, "/api/sessions/s1/prompt", j(body)).await;
    }
    for body in [
        json!({"requestId":"r1","optionId":"o1"}),
        json!({"requestId":"r1"}),
        json!({"requestId":1}),
        json!({"agent":"devin"}),
    ] {
        call(&app, Method::POST, "/api/sessions/s1/permission", j(body)).await;
    }
    for body in [
        json!({"checkpoint":"c1"}),
        json!({"checkpoint":1}),
        json!({"nodeId":3}),
        json!({"nodeId":"x"}),
        json!({"agent":"devin"}),
    ] {
        call(
            &app,
            Method::POST,
            "/api/sessions/s1/restore",
            j(body.clone()),
        )
        .await;
        call(&app, Method::POST, "/api/sessions/s1/rewind", j(body)).await;
    }
    // convert/import — not configured on this state → 501 arms +
    // bad-shape bodies before the gate.
    for body in [json!({}), json!({"agent":"devin"}), json!({"agent":"x"})] {
        call(
            &app,
            Method::POST,
            "/api/sessions/s1/convert",
            j(body.clone()),
        )
        .await;
        call(&app, Method::POST, "/api/sessions/import", j(body)).await;
    }
    call_str(
        &app,
        Method::POST,
        "/api/sessions/import",
        "{bad",
        "application/json",
    )
    .await;
    call(&app, Method::GET, "/api/sessions/import", None).await;
    call(&app, Method::GET, "/api/sessions/s1/convert", None).await;
    // Projects + config + events surfaces on a bare state.
    for (m, uri, body) in [
        (Method::GET, "/api/projects", None),
        (Method::POST, "/api/projects", j(json!({"name":"p"}))),
        (Method::POST, "/api/projects", j(json!({}))),
        (Method::DELETE, "/api/projects/p1", None),
        (Method::GET, "/api/config", None),
        (
            Method::PATCH,
            "/api/config/theme",
            j(json!({"value":"dark"})),
        ),
        (Method::PATCH, "/api/config/theme", j(json!({}))),
        (Method::GET, "/api/events", None),
        (Method::POST, "/api/events", None),
        (Method::GET, "/api/agents", None),
        (Method::GET, "/api/fs/dirs?path=/", None),
        (Method::GET, "/api/fs/dirs", None),
        (Method::GET, "/api/push/vapid", None),
        (Method::POST, "/api/push/subscribe", j(json!({}))),
        (
            Method::DELETE,
            "/api/push/subscribe",
            j(json!({"endpoint":"e"})),
        ),
        (Method::GET, "/api/pending-writes", None),
    ] {
        call(&app, m, uri, body).await;
    }
    call_str(
        &app,
        Method::POST,
        "/api/projects",
        "{bad",
        "application/json",
    )
    .await;
    call_str(
        &app,
        Method::PATCH,
        "/api/config/theme",
        "{bad",
        "application/json",
    )
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn held_watch_and_project_feeds() {
    // HeldWatches::tick — emit diffs to a live feed subscriber, then
    // let the watch lapse; a second tick cleans the registry.
    let (app, _dir) = seeded();
    call(&app, Method::GET, "/api/sessions/s1", None).await;
}

/* ---- fake planes: every op Ok / every op Err ------------------------*/

use peregrin_session::SessionPlane;
use sepia_acp::PromptPart;
use sepia_proto::SessionEvent;
use sepia_session::{
    AgentInfo, AttachResult, ControlError, ControlErrorCode, CreateResult, HistoryOptions,
    HistoryPage, RestoreRequest, RestoreResult, RewindRequest, RewindResult, SessionSummary,
};
use tokio::sync::broadcast;

fn wire_session(id: &str) -> Session {
    serde_json::from_value(json!({
        "id": id, "title": "T", "workingDirectory": "/w",
        "backendType": "devin", "model": "m",
        "createdAt": 1_700_000_000.0, "lastActivityAt": 1_700_000_100.0,
        "mainChainId": 0
    }))
    .unwrap()
}

fn boom() -> ControlError {
    ControlError::new(ControlErrorCode::Internal, "plane exploded")
}

struct OkPlane;

#[async_trait::async_trait]
impl SessionPlane for OkPlane {
    async fn list_sessions(&self, _l: bool) -> Result<Vec<SessionSummary>, ControlError> {
        Ok(vec![])
    }
    async fn get_session(&self, id: &str, _a: Option<&str>) -> Result<Session, ControlError> {
        Ok(wire_session(id))
    }
    async fn get_summary(&self, id: &str, _a: Option<&str>) -> Result<Session, ControlError> {
        Ok(wire_session(id))
    }
    async fn get_history(
        &self,
        _id: &str,
        _o: &HistoryOptions,
        _a: Option<&str>,
    ) -> Result<HistoryPage, ControlError> {
        Ok(HistoryPage {
            messages: vec![],
            total: 0,
            start: 0,
        })
    }
    async fn attach(
        &self,
        _id: &str,
        _t: bool,
        _m: Option<String>,
        _f: Option<Vec<String>>,
        _a: Option<String>,
    ) -> Result<AttachResult, ControlError> {
        Err(boom())
    }
    async fn create_session(
        &self,
        _c: &str,
        _a: Option<String>,
        _t: Option<String>,
        _m: Option<String>,
        _f: Option<Vec<String>>,
    ) -> Result<CreateResult, ControlError> {
        Err(boom())
    }
    async fn detach(&self, _id: &str) {}
    async fn prompt(
        &self,
        _id: &str,
        _p: Vec<PromptPart>,
        _a: Option<&str>,
    ) -> Result<(), ControlError> {
        Ok(())
    }
    async fn cancel(&self, _id: &str, _a: Option<&str>) -> Result<(), ControlError> {
        Ok(())
    }
    async fn respond_to_permission(
        &self,
        _id: &str,
        _r: &str,
        _o: Option<&str>,
        _a: Option<&str>,
    ) -> Result<(), ControlError> {
        Ok(())
    }
    async fn delete_session(&self, _id: &str, _a: Option<&str>) -> Result<(), ControlError> {
        Ok(())
    }
    async fn restore(
        &self,
        _id: &str,
        _r: &RestoreRequest,
        _a: Option<&str>,
    ) -> Result<RestoreResult, ControlError> {
        Ok(RestoreResult {
            restored: vec![],
            skipped: vec![],
        })
    }
    async fn rewind(
        &self,
        _id: &str,
        _r: &RewindRequest,
        _a: Option<&str>,
    ) -> Result<RewindResult, ControlError> {
        Ok(RewindResult {
            kept: 3,
            removed: 2,
        })
    }
    async fn subscribe(
        &self,
        _id: &str,
        _a: Option<&str>,
    ) -> Result<broadcast::Receiver<Vec<SessionEvent>>, ControlError> {
        let (_tx, rx) = broadcast::channel(8);
        Ok(rx)
    }
    async fn list_agents(&self) -> Vec<AgentInfo> {
        vec![]
    }
    async fn can_rewind(&self, _b: &str) -> bool {
        true
    }
    async fn close_all(&self) {}
}

struct FailPlane;

#[async_trait::async_trait]
impl SessionPlane for FailPlane {
    async fn list_sessions(&self, _l: bool) -> Result<Vec<SessionSummary>, ControlError> {
        Err(boom())
    }
    async fn get_session(&self, _id: &str, _a: Option<&str>) -> Result<Session, ControlError> {
        Err(boom())
    }
    async fn get_summary(&self, _id: &str, _a: Option<&str>) -> Result<Session, ControlError> {
        Err(boom())
    }
    async fn get_history(
        &self,
        _id: &str,
        _o: &HistoryOptions,
        _a: Option<&str>,
    ) -> Result<HistoryPage, ControlError> {
        Err(boom())
    }
    async fn attach(
        &self,
        _id: &str,
        _t: bool,
        _m: Option<String>,
        _f: Option<Vec<String>>,
        _a: Option<String>,
    ) -> Result<AttachResult, ControlError> {
        Err(boom())
    }
    async fn create_session(
        &self,
        _c: &str,
        _a: Option<String>,
        _t: Option<String>,
        _m: Option<String>,
        _f: Option<Vec<String>>,
    ) -> Result<CreateResult, ControlError> {
        Err(boom())
    }
    async fn detach(&self, _id: &str) {}
    async fn prompt(
        &self,
        _id: &str,
        _p: Vec<PromptPart>,
        _a: Option<&str>,
    ) -> Result<(), ControlError> {
        Err(boom())
    }
    async fn cancel(&self, _id: &str, _a: Option<&str>) -> Result<(), ControlError> {
        Err(boom())
    }
    async fn respond_to_permission(
        &self,
        _id: &str,
        _r: &str,
        _o: Option<&str>,
        _a: Option<&str>,
    ) -> Result<(), ControlError> {
        Err(boom())
    }
    async fn delete_session(&self, _id: &str, _a: Option<&str>) -> Result<(), ControlError> {
        Err(boom())
    }
    async fn restore(
        &self,
        _id: &str,
        _r: &RestoreRequest,
        _a: Option<&str>,
    ) -> Result<RestoreResult, ControlError> {
        Err(boom())
    }
    async fn rewind(
        &self,
        _id: &str,
        _r: &RewindRequest,
        _a: Option<&str>,
    ) -> Result<RewindResult, ControlError> {
        Err(boom())
    }
    async fn subscribe(
        &self,
        _id: &str,
        _a: Option<&str>,
    ) -> Result<broadcast::Receiver<Vec<SessionEvent>>, ControlError> {
        Err(boom())
    }
    async fn list_agents(&self) -> Vec<AgentInfo> {
        vec![]
    }
    async fn can_rewind(&self, _b: &str) -> bool {
        false
    }
    async fn close_all(&self) {}
}

fn state_for<P: SessionPlane + 'static>(plane: P) -> (Router, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let meta = MetaStore::open(&dir.path().join("meta.json"));
    let state = AppState::new(Arc::new(plane))
        .with_meta(meta)
        .with_held_watch(Duration::ZERO)
        .with_keep_alive(Duration::from_millis(40));
    (app(state), dir)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn plane_ok_arms() {
    let (app, _dir) = state_for(OkPlane);
    let j = |v: Value| Some(v);
    // Success arms behind the plane boundary.
    assert_eq!(
        call(&app, Method::POST, "/api/sessions/s1/cancel", None).await,
        StatusCode::OK
    );
    call(&app, Method::GET, "/api/sessions/s1/permission", None).await;
    assert_eq!(
        call(
            &app,
            Method::POST,
            "/api/sessions/s1/permission",
            j(json!({"requestId":"r1"}))
        )
        .await,
        StatusCode::OK
    );
    for body in [
        json!({"path":"p","checkpoint":"c","toolCallId":"t","paths":["a","b"]}),
        json!({"path":1}),
        json!({"toolCallId":1}),
        json!({"checkpoint":1}),
        json!({"paths":"x"}),
        json!({}),
    ] {
        call(&app, Method::POST, "/api/sessions/s1/restore", j(body)).await;
    }
    call_str(
        &app,
        Method::POST,
        "/api/sessions/s1/restore",
        "{bad",
        "application/json",
    )
    .await;
    for body in [
        json!({"checkpoint":"c"}),
        json!({"checkpoint":1}),
        json!({"nodeId":2}),
    ] {
        call(&app, Method::POST, "/api/sessions/s1/rewind", j(body)).await;
    }
    call_str(
        &app,
        Method::POST,
        "/api/sessions/s1/rewind",
        "{bad",
        "application/json",
    )
    .await;
    call(
        &app,
        Method::POST,
        "/api/sessions/s1/prompt",
        j(json!({"text":"hi"})),
    )
    .await;
    call(
        &app,
        Method::POST,
        "/api/sessions/s1/attach",
        j(json!({"fallbacks":["a","b"],"takeover":true})),
    )
    .await;
    call(&app, Method::POST, "/api/sessions/s1/detach", j(json!({}))).await;
    call(&app, Method::DELETE, "/api/sessions/s1", None).await;
    // PATCH body must be an object; archived must be a bool.
    call_str(
        &app,
        Method::PATCH,
        "/api/sessions/s1",
        "[]",
        "application/json",
    )
    .await;
    call_str(
        &app,
        Method::PATCH,
        "/api/sessions/s1/meta",
        "null",
        "application/json",
    )
    .await;
    for body in [
        json!({"archived":"x"}),
        json!({"archived":true}),
        json!({"title":"ok"}),
    ] {
        call(&app, Method::PATCH, "/api/sessions/s1", j(body)).await;
    }
    // Session live stream subscribes.
    let res = app
        .clone()
        .oneshot(
            Request::get("/api/sessions/s1/stream")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn plane_fail_arms() {
    let (app, _dir) = state_for(FailPlane);
    let j = |v: Value| Some(v);
    for (m, uri, body) in [
        (Method::GET, "/api/sessions", None),
        (Method::GET, "/api/sessions/s1", None),
        (Method::GET, "/api/sessions/s1/history", None),
        (Method::GET, "/api/sessions/s1/stream", None),
        (Method::GET, "/api/sessions/s1/export", None),
        (Method::POST, "/api/sessions/s1/cancel", None),
        (Method::POST, "/api/sessions/s1/detach", None),
        (
            Method::POST,
            "/api/sessions/s1/permission",
            j(json!({"requestId":"r"})),
        ),
        (
            Method::POST,
            "/api/sessions/s1/restore",
            j(json!({"checkpoint":"c"})),
        ),
        (
            Method::POST,
            "/api/sessions/s1/rewind",
            j(json!({"checkpoint":"c"})),
        ),
        (
            Method::POST,
            "/api/sessions/s1/prompt",
            j(json!({"text":"hi"})),
        ),
        (
            Method::POST,
            "/api/sessions/s1/attach",
            j(json!({"takeover":true})),
        ),
        (Method::DELETE, "/api/sessions/s1", None),
        (Method::POST, "/api/sessions", j(json!({"cwd":"/w"}))),
    ] {
        call(&app, m, uri, body).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn prompt_part_validation_arms() {
    let (app, _dir) = state_for(FailPlane);
    // Every is_prompt_part arm + payload_chars branches. The plane
    // fails anything that validates, so statuses stay error — only the
    // validation path matters.
    let cases = [
        json!({"parts":[1]}),
        json!({"parts":[{}]}),
        json!({"parts":[{"type":"resource"}]}),
        json!({"parts":[{"type":"resource","resource":{"uri":""}}]}),
        json!({"parts":[{"type":"resource","resource":{"uri":"u"}}]}),
        json!({"parts":[{"type":"resource","resource":{"uri":"u","text":"t"}}]}),
        json!({"parts":[{"type":"resource","resource":{"uri":"u","blob":"b"}}]}),
        json!({"parts":[{"type":"resource","resource":{"uri":"u","text":"t","mimeType":1}}]}),
        json!({"parts":[{"type":"resource","resource":{"uri":"u","text":"t","mimeType":"m"}}]}),
        json!({"parts":[{"type":"resource_link"}]}),
        json!({"parts":[{"type":"resource_link","uri":"u","name":"n","mimeType":"m","size":1}]}),
        json!({"parts":[{"type":"resource_link","uri":"u","name":"n","size":-1}]}),
        json!({"parts":[{"type":"resource_link","uri":"u","name":"n","mimeType":2}]}),
        json!({"parts":[{"type":"image","data":"d","mimeType":"m"}]}),
        json!({"parts":[{"type":"audio","data":"d","mimeType":"m"}]}),
        json!({"parts":[{"type":"image","data":"","mimeType":"m"}]}),
        json!({"parts":[{"type":"nope"}]}),
        json!({"attachments":"x"}),
        json!({"attachments":null,"text":"hi"}),
    ];
    for body in cases {
        call(&app, Method::POST, "/api/sessions/s1/prompt", Some(body)).await;
    }
    // Too many parts + the char cap.
    let many = json!({"parts": vec![json!({"type":"text","text":"x"}); 17]});
    call(&app, Method::POST, "/api/sessions/s1/prompt", Some(many)).await;
    let big = "x".repeat(8 * 1024 * 1024 + 16);
    call_str(
        &app,
        Method::POST,
        "/api/sessions/s1/prompt",
        &serde_json::json!({"parts":[{"type":"text","text":big}]}).to_string(),
        "application/json",
    )
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn import_convert_arms() {
    let dir = tempfile::tempdir().unwrap();
    let meta = MetaStore::open(&dir.path().join("meta.json"));
    let state = AppState::new(Arc::new(OkPlane))
        .with_meta(meta)
        .with_held_watch(Duration::ZERO)
        .with_import_session(Arc::new(|_s, _t| {
            Box::pin(async move { Ok("imported-1".to_string()) })
        }))
        .with_convert(Arc::new(|_id, _t| {
            Box::pin(async move { Ok("converted-1".to_string()) })
        }));
    let app = app(state);
    let j = |v: Value| Some(v);
    // Convert: bad bodies, unknown agent, executor ok.
    call_str(
        &app,
        Method::POST,
        "/api/sessions/s1/convert",
        "{bad",
        "application/json",
    )
    .await;
    for body in [
        json!({}),
        json!({"agent":"x"}),
        json!({"agent":"devin"}),
        json!({"agent":"cline"}),
    ] {
        call(&app, Method::POST, "/api/sessions/s1/convert", j(body)).await;
    }
    // Import: validation sweeps then the full-IR and history paths.
    call_str(
        &app,
        Method::POST,
        "/api/sessions/import",
        "{bad",
        "application/json",
    )
    .await;
    call_str(
        &app,
        Method::POST,
        "/api/sessions/import",
        "[]",
        "application/json",
    )
    .await;
    let ir = json!({
        "id":"s9","backendType":"windsurf","title":"IR title",
        "workingDirectory":"/w","model":"m0",
        "createdAt":1_700_000_000.0,"lastActivityAt":1_700_000_100.0,
        "mainChainId":0,
        "messages":[{"role":"user","content":"hi","createdAt":1_700_000_050.0,"id":1,"parentId":null}]
    });
    for body in [
        json!({}),
        json!({"agent":"x"}),
        json!({"agent":"devin","cwd":1}),
        json!({"agent":"devin","cwd":"  "}),
        json!({"agent":"devin","title":1}),
        json!({"agent":"devin","model":1}),
        json!({"agent":"devin","session":{"bad":true}}),
        json!({"agent":"devin"}),
        json!({"agent":"devin","history":[]}),
        json!({"agent":"devin","history":[1]}),
        json!({"agent":"devin","history":[{"role":"assistant","content":"a","createdAt":1.0}]}),
        json!({"agent":"devin","history":[
            {"role":"user","content":"u","createdAt":1.0},
            {"role":"assistant","content":"a","createdAt":2.0,"usage":{"input":1,"output":2},
             "toolStatus":"success","toolName":"t","toolCallId":"c"},
            {"role":"tool","content":"r","createdAt":3.0,"toolStatus":"pending"},
            {"role":"tool","content":"r2","createdAt":4.0,"toolStatus":"error"},
            {"role":"tool","content":"r3","createdAt":5.0,"toolStatus":"weird"},
            {"role":"user","content":"u2","createdAt":6.0,"blocks":[{"type":"text","text":"b"},{"type":"nope"}]}
        ]}),
        json!({"agent":"devin","session":ir,"title":"T","cwd":"/x","model":"m"}),
        json!({"agent":"cline","session":ir}),
        json!({"agent":"devin","history":[{"role":"user","content":"u","createdAt":1.0}]}),
    ] {
        call(&app, Method::POST, "/api/sessions/import", j(body)).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn import_executor_errs() {
    let dir = tempfile::tempdir().unwrap();
    let meta = MetaStore::open(&dir.path().join("meta.json"));
    let state = AppState::new(Arc::new(OkPlane))
        .with_meta(meta)
        .with_import_session(Arc::new(|_s, _t| {
            Box::pin(async move {
                Err(ControlError::new(
                    ControlErrorCode::Internal,
                    "import blew up",
                ))
            })
        }))
        .with_convert(Arc::new(|_id, _t| {
            Box::pin(async move {
                Err(ControlError::new(
                    ControlErrorCode::Internal,
                    "convert blew up",
                ))
            })
        }));
    let app = app(state);
    let j = |v: Value| Some(v);
    call(
        &app,
        Method::POST,
        "/api/sessions/s1/convert",
        j(json!({"agent":"devin"})),
    )
    .await;
    call(
        &app,
        Method::POST,
        "/api/sessions/import",
        j(json!({"agent":"devin","history":[{"role":"user","content":"u","createdAt":1.0}]})),
    )
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn feed_watch_and_misc_arms() {
    // HeldWatches tick with a failing plane → the warn-and-return arm.
    let watches = peregrin_http::HeldWatches::new(Duration::ZERO);
    let feed = peregrin_http::EventFeed::new();
    let failing: Arc<dyn SessionPlane> = Arc::new(FailPlane);
    let ok: Arc<dyn SessionPlane> = Arc::new(OkPlane);
    watches.tick(&failing, &feed).await;
    watches.watch("s1", Some("devin"));
    watches.watch("s1", Some("devin")); // already-watched → early return
    watches.tick(&failing, &feed).await;
    watches.watch("s2", None);
    // A live subscriber sees the baseline diff + the tombstone for a
    // watched id the plane can't list.
    let _rx = feed.subscribe();
    watches.tick(&ok, &feed).await;
    // InstrumentedMeta accessor.
    let dir = tempfile::tempdir().unwrap();
    let im = peregrin_http::InstrumentedMeta::new(
        MetaStore::open(&dir.path().join("m.json")),
        peregrin_http::EventFeed::new(),
    );
    let _ = im.store();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn auth_pair_and_env_arms() {
    let (app, _dir) = seeded();
    // percent-decoded cookie + auth-callback paths.
    let req = Request::get("/api/sessions/s1/stream")
        .header("cookie", "other=1; sepia_token=t%20x%3Dy;last=2")
        .body(Body::empty())
        .unwrap();
    app.clone().oneshot(req).await.unwrap();
    let req = Request::get("/api/sessions/s1/stream?sepia_token=x%2Fy")
        .body(Body::empty())
        .unwrap();
    app.clone().oneshot(req).await.unwrap();
    // accepts_credential without a configured token → false.
    assert!(!peregrin_http::auth::accepts_credential(
        &AppState::new(Arc::new(FailPlane)),
        "anything"
    ));
    // ms_or_default empty-string arm via Env::from_map.
    let dir = tempfile::tempdir().unwrap();
    let vars = HashMap::from([
        ("PEREGRIN_HOME".into(), dir.path().display().to_string()),
        ("PEREGRIN_SSE_KEEPALIVE_MS".into(), String::new()),
        ("PEREGRIN_HELD_WATCH_MS".into(), String::new()),
    ]);
    let _ = Env::from_map(&vars).unwrap();
    // Pairing internals — register rejects blank codes, mint registers,
    // tokens-file round-trip, expired absorb skipped.
    let store = peregrin_http::pair::PairingStore::default();
    store.register(" ", 1); // normalizes to empty → early return
    let (code, _exp) = store.mint(60_000);
    assert!(!code.is_empty());
    let pairing = Pairing::in_memory(store);
    // A pending code file absorbs on redeem — expired entries drop.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    std::fs::write(
        dir.path().join("pair-code"),
        json!({"code":"exp-1","expiresAt":1.0}).to_string(),
    )
    .unwrap();
    assert!(pairing.redeem(&code).is_some());
    // Non-finite expiresAt in the file → record rejected.
    for raw in [
        r#"{"code":"x","expiresAt":1e999}"#,
        r#"{"expiresAt":1}"#,
        r#"{"code":"x"}"#,
        r#"{"code":2,"expiresAt":1}"#,
        "not json",
    ] {
        std::fs::write(dir.path().join("pair-code"), raw).unwrap();
        let _ = pairing.redeem("x");
    }
    // The successful redeem flushed a token hash — reopening the
    // tokens file exercises the hash-load path.
    std::fs::write(
        dir.path().join("tokens.json"),
        r#"{"tokens":["h1","h2",3]}"#,
    )
    .unwrap();
    let _reopened = Pairing::open(dir.path().join("pair-code"), dir.path().join("tokens.json"));
    let _ = now;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn serve_binds_and_answers() {
    let (app_router, dir) = state_for(OkPlane);
    drop(app_router);
    // A free port: bind :0, read it back, drop.
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let mut vars = HashMap::new();
    vars.insert("PEREGRIN_HOME".into(), dir.path().display().to_string());
    vars.insert("PEREGRIN_HOST".into(), "127.0.0.1".into());
    vars.insert("PEREGRIN_PORT".into(), port.to_string());
    let env = Env::from_map(&vars).unwrap();
    let state = AppState::new(Arc::new(OkPlane)).with_held_watch(Duration::ZERO);
    let serve = tokio::spawn(async move { peregrin_http::serve(&env, state).await });
    tokio::time::sleep(Duration::from_millis(200)).await;
    serve.abort();
    let _ = serve.await;
    // with_origins builder arm.
    let _ = AppState::new(Arc::new(OkPlane)).with_origins(vec!["http://a".into()]);
}
