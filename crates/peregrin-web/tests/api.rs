//! `NodeApi` + `#[server]` coverage — every hub server fn driven
//! in-process against `FakeNodeApi` (happy paths, validation guards,
//! upstream `Err` → `ServerFnError` mapping, missing-context), plus
//! `HttpNodeApi` exercised end-to-end against a canned HTTP/1.1 stub
//! server so request shaping and response parsing both run for real.

#![cfg(feature = "ssr")]
#![allow(clippy::unwrap_used, clippy::pedantic)]

use std::future::Future;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::Arc;

use leptos::prelude::*;
use peregrin_web::api::{self, HttpNodeApi, NodeApi, RewindSelector};
use peregrin_web::dto::{PushKeysDto, PushSubscriptionDto};
use peregrin_web::testing::FakeNodeApi;
use serde_json::json;

// ---------- leptos context harness -----------------------------------

/// Run `fut` with `api` provided as `Arc<dyn NodeApi>` in leptos
/// context — the same shape `peregrin-hub` gives request handlers.
/// `ScopedFuture` re-sets the owner on every poll, which is what the
/// server fns' `use_context` calls need.
async fn scoped<T, Fut>(api: Option<Arc<dyn NodeApi>>, fut: Fut) -> T
where
    Fut: Future<Output = T> + 'static,
    T: 'static,
{
    static LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
    let _g = LOCK.lock().await;
    let _ = any_spawner::Executor::init_tokio();
    let owner = Owner::new_root(None);
    let fut = owner.with(|| {
        if let Some(api) = api {
            provide_context::<Arc<dyn NodeApi>>(api);
        }
        leptos::reactive::computed::ScopedFuture::new_untracked(fut)
    });
    let out = tokio::task::LocalSet::new().run_until(fut).await;
    drop(owner);
    out
}

fn api() -> Option<Arc<dyn NodeApi>> {
    Some(FakeNodeApi::seeded().shared())
}

// ---------- server fns: happy paths ---------------------------------

#[tokio::test]
async fn server_fns_happy_paths() {
    let a = api();
    scoped(a.clone(), async {
        assert_eq!(api::list_sessions().await.unwrap().len(), 6);
        assert_eq!(api::list_agents().await.unwrap().len(), 2);
        assert_eq!(api::list_projects().await.unwrap().len(), 2);
        assert_eq!(api::node_info().await.unwrap().id, "n1");
        assert!(api::node_status().await.unwrap()[0].status == "up");
        assert!(api::pending_writes().await.unwrap().is_empty());
        assert_eq!(api::get_config().await.unwrap().len(), 2);
        assert!(api::push_vapid_key().await.unwrap().starts_with("BJx"));

        let h = api::session_history("s1".into(), None, Some(4), Some(2))
            .await
            .unwrap();
        assert_eq!(h.total, 8);
        let s = api::get_session("s1".into(), Some("devin".into()))
            .await
            .unwrap();
        assert_eq!(s.id, "s1");

        api::send_prompt("s1".into(), None, "hi".into(), Vec::new())
            .await
            .unwrap();
        api::send_prompt(
            "s1".into(),
            None,
            String::new(),
            vec![json!({"type":"image"})],
        )
        .await
        .unwrap();
        api::cancel_run("s1".into(), None).await.unwrap();
        let c = api::create_session(
            " /work ".into(),
            Some(" devin ".into()),
            Some(" t ".into()),
            Some(" m ".into()),
            Some(" ".into()),
        )
        .await
        .unwrap();
        assert_eq!(c.id, "s-new");
        let at = api::attach_session("s1".into(), None, true).await.unwrap();
        assert!(at.attached);
        api::detach_session("s1".into(), None).await.unwrap();
        api::answer_permission("s1".into(), None, "r1".into(), Some("o1".into()))
            .await
            .unwrap();
        api::answer_permission("s1".into(), None, "r1".into(), None)
            .await
            .unwrap();
        api::rename_session("s1".into(), None, " new name ".into())
            .await
            .unwrap();
        api::set_session_projects("s1".into(), None, vec!["p1".into()])
            .await
            .unwrap();
        api::pin_session("s1".into(), None, true).await.unwrap();
        api::set_session_model("s1".into(), None, Some("m1".into()))
            .await
            .unwrap();
        api::set_session_model("s1".into(), None, None)
            .await
            .unwrap();
        api::archive_session("s1".into(), None, true).await.unwrap();
        api::delete_session("s1".into(), None).await.unwrap();
        assert_eq!(
            api::list_checkpoints("s1".into(), None)
                .await
                .unwrap()
                .len(),
            2
        );
        api::restore_checkpoint("s1".into(), None, "cp1".into())
            .await
            .unwrap();
        let r = api::rewind_session("s1".into(), None, RewindSelector::NodeId(3))
            .await
            .unwrap();
        assert_eq!(r.kept, 0);
        let conv = api::convert_session("s1".into(), None, "cline".into())
            .await
            .unwrap();
        assert_eq!(conv, "s-converted");
        let imp = api::import_session(json!({"agent":"devin","session":{"nodes":[]}}))
            .await
            .unwrap();
        assert_eq!(imp.id, "s-imported");
        let t = api::transfer_session("s1".into(), None, "tower".into(), Some("/w".into()))
            .await
            .unwrap();
        assert_eq!(t.new_session_id, "s-transferred");
        let pt = api::transfer_project("/work/acme-api".into(), "a".into(), "b".into(), None)
            .await
            .unwrap();
        assert_eq!(pt.target_node, "b");
        let p = api::create_project("cap".into(), None).await.unwrap();
        assert_eq!(p.id, "p-new");
        api::delete_project("p1".into(), None).await.unwrap();
        api::set_config("k".into(), json!(1)).await.unwrap();
        assert_eq!(api::fs_dirs("/".into(), None).await.unwrap().len(), 2);
        assert_eq!(
            api::redeem_pair_code("code".into(), None).await.unwrap(),
            "sepia-pair-fake-token-0123456789"
        );
        let n = api::rename_node("bench".into(), None).await.unwrap();
        assert_eq!(n.name, "bench");
        api::push_subscribe(PushSubscriptionDto {
            endpoint: "https://push/x".into(),
            keys: PushKeysDto::default(),
            prefs: None,
        })
        .await
        .unwrap();
        api::push_unsubscribe("https://push/x".into())
            .await
            .unwrap();
    })
    .await;
}

#[tokio::test]
async fn server_fns_validation_guards() {
    let a = api();
    scoped(a.clone(), async {
        assert!(
            api::session_history(String::new(), None, None, None)
                .await
                .is_err()
        );
        assert!(api::get_session(String::new(), None).await.is_err());
        assert!(
            api::send_prompt(String::new(), None, "x".into(), vec![])
                .await
                .is_err()
        );
        assert!(
            api::send_prompt("s1".into(), None, "  ".into(), vec![])
                .await
                .is_err()
        );
        assert!(
            api::create_session("  ".into(), None, None, None, None)
                .await
                .is_err()
        );
        assert!(
            api::attach_session(String::new(), None, false)
                .await
                .is_err()
        );
        assert!(api::detach_session(String::new(), None).await.is_err());
        assert!(
            api::answer_permission(String::new(), None, "r".into(), None)
                .await
                .is_err()
        );
        assert!(
            api::answer_permission("s1".into(), None, String::new(), None)
                .await
                .is_err()
        );
        assert!(
            api::rename_session(String::new(), None, "t".into())
                .await
                .is_err()
        );
        assert!(
            api::rename_session("s1".into(), None, " ".into())
                .await
                .is_err()
        );
        assert!(
            api::rename_session("s1".into(), None, "x".repeat(201))
                .await
                .is_err()
        );
        assert!(
            api::set_session_projects(String::new(), None, vec![])
                .await
                .is_err()
        );
        assert!(api::pin_session(String::new(), None, true).await.is_err());
        assert!(
            api::set_session_model(String::new(), None, None)
                .await
                .is_err()
        );
        assert!(
            api::set_session_model("s1".into(), None, Some("m".repeat(101)))
                .await
                .is_err()
        );
        assert!(
            api::archive_session(String::new(), None, true)
                .await
                .is_err()
        );
        assert!(api::delete_session(String::new(), None).await.is_err());
        assert!(api::list_checkpoints(String::new(), None).await.is_err());
        assert!(
            api::restore_checkpoint(String::new(), None, "c".into())
                .await
                .is_err()
        );
        assert!(
            api::restore_checkpoint("s1".into(), None, String::new())
                .await
                .is_err()
        );
        assert!(
            api::rewind_session(String::new(), None, RewindSelector::NodeId(1))
                .await
                .is_err()
        );
        assert!(
            api::convert_session(String::new(), None, "cline".into())
                .await
                .is_err()
        );
        assert!(
            api::convert_session("s1".into(), None, "bogus".into())
                .await
                .is_err()
        );
        assert!(api::import_session(json!(null)).await.is_err());
        assert!(
            api::transfer_session(String::new(), None, "n".into(), None)
                .await
                .is_err()
        );
        assert!(
            api::transfer_session("s1".into(), None, String::new(), None)
                .await
                .is_err()
        );
        assert!(
            api::transfer_project(String::new(), "a".into(), "b".into(), None)
                .await
                .is_err()
        );
        assert!(
            api::transfer_project("c".into(), String::new(), "b".into(), None)
                .await
                .is_err()
        );
        assert!(
            api::transfer_project("c".into(), "a".into(), String::new(), None)
                .await
                .is_err()
        );
        assert!(api::create_project(" ".into(), None).await.is_err());
        assert!(api::delete_project(String::new(), None).await.is_err());
        assert!(api::set_config(String::new(), json!(1)).await.is_err());
        assert!(api::fs_dirs(String::new(), None).await.is_err());
        assert!(api::redeem_pair_code(" ".into(), None).await.is_err());
        assert!(api::rename_node(" ".repeat(101), None).await.is_err());
        assert!(
            api::push_subscribe(PushSubscriptionDto::default())
                .await
                .is_err()
        );
        assert!(api::push_unsubscribe(String::new()).await.is_err());
    })
    .await;
}

#[tokio::test]
async fn server_fns_upstream_and_context_errors() {
    // Upstream `Err` → `ServerFnError` via `map_err` on read endpoints.
    let failing = Some(FakeNodeApi::seeded().with_error("node is down").shared());
    scoped(failing, async {
        assert!(api::list_sessions().await.is_err());
        assert!(api::list_agents().await.is_err());
        assert!(api::list_projects().await.is_err());
        assert!(api::node_info().await.is_err());
        assert!(api::node_status().await.is_err());
        assert!(api::pending_writes().await.is_err());
        assert!(api::get_config().await.is_err());
        assert!(
            api::session_history("s1".into(), None, None, None)
                .await
                .is_err()
        );
        assert!(api::list_checkpoints("s1".into(), None).await.is_err());
        assert!(api::fs_dirs("/".into(), None).await.is_err());
        assert!(api::redeem_pair_code("c".into(), None).await.is_err());
        assert!(api::push_vapid_key().await.is_err());
        assert!(api::get_session("s1".into(), None).await.is_err());
    })
    .await;
    // `get_session` on a healthy api misses by id → also `Err`.
    scoped(api(), async {
        assert!(api::get_session("nope".into(), None).await.is_err());
    })
    .await;
    // No `Arc<dyn NodeApi>` in context → every fn fails at `node_api()?`.
    scoped(None, async {
        assert!(api::list_sessions().await.is_err());
        assert!(
            api::send_prompt("s1".into(), None, "x".into(), vec![])
                .await
                .is_err()
        );
        assert!(api::node_info().await.is_err());
    })
    .await;
}

#[tokio::test]
async fn query_scopes_delegate() {
    let a = api();
    scoped(a.clone(), async {
        assert_eq!(api::sessions_scope(()).await.unwrap().len(), 6);
        assert!(api::pending_scope(()).await.unwrap().is_empty());
        assert_eq!(api::agents_scope(()).await.unwrap().len(), 2);
        assert_eq!(api::projects_scope(()).await.unwrap().len(), 2);
        assert_eq!(api::node_info_scope(()).await.unwrap().id, "n1");
        assert!(!api::node_status_scope(()).await.unwrap().is_empty());
        assert_eq!(
            api::session_scope(("s1".into(), None)).await.unwrap().id,
            "s1"
        );
        // Closed/empty key short-circuits before the api call.
        assert!(
            api::checkpoints_scope((false, "s1".into(), None))
                .await
                .unwrap()
                .is_empty()
        );
        assert!(
            api::checkpoints_scope((true, String::new(), None))
                .await
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            api::checkpoints_scope((true, "s1".into(), None))
                .await
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            api::history_scope(("s1".into(), None)).await.unwrap().total,
            8
        );
    })
    .await;
}

// ---------- HttpNodeApi vs canned server ----------------------------

/// Minimal HTTP/1.1 stub: matches `(METHOD SP PATH)` exactly, else
/// falls back to a default; request bodies are drained via
/// Content-Length so `delete_json`'s payload doesn't confuse the next
/// accept.
struct Stub {
    addr: String,
    requests: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
    _t: std::thread::JoinHandle<()>,
}

impl Stub {
    fn serve(routes: Vec<(&'static str, &'static str, u16, &'static str)>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(std::sync::Mutex::new(Vec::new()));
        let reqs = requests.clone();
        let routes: Vec<(String, String, u16, String)> = routes
            .into_iter()
            .map(|(m, p, s, b)| (m.to_string(), p.to_string(), s, b.to_string()))
            .collect();
        let _t = std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                serve_conn(&mut stream, &reqs, &routes);
            }
        });
        Self { addr, requests, _t }
    }

    fn client(&self) -> HttpNodeApi {
        HttpNodeApi::new(&self.addr, Some("tok-123".to_string()))
    }
}

fn find_subslice(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

fn content_length(head: &str) -> usize {
    head.lines()
        .find_map(|l| {
            l.to_ascii_lowercase()
                .strip_prefix("content-length:")
                .and_then(|v| v.trim().parse().ok())
        })
        .unwrap_or(0)
}

fn serve_conn(
    stream: &mut std::net::TcpStream,
    reqs: &std::sync::Mutex<Vec<String>>,
    routes: &[(String, String, u16, String)],
) {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 8192];
    let mut head_end = None;
    loop {
        match stream.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                buf.extend_from_slice(&chunk[..n]);
                if head_end.is_none() {
                    head_end = find_subslice(&buf, b"\r\n\r\n");
                }
                let Some(he) = head_end else { continue };
                let head = String::from_utf8_lossy(&buf[..he]).to_string();
                if buf.len() < he + 4 + content_length(&head) {
                    continue;
                }
                let line = head.lines().next().unwrap_or("").to_string();
                reqs.lock().unwrap().push(line.clone());
                let (status, body) = routes
                    .iter()
                    .find(|(m, p, _, _)| line.starts_with(&format!("{m} {p}")))
                    .map(|(_, _, s, b)| (*s, b.clone()))
                    .unwrap_or((404, "{}".to_string()));
                let reason = if status == 200 { "OK" } else { "ERR" };
                let res = format!(
                    "HTTP/1.1 {status} {reason}\r\ncontent-type: application/json\r\n\
                     content-length: {}\r\nconnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(res.as_bytes());
                break;
            }
        }
    }
}

fn stub_routes() -> Stub {
    // Order matters — matching is `starts_with`, so specific paths
    // precede the generic collection routes.
    Stub::serve(vec![
        (
            "GET",
            "/api/sessions/s1/history",
            200,
            r#"{"messages":[],"total":0,"start":0}"#,
        ),
        ("POST", "/api/sessions/s1/prompt", 200, "{}"),
        ("POST", "/api/sessions/s1/cancel", 200, "{}"),
        (
            "POST",
            "/api/sessions/s1/attach",
            200,
            r#"{"attached":true,"readOnly":false,"agentId":"d"}"#,
        ),
        ("POST", "/api/sessions/s1/detach", 200, "{}"),
        ("POST", "/api/sessions/s1/permission", 200, "{}"),
        (
            "GET",
            "/api/sessions/s1/checkpoints",
            200,
            r#"{"checkpoints":[{"ref":"cp1"}]}"#,
        ),
        ("POST", "/api/sessions/s1/restore", 200, "{}"),
        (
            "POST",
            "/api/sessions/s1/rewind",
            200,
            r#"{"kept":1,"removed":2}"#,
        ),
        (
            "POST",
            "/api/sessions/s1/convert",
            200,
            r#"{"sessionId":"s2"}"#,
        ),
        (
            "POST",
            "/api/sessions/import",
            200,
            r#"{"id":"s9","title":"t","agent":"d"}"#,
        ),
        ("PATCH", "/api/sessions/s1", 200, "{}"),
        ("DELETE", "/api/sessions/s1", 200, "{}"),
        ("GET", "/api/sessions/s1", 200, r#"{"id":"s1"}"#),
        ("GET", "/api/sessions", 200, r#"{"sessions":[{"id":"s1"}]}"#),
        (
            "POST",
            "/api/sessions",
            200,
            r#"{"id":"s9","agentId":"devin"}"#,
        ),
        (
            "GET",
            "/api/agents",
            200,
            r#"{"agents":[{"id":"d","label":"D"}]}"#,
        ),
        (
            "GET",
            "/api/projects",
            200,
            r#"{"projects":[{"id":"p1","name":"P"}]}"#,
        ),
        (
            "POST",
            "/api/projects",
            200,
            r#"{"project":{"id":"p2","name":"N"}}"#,
        ),
        ("DELETE", "/api/projects/p1", 200, "{}"),
        ("GET", "/api/config", 200, r#"{"config":{"k":1}}"#),
        ("PATCH", "/api/config/k", 200, "{}"),
        (
            "PATCH",
            "/api/node",
            200,
            r#"{"id":"n1","name":"bench","version":"1","protocol":1}"#,
        ),
        (
            "GET",
            "/api/node",
            200,
            r#"{"id":"n1","name":"bench","version":"1","protocol":1}"#,
        ),
        ("GET", "/api/fs", 200, r#"{"dirs":["/a"]}"#),
        ("POST", "/api/pair", 200, r#"{"token":"tok"}"#),
        ("GET", "/api/push/vapid", 200, r#"{"publicKey":"pk"}"#),
        ("POST", "/api/push/subscribe", 200, "{}"),
        ("DELETE", "/api/push/subscribe", 200, "{}"),
    ])
}

#[tokio::test]
async fn http_api_happy_paths() {
    let stub = stub_routes();
    let c = stub.client();
    let agent = Some("devin");

    assert_eq!(c.list_sessions().await.unwrap()[0].id, "s1");
    let h = c.history("s1", agent, Some(4), Some(2)).await.unwrap();
    assert_eq!(h.total, 0);
    // No-query history path too (id_query empty, no params).
    let _ = c.history("s1", None, None, None).await.unwrap();
    assert_eq!(c.get_session("s1", agent).await.unwrap().id, "s1");
    assert_eq!(c.get_session("s1", None).await.unwrap().id, "s1");
    c.prompt("s1", agent, "hi", &[]).await.unwrap();
    c.cancel("s1", agent).await.unwrap();
    let created = c
        .create_session("/w", agent, Some("t"), Some("m"), None)
        .await
        .unwrap();
    assert_eq!(created.id, "s9");
    // Optional body fields skipped.
    let _ = c
        .create_session("/w", None, None, None, None)
        .await
        .unwrap();
    let at = c.attach("s1", agent, true).await.unwrap();
    assert!(at.attached);
    c.detach("s1", agent).await.unwrap();
    c.answer_permission("s1", agent, "r1", Some("o1"))
        .await
        .unwrap();
    c.patch_meta("s1", agent, &json!({"title":"t"}))
        .await
        .unwrap();
    c.delete_session("s1", agent).await.unwrap();
    assert_eq!(c.checkpoints("s1", agent).await.unwrap()[0].r#ref, "cp1");
    c.restore("s1", agent, "cp1").await.unwrap();
    let rw = c
        .rewind("s1", agent, &RewindSelector::NodeId(3))
        .await
        .unwrap();
    assert_eq!(rw.removed, 2);
    let rw = c
        .rewind("s1", agent, &RewindSelector::Checkpoint("cp1".into()))
        .await
        .unwrap();
    assert_eq!(rw.kept, 1);
    assert_eq!(c.convert("s1", agent, "cline").await.unwrap(), "s2");
    let imp = c.import(&json!({"x":1})).await.unwrap();
    assert_eq!(imp.id, "s9");
    // Direct-connection stubs — always the same Err, no I/O.
    assert!(c.transfer_session("s", None, "n", None).await.is_err());
    assert!(c.transfer_project("c", "a", "b", None).await.is_err());
    assert_eq!(c.list_agents().await.unwrap()[0].id, "d");
    assert_eq!(c.list_projects().await.unwrap()[0].id, "p1");
    assert_eq!(c.create_project("n", None).await.unwrap().id, "p2");
    c.delete_project("p1", None).await.unwrap();
    assert!(c.get_config().await.unwrap().contains_key("k"));
    c.set_config("k", &json!(2)).await.unwrap();
    assert_eq!(c.rename_node("x", None).await.unwrap().name, "bench");
    assert_eq!(c.fs_dirs("/a/b", None).await.unwrap()[0], "/a");
    assert_eq!(c.pair("code", None).await.unwrap(), "tok");
    assert_eq!(c.node_info().await.unwrap().id, "n1");
    let st = c.node_status().await.unwrap();
    assert_eq!(st[0].status, "up");
    assert_eq!(st[0].label, "bench");
    assert!(c.pending_writes().await.unwrap().is_empty());
    assert_eq!(c.push_vapid().await.unwrap(), "pk");
    c.push_subscribe(&PushSubscriptionDto {
        endpoint: "e".into(),
        keys: PushKeysDto::default(),
        prefs: None,
    })
    .await
    .unwrap();
    c.push_unsubscribe("e").await.unwrap();
}

#[tokio::test]
async fn http_api_error_paths() {
    // Every route 500s with an `{error}` payload → `error_of` message.
    let err_stub = Stub::serve(vec![("GET", "/api", 500, r#"{"error":"node exploded"}"#)]);
    let c = err_stub.client();
    let e = c.list_sessions().await.unwrap_err();
    assert_eq!(e, "node exploded");
    // `node_status` maps upstream failure to a `down` row instead.
    let st = c.node_status().await.unwrap();
    assert_eq!(st[0].status, "down");
    assert_eq!(st[0].label, err_stub.addr);

    // 500 without an `{error}` payload → the `what` fallback message.
    let bare = Stub::serve(vec![("POST", "/api", 500, "{}")]);
    let c = bare.client();
    let e = c.prompt("s1", None, "x", &[]).await.unwrap_err();
    assert!(e.contains("prompt"));
    assert!(e.contains("500"));
    let e = c.pair("c", None).await.unwrap_err();
    assert!(e.contains("pairing"));

    // 200 with an empty token → the pair client's own guard.
    let notok = Stub::serve(vec![("POST", "/api/pair", 200, r#"{"token":""}"#)]);
    let e = notok.client().pair("c", None).await.unwrap_err();
    assert!(e.contains("no token"));

    // 200 convert missing `sessionId` → the field guard.
    let noc = Stub::serve(vec![("POST", "/api", 200, "{}")]);
    let e = noc.client().convert("s", None, "cline").await.unwrap_err();
    assert!(e.contains("sessionId"));

    // 200 with a body that isn't JSON → `read_json` decode error.
    let bad = Stub::serve(vec![("GET", "/api", 200, "not json")]);
    assert!(bad.client().list_agents().await.is_err());

    // Connection refused → transport error (`get` → `e.to_string()`).
    let dead = HttpNodeApi::new("http://127.0.0.1:1", None);
    assert!(dead.list_sessions().await.is_err());
    // And `node_status` still degrades to the `down` row.
    assert_eq!(dead.node_status().await.unwrap()[0].status, "down");

    // A `down` row reports the base url for url+label.
    let st = dead.node_status().await.unwrap();
    assert_eq!(st[0].url, "http://127.0.0.1:1");
}

#[tokio::test]
async fn http_api_method_bodies() {
    // The request was actually issued — check method+path shapes the
    // stub recorded (agent query + url encoding included).
    let stub = stub_routes();
    let c = stub.client();
    c.get_session("s1", Some("a gent")).await.unwrap();
    c.fs_dirs("/a b", None).await.unwrap();
    c.push_unsubscribe("e").await.unwrap();
    let reqs = stub.requests.lock().unwrap().clone();
    assert!(
        reqs.iter()
            .any(|r| r == "GET /api/sessions/s1?agent=a%20gent HTTP/1.1")
    );
    assert!(
        reqs.iter()
            .any(|r| r == "GET /api/fs?path=%2Fa%20b HTTP/1.1")
    );
    assert!(
        reqs.iter()
            .any(|r| r.starts_with("DELETE /api/push/subscribe"))
    );
    // Trailing-slash bases are trimmed.
    let c2 = HttpNodeApi::new(format!("{}/", stub.addr), None);
    c2.list_sessions().await.unwrap();
}
