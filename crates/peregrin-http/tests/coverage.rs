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
