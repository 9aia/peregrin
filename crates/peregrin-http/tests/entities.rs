#![allow(clippy::unwrap_used, clippy::pedantic, clippy::missing_panics_doc)]

//! `/api/entities*` kind-registry tests — a fake `test-widget` plane
//! registered via `AppState::with_kind` proves the whole entity
//! surface (create/list/get/ops/delete/snapshot round-trip) works for
//! a non-sepia kind while the `sepia-session` path is unchanged.

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Method, Request, StatusCode};
use axum::response::{IntoResponse, Response};
use peregrin_http::{AppState, EntityPlane, app};
use peregrin_session::LocalPlane;
use sepia_core::storage::SessionRepository;
use sepia_core::{Session, StorageError};
use sepia_meta::MetaStore;
use sepia_session::{ControlPlane, ControlPlaneOptions};
use serde_json::{Value, json};
use tower::ServiceExt;

// ---------- a real session plane over an in-memory store -------------

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

fn session(id: &str) -> Session {
    serde_json::from_value(json!({
        "id": id,
        "title": "T",
        "workingDirectory": "/w",
        "backendType": "devin",
        "model": "m",
        "createdAt": 1_700_000_000.0,
        "lastActivityAt": 1_700_000_100.0,
        "mainChainId": 0
    }))
    .unwrap()
}

/// A node hosting `sessions` as `sepia-session` entities (plus any
/// extra planes) — the `LocalPlane` wiring `AppState::new` auto-
/// registers the session kind for.
fn mk_app(sessions: Vec<Session>, extra: Option<Arc<WidgetPlane>>) -> (Router, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let meta = MetaStore::open(&dir.path().join("meta.json"));
    let store = Arc::new(MemStore {
        sessions: Mutex::new(sessions.into_iter().map(|s| (s.id.clone(), s)).collect()),
    });
    let plane = ControlPlane::new(store, ControlPlaneOptions::default());
    let mut state = AppState::new(Arc::new(LocalPlane(plane, None)))
        .with_meta(meta)
        .with_held_watch(Duration::ZERO);
    if let Some(widget) = extra {
        state = state.with_kind(widget);
    }
    (app(state), dir)
}

// ---------- the fake kind ---------------------------------------------

/// `kind="test-widget"` — owns the `widget.*` op namespace (proving
/// ops route on the namespace, not the kind prefix), keeps an
/// in-memory entity map, and round-trips JSON snapshots.
#[derive(Default)]
struct WidgetPlane {
    docs: Mutex<BTreeMap<String, Value>>,
    counter: Mutex<u64>,
    seen_ops: Mutex<Vec<(String, Value)>>,
}

fn respond(v: Value, status: StatusCode) -> Response {
    (status, axum::Json(v)).into_response()
}

fn not_found() -> Response {
    respond(json!({ "error": "Not found" }), StatusCode::NOT_FOUND)
}

impl WidgetPlane {
    fn doc(&self, id: &str, index_doc: Value) -> Value {
        json!({
            "id": id,
            "kind": "test-widget",
            "indexDoc": index_doc,
            "updatedAt": "2026-01-01T00:00:00.000Z",
            "locked": false,
            "live": false,
        })
    }

    fn insert(&self, index_doc: Value) -> Value {
        let mut counter = self.counter.lock().unwrap();
        *counter += 1;
        let id = format!("w-{}", *counter);
        let doc = self.doc(&id, index_doc);
        self.docs.lock().unwrap().insert(id, doc.clone());
        doc
    }
}

#[async_trait::async_trait]
impl EntityPlane for WidgetPlane {
    fn kind(&self) -> &'static str {
        "test-widget"
    }

    /// Ops arrive as `widget.<verb>` — a namespace distinct from the
    /// kind tag on purpose.
    fn op_namespace(&self) -> &'static str {
        "widget"
    }

    async fn list(&self, _state: &AppState, _query: Option<&str>) -> Vec<Value> {
        self.docs.lock().unwrap().values().cloned().collect()
    }

    async fn get(&self, _state: &AppState, id: &str, _query: Option<&str>) -> Response {
        match self.docs.lock().unwrap().get(id) {
            Some(doc) => respond(doc.clone(), StatusCode::OK),
            None => not_found(),
        }
    }

    async fn create(&self, _state: &AppState, init: Value) -> Response {
        let doc = self.insert(init);
        respond(
            json!({ "id": doc["id"], "entity": doc }),
            StatusCode::CREATED,
        )
    }

    async fn delete(&self, _state: &AppState, id: &str, _query: Option<&str>) -> Response {
        self.docs.lock().unwrap().remove(id);
        respond(json!({ "ok": true }), StatusCode::OK)
    }

    async fn op(
        &self,
        _state: &AppState,
        id: &str,
        op: &str,
        payload: Value,
        _query: Option<&str>,
    ) -> Response {
        let verb = op.strip_prefix("widget.").unwrap_or(op);
        match verb {
            "poke" => {
                self.seen_ops
                    .lock()
                    .unwrap()
                    .push((format!("{id}:{op}"), payload.clone()));
                respond(json!({ "ok": true, "echoed": payload }), StatusCode::OK)
            }
            _ => respond(
                json!({ "error": format!("unknown op {op:?}") }),
                StatusCode::NOT_IMPLEMENTED,
            ),
        }
    }

    async fn export(&self, _state: &AppState, id: &str, _query: Option<&str>) -> Response {
        match self.docs.lock().unwrap().get(id) {
            Some(doc) => respond(json!({ "snapshot": doc }), StatusCode::OK),
            None => not_found(),
        }
    }

    async fn import(&self, _state: &AppState, body: Value) -> Response {
        let init = body.get("indexDoc").cloned().unwrap_or_else(|| json!({}));
        let doc = self.insert(init);
        respond(doc, StatusCode::CREATED)
    }
}

// ---------- request helpers -------------------------------------------

async fn call(app: &Router, method: Method, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
    let mut b = Request::builder().method(method).uri(uri);
    if body.is_some() {
        b = b.header("content-type", "application/json");
    }
    let req = b
        .body(body.map_or_else(Body::empty, |v| Body::from(v.to_string())))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = to_bytes(res.into_body(), 8 << 20).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

// ---------- tests ------------------------------------------------------

/// A registered non-sepia kind gets the full entity surface.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn registered_kind_serves_create_get_ops_snapshot_delete() {
    let widget = Arc::new(WidgetPlane::default());
    let (app, _dir) = mk_app(vec![session("s1")], Some(Arc::clone(&widget)));

    // Create through `kind` + `indexDoc`.
    let (status, body) = call(
        &app,
        Method::POST,
        "/api/entities",
        Some(json!({ "kind": "test-widget", "indexDoc": { "name": "w0" } })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    let id = body["id"].as_str().unwrap().to_string();

    // The list mixes kinds; `?kind=` filters server-side.
    let (status, body) = call(&app, Method::GET, "/api/entities", None).await;
    assert_eq!(status, StatusCode::OK);
    let entities = body["entities"].as_array().unwrap();
    assert!(
        entities
            .iter()
            .any(|e| e["id"] == json!(id) && e["kind"] == json!("test-widget")),
        "{entities:?}"
    );
    assert!(entities.iter().any(|e| e["kind"] == json!("sepia-session")));
    let (_, body) = call(&app, Method::GET, "/api/entities?kind=test-widget", None).await;
    let kinds: Vec<_> = body["entities"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["kind"].as_str().unwrap())
        .collect();
    assert_eq!(kinds, ["test-widget"]);
    let (_, body) = call(&app, Method::GET, "/api/entities?kind=ghost", None).await;
    assert_eq!(body["entities"].as_array().unwrap().len(), 0);

    // Owner resolution: the widget id skips the session plane's 404.
    let (status, body) = call(&app, Method::GET, &format!("/api/entities/{id}"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["indexDoc"]["name"], json!("w0"));

    // `widget.*` ops dispatch into the plane's own verb table.
    let (status, body) = call(
        &app,
        Method::POST,
        &format!("/api/entities/{id}/ops"),
        Some(json!({ "kind": "widget.poke", "payload": { "x": 1 } })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["echoed"], json!({ "x": 1 }));
    assert_eq!(
        widget.seen_ops.lock().unwrap().as_slice(),
        [(format!("{id}:widget.poke"), json!({ "x": 1 }))].as_slice()
    );

    // Snapshot export → import round-trips a fresh entity.
    let (status, body) = call(
        &app,
        Method::GET,
        &format!("/api/entities/{id}/snapshot"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let snap = body["snapshot"].clone();
    assert_eq!(snap["indexDoc"]["name"], json!("w0"));
    let (status, body) = call(
        &app,
        Method::POST,
        "/api/entities/snapshot",
        Some(json!({ "kind": "test-widget", "indexDoc": snap["indexDoc"] })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    let imported = body["id"].as_str().unwrap().to_string();
    assert_ne!(imported, id);

    // `entity.delete` resolves the widget plane and removes it.
    let (status, body) = call(
        &app,
        Method::POST,
        &format!("/api/entities/{id}/ops"),
        Some(json!({ "kind": "entity.delete" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body, json!({ "ok": true }));
    let (status, _) = call(&app, Method::GET, &format!("/api/entities/{id}"), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

/// The built-in kind keeps answering exactly as before the registry.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sepia_session_path_is_unchanged() {
    let widget = Arc::new(WidgetPlane::default());
    let (app, _dir) = mk_app(vec![session("s1")], Some(Arc::clone(&widget)));

    // List projects sessions into entity docs, widget rows alongside.
    let (status, body) = call(&app, Method::GET, "/api/entities", None).await;
    assert_eq!(status, StatusCode::OK);
    let entities = body["entities"].as_array().unwrap();
    let s1 = entities.iter().find(|e| e["id"] == json!("s1")).unwrap();
    assert_eq!(s1["kind"], json!("sepia-session"));
    assert_eq!(s1["indexDoc"]["title"], json!("T"));

    // Item GET resolves the session plane for `s1`.
    let (status, body) = call(&app, Method::GET, "/api/entities/s1", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["kind"], json!("sepia-session"));

    // `sepia.*` ops still land on the session verb handlers —
    // `sepia.meta` is the meta PATCH (needs a real body field).
    let (status, body) = call(
        &app,
        Method::POST,
        "/api/entities/s1/ops",
        Some(json!({ "kind": "sepia.meta", "payload": { "title": "Renamed" } })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body, json!({ "ok": true }));

    // A `sepia.*` verb that doesn't exist 501s from the plane's own
    // verb table — not from the registry.
    let (status, body) = call(
        &app,
        Method::POST,
        "/api/entities/s1/ops",
        Some(json!({ "kind": "sepia.frobnicate" })),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_IMPLEMENTED);
    assert_eq!(body["error"], json!("unknown op \"sepia.frobnicate\""));

    // Snapshot export is the session export.
    let (status, body) = call(&app, Method::GET, "/api/entities/s1/snapshot", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["session"]["id"], json!("s1"));

    // `entity.delete` still drives the session DELETE — the plane has
    // no agent runtimes here, so `session/delete` fails with the
    // plane's own `unknown_agent` 400 (proof the op dispatched into
    // the session path, not that deletion succeeded).
    let (status, body) = call(
        &app,
        Method::POST,
        "/api/entities/s1/ops",
        Some(json!({ "kind": "entity.delete" })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(body["code"], json!("unknown_agent"));
    // Same answer through `DELETE /api/entities/{id}`.
    let (del_status, _) = call(&app, Method::DELETE, "/api/entities/s1", None).await;
    assert_eq!(del_status, status);
}

/// Unknown kinds/ops keep the 501 + `not registered`/`unknown op`
/// surface the hub maps back to 501.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unknown_kinds_and_ops_501() {
    let (app, _dir) = mk_app(vec![], Some(Arc::new(WidgetPlane::default())));

    let (status, body) = call(
        &app,
        Method::POST,
        "/api/entities",
        Some(json!({ "kind": "ghost", "indexDoc": {} })),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_IMPLEMENTED);
    assert_eq!(body["error"], json!("entity kind \"ghost\" not registered"));

    let (status, body) = call(
        &app,
        Method::POST,
        "/api/entities/snapshot",
        Some(json!({ "kind": "ghost" })),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_IMPLEMENTED);
    assert_eq!(body["error"], json!("entity kind \"ghost\" not registered"));

    for kind in ["ghost.verb", "entity.frobnicate", "widget.nope"] {
        let (status, body) = call(
            &app,
            Method::POST,
            "/api/entities/w-1/ops",
            Some(json!({ "kind": kind })),
        )
        .await;
        assert_eq!(status, StatusCode::NOT_IMPLEMENTED, "{kind}: {body}");
        assert_eq!(body["error"], json!(format!("unknown op {kind:?}")));
    }
}

/// A `kind`-less create/import targets the default (first-registered)
/// plane — `sepia-session` here, so the body is the session create
/// payload verbatim (`cwd` required), same as before.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn kindless_requests_hit_the_default_plane() {
    let (app, _dir) = mk_app(vec![], Some(Arc::new(WidgetPlane::default())));

    // No `kind` → session create → the sessions route's own 400.
    let (status, body) = call(
        &app,
        Method::POST,
        "/api/entities",
        Some(json!({ "indexDoc": { "title": "x" } })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(body["error"], json!("cwd is required"));

    // `entity.delete` of a missing entity stays idempotent — 200, no
    // dead-letter on replay.
    let (status, body) = call(
        &app,
        Method::POST,
        "/api/entities/ghost/ops",
        Some(json!({ "kind": "entity.delete" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, json!({ "ok": true }));
}
