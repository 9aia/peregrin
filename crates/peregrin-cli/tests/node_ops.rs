//! Drive every `api::*` request helper and `node_ops::*` verb against a
//! canned HTTP/1.1 node — the same paths the `peregrin` binary hits,
//! plus `{error, code}` decoding, transport failures and the SSE
//! reader.

#![allow(clippy::unwrap_used, clippy::pedantic)]

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

use peregrin_cli::node_ops::{self, NodeArgs, TargetArgs};
use peregrin_cli::{CliError, api};
use serde_json::{Value, json};

// ---------- canned node ------------------------------------------------

/// Minimal HTTP/1.1 responder. Routes match `(METHOD, path-prefix)` in
/// order; the default is 404 `{}`. Requests are recorded for
/// assertions. Bodies are drained via Content-Length.
struct Stub {
    base: String,
    requests: Arc<Mutex<Vec<String>>>,
    _t: std::thread::JoinHandle<()>,
}

impl Stub {
    fn serve(routes: Vec<(&'static str, &'static str, u16, &'static str)>) -> Self {
        Self::serve_raw(
            routes
                .into_iter()
                .map(|(m, p, s, b)| (m.to_string(), p.to_string(), s, b.to_string()))
                .collect(),
        )
    }

    fn serve_raw(routes: Vec<(String, String, u16, String)>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let reqs = requests.clone();
        let _t = std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                serve_conn(&mut stream, &reqs, &routes);
            }
        });
        Self { base, requests, _t }
    }

    fn node_args(&self, json: bool) -> NodeArgs {
        NodeArgs {
            node: self.base.clone(),
            token: Some("tok".into()),
            json,
        }
    }

    fn target_args(&self) -> TargetArgs {
        TargetArgs {
            node: self.base.clone(),
            token: Some("tok".into()),
        }
    }

    fn target(&self) -> api::NodeTarget {
        api::resolve_target(&self.base, Some("tok"))
    }

    fn lines(&self) -> Vec<String> {
        self.requests.lock().unwrap().clone()
    }
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

fn serve_conn(
    stream: &mut std::net::TcpStream,
    reqs: &Mutex<Vec<String>>,
    routes: &[(String, String, u16, String)],
) {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 16 * 1024];
    let mut head_end = None;
    loop {
        match stream.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                buf.extend_from_slice(&chunk[..n]);
                if head_end.is_none() {
                    head_end = find(&buf, b"\r\n\r\n");
                }
                let Some(he) = head_end else { continue };
                let head = String::from_utf8_lossy(&buf[..he]).to_string();
                let len = head
                    .lines()
                    .find_map(|l| {
                        l.to_ascii_lowercase()
                            .strip_prefix("content-length:")
                            .and_then(|v| v.trim().parse().ok())
                    })
                    .unwrap_or(0);
                if buf.len() < he + 4 + len {
                    continue;
                }
                let line = head.lines().next().unwrap_or("").to_string();
                reqs.lock().unwrap().push(line.clone());
                let (status, body) = routes
                    .iter()
                    .find(|(m, p, _, _)| line.starts_with(&format!("{m} {p}")))
                    .map(|(_, _, s, b)| (*s, b.clone()))
                    .unwrap_or((404, "{}".into()));
                let res = format!(
                    "HTTP/1.1 {status} X\r\ncontent-type: application/json\r\n\
                     content-length: {}\r\nconnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(res.as_bytes());
                break;
            }
        }
    }
}

fn node_stub() -> Stub {
    // Specific routes before generic prefixes.
    Stub::serve(vec![
        ("GET", "/api/health", 200, r#"{"ok":true}"#),
        (
            "GET",
            "/api/node",
            200,
            r#"{"id":"n1","name":"bench","version":"1.0","protocol":2,
                "agents":["devin"],"capabilities":["attach"]}"#,
        ),
        (
            "GET",
            "/api/user",
            200,
            r#"{"user":{"username":"lu","hostname":"box","homedir":"/home/lu",
                "shell":"/bin/bash","platform":"linux","arch":"x86_64"}}"#,
        ),
        (
            "GET",
            "/api/agents",
            200,
            r#"{"agents":[{"id":"devin","label":"Devin","capabilities":{}},
                           {"id":"cline","label":"Cline"}]}"#,
        ),
        ("GET", "/api/fs", 200, r#"{"dirs":["/a","/b"]}"#),
        ("POST", "/api/pair", 200, r#"{"token":"secret-tok"}"#),
        (
            "GET",
            "/api/sessions/s1/history?limit=2&before=9",
            200,
            r#"{"messages":[],"total":3,"start":0}"#,
        ),
        (
            "GET",
            "/api/sessions/s1/history",
            200,
            r#"{"messages":[{"role":"user","content":"hi","createdAt":1791441590000.0},
                            {"role":"assistant","content":"yo","toolName":"exec","createdAt":1791441591000.0}],
                "total":2,"start":5}"#,
        ),
        (
            "GET",
            "/api/sessions/s1/checkpoints",
            200,
            r#"{"checkpoints":[{"ref":"cp1","createdAt":1791441590000.0,"kind":"auto","runCount":3},
                              {"ref":"cp2"}]}"#,
        ),
        (
            "GET",
            "/api/sessions/s1/export",
            200,
            r#"{"session":{"id":"s1","nodes":[]}}"#,
        ),
        (
            "GET",
            "/api/sessions/s1/stream",
            200,
            "event: run\ndata: {\"k\":1}\n\ndata: lone\n\n: ping\n\n",
        ),
        ("POST", "/api/sessions/s1/prompt", 200, "{}"),
        ("POST", "/api/sessions/s1/cancel", 200, "{}"),
        (
            "POST",
            "/api/sessions/s1/attach",
            200,
            r#"{"attached":true,"readOnly":false,"agentId":"devin"}"#,
        ),
        ("POST", "/api/sessions/s1/permission", 200, "{}"),
        ("PATCH", "/api/sessions/s1", 200, "{}"),
        ("DELETE", "/api/sessions/s1", 200, "{}"),
        (
            "POST",
            "/api/sessions/s1/convert",
            200,
            r#"{"sessionId":"s2"}"#,
        ),
        (
            "POST",
            "/api/sessions/s1/restore",
            200,
            r#"{"restored":[{"path":"a.rs","action":"rewrote"}],
                "skipped":[{"path":"b.rs","reason":"conflict"}]}"#,
        ),
        (
            "POST",
            "/api/sessions/s1/rewind",
            200,
            r#"{"kept":4,"removed":2}"#,
        ),
        (
            "POST",
            "/api/sessions/import",
            200,
            r#"{"id":"s9","title":"imp","cwd":"/w","agent":"devin","updatedAt":"now"}"#,
        ),
        (
            "GET",
            "/api/sessions",
            200,
            r#"{"sessions":[{"id":"s1","title":"t","cwd":"/w","agent":"devin","updatedAt":"u",
                            "locked":true,"busy":true,"pinned":true,"archived":true},
                           {"id":"s2","title":"t2","cwd":"/x","agent":"cline","updatedAt":"u"}]}"#,
        ),
        (
            "POST",
            "/api/sessions",
            200,
            r#"{"id":"s9","agentId":"devin"}"#,
        ),
        ("GET", "/api/events", 200, "event: tick\ndata: {}\n\n"),
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
        ("PATCH", "/api/projects/p1", 200, "{}"),
        ("DELETE", "/api/projects/p1", 200, "{}"),
        ("GET", "/api/config", 200, r#"{"config":{"k":1,"s":"v"}}"#),
        ("PATCH", "/api/config/k", 200, "{}"),
        ("GET", "/api/push/vapid", 200, r#"{"publicKey":"pk"}"#),
        ("POST", "/api/push/subscribe", 200, "{}"),
        ("DELETE", "/api/push/subscribe", 200, "{}"),
    ])
}

// ---------- api::* helpers ----------------------------------------------

#[test]
fn api_requests_happy_and_errors() {
    let stub = node_stub();
    let t = stub.target();

    assert_eq!(api::get_health(&t).unwrap()["ok"], json!(true));
    assert_eq!(api::get_node(&t).unwrap()["id"], json!("n1"));
    assert!(api::get_user(&t).unwrap().get("user").is_some());
    assert_eq!(api::list_agents(&t).unwrap().len(), 2);
    assert_eq!(api::pair_redeem(&t, "code").unwrap(), "secret-tok");
    assert_eq!(api::list_dirs(&t, "/a b").unwrap(), vec!["/a", "/b"]);

    let sessions = api::list_sessions(&t, true).unwrap();
    assert_eq!(sessions.len(), 2);
    assert!(sessions[0].locked);
    assert_eq!(api::list_sessions(&t, false).unwrap().len(), 2);

    assert_eq!(
        api::create_session(&t, &json!({"cwd":"/w"})).unwrap()["id"],
        json!("s9")
    );
    let page = api::get_history(&t, "s1", Some(2), Some(9), Some("devin")).unwrap();
    assert_eq!(page["total"], json!(3));
    let page = api::get_history(&t, "s1", None, None, None).unwrap();
    assert_eq!(page["start"], json!(5));
    assert_eq!(api::get_checkpoints(&t, "s1", Some("a")).unwrap().len(), 2);
    assert_eq!(
        api::export_session(&t, "s1", None).unwrap()["id"],
        json!("s1")
    );
    let att = api::attach(&t, "s1", true, Some("m"), Some(&["f".into()]), None).unwrap();
    assert!(att.attached);
    let att = api::attach(&t, "s1", false, None, None, None).unwrap();
    assert!(!att.read_only);
    api::prompt(&t, "s1", "hi", None).unwrap();
    api::cancel(&t, "s1", Some("a")).unwrap();
    api::respond_to_permission(&t, "s1", "r1", Some("o"), None).unwrap();
    api::respond_to_permission(&t, "s1", "r1", None, None).unwrap();
    api::delete_session(&t, "s1", None).unwrap();
    api::patch_session(&t, "s1", &json!({"title":"x"}), None).unwrap();
    assert_eq!(api::convert_session(&t, "s1", "cline").unwrap(), "s2");
    let imp = api::import_session(&t, &json!({"agent":"devin"})).unwrap();
    assert_eq!(imp.id, "s9");
    let rs = api::restore_session(&t, "s1", &json!({"path":"a"}), None).unwrap();
    assert_eq!(rs.restored[0].path, "a.rs");
    assert_eq!(rs.skipped[0].reason, "conflict");
    // selector that isn't an object → confirm-only body.
    let rw = api::rewind_session(&t, "s1", &json!(null), None).unwrap();
    assert_eq!(rw.removed, 2);
    assert_eq!(api::get_config(&t).unwrap()["k"], json!(1));
    api::set_config(&t, "k", &json!(2)).unwrap();
    assert_eq!(api::list_projects(&t).unwrap()[0].id, "p1");
    assert_eq!(api::create_project(&t, "n").unwrap().id, "p2");
    api::rename_project(&t, "p1", "x").unwrap();
    api::delete_project(&t, "p1").unwrap();
    assert_eq!(api::get_vapid(&t).unwrap(), "pk");
    api::push_subscribe(&t, &json!({"endpoint":"e"})).unwrap();
    api::push_unsubscribe(&t, "e").unwrap();

    // Error payload decode: `{error, code}` wins over friendly text.
    let err = Stub::serve(vec![(
        "GET",
        "/api/health",
        409,
        r#"{"error":"locked","code":"locked"}"#,
    )]);
    let e = api::get_health(&err.target()).unwrap_err();
    assert_eq!(e.status, Some(409));
    assert_eq!(e.code.as_deref(), Some("locked"));
    assert_eq!(e.message, "locked");

    // Error payload without `{error}` → friendly status text.
    let bare = Stub::serve(vec![("GET", "/api/health", 401, "{}")]);
    let e = api::get_health(&bare.target()).unwrap_err();
    assert_eq!(e.status, Some(401));
    assert!(e.message.contains("Unauthorized"));
    assert!(e.code.is_none());

    // Empty `{error}` also falls back.
    let empty = Stub::serve(vec![(
        "GET",
        "/api/health",
        404,
        r#"{"error":"","code":""}"#,
    )]);
    let e = api::get_health(&empty.target()).unwrap_err();
    assert_eq!(e.message, "Not found");

    // Non-JSON body on error → friendly text; on success → parse error.
    let text = Stub::serve(vec![("GET", "/api/health", 500, "oops")]);
    let e = api::get_health(&text.target()).unwrap_err();
    assert!(e.message.contains("server hit an error"));
    let bad = Stub::serve(vec![("GET", "/api/health", 200, "not json")]);
    assert!(api::get_health(&bad.target()).is_err());
    // Empty 200 body → Value::Null.
    let null = Stub::serve(vec![("GET", "/api/health", 200, "")]);
    assert_eq!(api::get_health(&null.target()).unwrap(), Value::Null);

    // Transport failure → the "Cannot reach" wrapper.
    let dead = api::resolve_target("http://127.0.0.1:1", None);
    let e = api::get_health(&dead).unwrap_err();
    assert!(e.message.contains("Cannot reach"));
    assert!(e.status.is_none());

    // Display + Error impls.
    let shown = format!(
        "{}",
        api::ApiError {
            message: "m".into(),
            status: None,
            code: None,
        }
    );
    assert_eq!(shown, "m");
    let _: &dyn std::error::Error = &api::ApiError {
        message: "m".into(),
        status: None,
        code: None,
    };
    let _: CliError = api::ApiError {
        message: "m".into(),
        status: None,
        code: None,
    }
    .into();
}

#[test]
fn api_friendly_statuses() {
    for (code, needle) in [(400u16, "rejected"), (403, "denied"), (418, "418")] {
        let stub = Stub::serve(vec![("GET", "/api/health", code, "{}")]);
        let e = api::get_health(&stub.target()).unwrap_err();
        assert!(e.message.contains(needle), "{code}: {}", e.message);
    }
}

#[test]
fn api_sse_streaming() {
    let stub = node_stub();
    let t = stub.target();
    let mut frames = Vec::new();
    api::stream_sse(&t, "GET", "/api/sessions/s1/stream", None, |f| {
        frames.push(f.clone())
    })
    .unwrap();
    assert_eq!(frames.len(), 2);
    assert_eq!(frames[0].event.as_deref(), Some("run"));
    assert_eq!(frames[0].data, "{\"k\":1}");
    assert_eq!(frames[1].event, None);
    assert_eq!(frames[1].data, "lone");

    // POST variant + non-2xx error path.
    let err = Stub::serve(vec![("POST", "/api/stream", 500, r#"{"error":"x"}"#)]);
    assert!(
        api::stream_sse(
            &err.target(),
            "POST",
            "/api/stream",
            Some(&json!({})),
            |_| {}
        )
        .is_err()
    );
    let dead = api::resolve_target("http://127.0.0.1:1", None);
    assert!(api::stream_sse(&dead, "GET", "/api/events", None, |_| {}).is_err());
}

#[test]
fn api_target_resolution() {
    assert_eq!(api::DEFAULT_NODE_URL, "http://127.0.0.1:8787");
    let t = api::resolve_target("http://x/", None);
    assert_eq!(t.base_url, "http://x");
    // PEREGRIN_TOKEN may or may not be set in the ambient env — either
    // way the explicit token wins.
    let t = api::resolve_target("http://x", Some("tok"));
    assert_eq!(t.token.as_deref(), Some("tok"));
    // `default_node_url` respects PEREGRIN_NODE_URL when present.
    let url = api::default_node_url();
    assert!(!url.is_empty());
}

// ---------- node_ops verbs ----------------------------------------------

#[test]
fn verbs_top_level() {
    let stub = node_stub();
    node_ops::health(&stub.node_args(false)).unwrap();
    node_ops::health(&stub.node_args(true)).unwrap();
    node_ops::node(&stub.node_args(false)).unwrap();
    node_ops::node(&stub.node_args(true)).unwrap();
    node_ops::agents(&stub.node_args(false)).unwrap();
    node_ops::agents(&stub.node_args(true)).unwrap();
    node_ops::user(&stub.node_args(false)).unwrap();
    node_ops::user(&stub.node_args(true)).unwrap();
    node_ops::fs("/a", &stub.node_args(false)).unwrap();
    node_ops::fs("/a", &stub.node_args(true)).unwrap();
    node_ops::events(&stub.target_args()).unwrap();
    node_ops::redeem("code", &stub.target_args()).unwrap();

    // Unhealthy shape → the other branch.
    let down = Stub::serve(vec![("GET", "/api/health", 200, r#"{"ok":false}"#)]);
    node_ops::health(&down.node_args(false)).unwrap();
    // Sparse node payload → default strings everywhere.
    let sparse = Stub::serve(vec![("GET", "/api/node", 200, "{}")]);
    node_ops::node(&sparse.node_args(false)).unwrap();
    // User missing the envelope → Null defaults.
    let nouser = Stub::serve(vec![("GET", "/api/user", 200, "{}")]);
    node_ops::user(&nouser.node_args(false)).unwrap();
    // Transport error propagates.
    let dead = NodeArgs {
        node: "http://127.0.0.1:1".into(),
        token: None,
        json: false,
    };
    assert!(node_ops::health(&dead).is_err());
}

#[test]
fn verbs_sessions() {
    let stub = node_stub();
    node_ops::sessions_list(false, &stub.node_args(false)).unwrap();
    node_ops::sessions_list(true, &stub.node_args(true)).unwrap();
    node_ops::sessions_create(
        "/w",
        Some("d"),
        Some("t"),
        Some("m"),
        &["f".into()],
        &stub.node_args(false),
    )
    .unwrap();
    node_ops::sessions_create("/w", None, None, None, &[], &stub.node_args(true)).unwrap();
    node_ops::sessions_attach(
        "s1",
        true,
        Some("m"),
        &["f".into()],
        None,
        &stub.node_args(false),
    )
    .unwrap();
    node_ops::sessions_attach("s1", false, None, &[], None, &stub.node_args(true)).unwrap();
    node_ops::sessions_prompt("s1", "hi", Some("a"), &stub.target_args()).unwrap();
    node_ops::sessions_cancel("s1", None, &stub.target_args()).unwrap();
    node_ops::sessions_permission("s1", "r1", Some("o"), None, &stub.target_args()).unwrap();
    node_ops::sessions_history("s1", None, None, None, &stub.node_args(false)).unwrap();
    node_ops::sessions_history("s1", Some(2), Some(9), None, &stub.node_args(true)).unwrap();
    node_ops::sessions_checkpoints("s1", None, &stub.node_args(false)).unwrap();
    node_ops::sessions_checkpoints("s1", None, &stub.node_args(true)).unwrap();
    node_ops::sessions_export("s1", None, None, &stub.target_args()).unwrap();
    node_ops::sessions_stream("s1", Some("a"), &stub.target_args()).unwrap();

    // Export to a file.
    let dir = tempfile::TempDir::new().unwrap();
    let out = dir.path().join("s.json");
    node_ops::sessions_export("s1", Some(out.to_str().unwrap()), None, &stub.target_args())
        .unwrap();
    assert!(out.exists());
    // Export to an unwritable path → CliError.
    assert!(
        node_ops::sessions_export(
            "s1",
            Some("/nonexistent-dir/x.json"),
            None,
            &stub.target_args()
        )
        .is_err()
    );

    // Meta: every patch arm + the conflict/empty guards.
    node_ops::sessions_meta(
        "s1",
        Some("t"),
        false,
        false,
        false,
        false,
        &[],
        None,
        false,
        None,
        &stub.target_args(),
    )
    .unwrap();
    node_ops::sessions_meta(
        "s1",
        None,
        true,
        false,
        false,
        false,
        &[],
        None,
        false,
        None,
        &stub.target_args(),
    )
    .unwrap();
    node_ops::sessions_meta(
        "s1",
        None,
        false,
        true,
        false,
        false,
        &[],
        None,
        false,
        None,
        &stub.target_args(),
    )
    .unwrap();
    node_ops::sessions_meta(
        "s1",
        None,
        false,
        false,
        true,
        false,
        &[],
        None,
        false,
        None,
        &stub.target_args(),
    )
    .unwrap();
    node_ops::sessions_meta(
        "s1",
        None,
        false,
        false,
        false,
        true,
        &[],
        None,
        false,
        None,
        &stub.target_args(),
    )
    .unwrap();
    node_ops::sessions_meta(
        "s1",
        None,
        false,
        false,
        false,
        false,
        &["p1".into()],
        None,
        false,
        None,
        &stub.target_args(),
    )
    .unwrap();
    node_ops::sessions_meta(
        "s1",
        None,
        false,
        false,
        false,
        false,
        &[],
        Some("m"),
        false,
        None,
        &stub.target_args(),
    )
    .unwrap();
    node_ops::sessions_meta(
        "s1",
        None,
        false,
        false,
        false,
        false,
        &[],
        None,
        true,
        None,
        &stub.target_args(),
    )
    .unwrap();
    assert!(
        node_ops::sessions_meta(
            "s1",
            None,
            true,
            true,
            false,
            false,
            &[],
            None,
            false,
            None,
            &stub.target_args()
        )
        .is_err()
    );
    assert!(
        node_ops::sessions_meta(
            "s1",
            None,
            false,
            false,
            true,
            true,
            &[],
            None,
            false,
            None,
            &stub.target_args()
        )
        .is_err()
    );
    assert!(
        node_ops::sessions_meta(
            "s1",
            None,
            false,
            false,
            false,
            false,
            &[],
            Some("m"),
            true,
            None,
            &stub.target_args()
        )
        .is_err()
    );
    assert!(
        node_ops::sessions_meta(
            "s1",
            None,
            false,
            false,
            false,
            false,
            &[],
            None,
            false,
            None,
            &stub.target_args()
        )
        .is_err()
    );

    node_ops::sessions_rename("s1", "t", None, &stub.target_args()).unwrap();
    node_ops::sessions_delete("s1", None, &stub.target_args()).unwrap();
    node_ops::sessions_convert("s1", "cline", &stub.target_args()).unwrap();

    // Import: array form, envelope form, IR form, bad JSON, missing file.
    let dir = tempfile::TempDir::new().unwrap();
    for (name, content) in [
        ("arr.json", r#"[{"role":"user"}]"#),
        ("env.json", r#"{"session":{"id":"s1"},"other":1}"#),
        ("ir.json", r#"{"id":"s1"}"#),
        ("nullenv.json", r#"{"session":null}"#),
        ("bad.json", "not json"),
    ] {
        std::fs::write(dir.path().join(name), content).unwrap();
    }
    for name in ["arr.json", "env.json", "ir.json", "nullenv.json"] {
        let p = dir.path().join(name);
        node_ops::sessions_import(
            p.to_str().unwrap(),
            "devin",
            Some("/w"),
            Some("t"),
            Some("m"),
            &stub.node_args(false),
        )
        .unwrap();
    }
    node_ops::sessions_import(
        dir.path().join("ir.json").to_str().unwrap(),
        "devin",
        None,
        None,
        None,
        &stub.node_args(true),
    )
    .unwrap();
    assert!(
        node_ops::sessions_import(
            dir.path().join("bad.json").to_str().unwrap(),
            "devin",
            None,
            None,
            None,
            &stub.node_args(false)
        )
        .is_err()
    );
    assert!(
        node_ops::sessions_import(
            dir.path().join("missing.json").to_str().unwrap(),
            "devin",
            None,
            None,
            None,
            &stub.node_args(false)
        )
        .is_err()
    );
}

#[test]
fn verbs_sessions_restore_rewind() {
    let stub = node_stub();
    // Restore: path selector, checkpoint+paths selector, json output.
    node_ops::sessions_restore(
        "s1",
        true,
        Some("a.rs"),
        Some("tc"),
        None,
        &[],
        None,
        &stub.node_args(false),
    )
    .unwrap();
    node_ops::sessions_restore(
        "s1",
        true,
        None,
        None,
        Some("cp1"),
        &["p".into()],
        None,
        &stub.node_args(true),
    )
    .unwrap();
    node_ops::sessions_restore(
        "s1",
        true,
        None,
        None,
        Some("cp1"),
        &[],
        None,
        &stub.node_args(false),
    )
    .unwrap();
    assert!(
        node_ops::sessions_restore(
            "s1",
            false,
            Some("a"),
            None,
            None,
            &[],
            None,
            &stub.node_args(false)
        )
        .is_err()
    );
    assert!(
        node_ops::sessions_restore(
            "s1",
            true,
            Some("a"),
            None,
            Some("c"),
            &[],
            None,
            &stub.node_args(false)
        )
        .is_err()
    );
    assert!(
        node_ops::sessions_restore(
            "s1",
            true,
            None,
            None,
            None,
            &[],
            None,
            &stub.node_args(false)
        )
        .is_err()
    );

    // Rewind: one selector each + json + guards.
    node_ops::sessions_rewind(
        "s1",
        true,
        Some(3),
        None,
        None,
        None,
        &stub.node_args(false),
    )
    .unwrap();
    node_ops::sessions_rewind("s1", true, None, Some(2), None, None, &stub.node_args(true))
        .unwrap();
    node_ops::sessions_rewind(
        "s1",
        true,
        None,
        None,
        Some("cp"),
        None,
        &stub.node_args(false),
    )
    .unwrap();
    assert!(
        node_ops::sessions_rewind(
            "s1",
            false,
            Some(1),
            None,
            None,
            None,
            &stub.node_args(false)
        )
        .is_err()
    );
    assert!(
        node_ops::sessions_rewind(
            "s1",
            true,
            Some(1),
            Some(2),
            None,
            None,
            &stub.node_args(false)
        )
        .is_err()
    );
    assert!(
        node_ops::sessions_rewind("s1", true, None, None, None, None, &stub.node_args(false))
            .is_err()
    );
}

#[test]
fn verbs_projects_config_push() {
    let stub = node_stub();
    node_ops::projects_list(&stub.node_args(false)).unwrap();
    node_ops::projects_list(&stub.node_args(true)).unwrap();
    node_ops::projects_create("n", &stub.target_args()).unwrap();
    node_ops::projects_rename("p1", "x", &stub.target_args()).unwrap();
    node_ops::projects_delete("p1", &stub.target_args()).unwrap();
    node_ops::config_get(&stub.node_args(false)).unwrap();
    node_ops::config_get(&stub.node_args(true)).unwrap();
    node_ops::config_set("k", "2", &stub.target_args()).unwrap();
    node_ops::config_set("k", "plain text", &stub.target_args()).unwrap();
    node_ops::push_vapid(&stub.target_args()).unwrap();
    node_ops::push_subscribe(r#"{"endpoint":"e"}"#, &stub.target_args()).unwrap();
    assert!(node_ops::push_subscribe("nope", &stub.target_args()).is_err());
    node_ops::push_unsubscribe("e", &stub.target_args()).unwrap();
}

#[test]
fn verbs_resume() {
    // Export available → IR path.
    let stub = node_stub();
    node_ops::sessions_resume(
        "s1",
        "devin",
        &stub.base,
        None,
        None,
        Some("/w"),
        Some("t"),
        Some("m"),
        &stub.node_args(false),
    )
    .unwrap();
    node_ops::sessions_resume(
        "s1",
        "devin",
        &stub.base,
        None,
        Some("agent"),
        None,
        None,
        None,
        &stub.node_args(true),
    )
    .unwrap();

    // Export 404s → history paging fallback (two pages then stop).
    let stub2 = Stub::serve(vec![
        ("GET", "/api/sessions/s1/export", 404, "{}"),
        (
            "GET",
            "/api/sessions/s1/history?limit=500&before=7",
            200,
            r#"{"messages":[],"total":0,"start":0}"#,
        ),
        (
            "GET",
            "/api/sessions/s1/history",
            200,
            r#"{"messages":[{"role":"user","content":"x"}],"total":9,"start":7}"#,
        ),
        (
            "POST",
            "/api/sessions/import",
            200,
            r#"{"id":"s9","title":"t","cwd":"/w","agent":"d","updatedAt":"u"}"#,
        ),
    ]);
    node_ops::sessions_resume(
        "s1",
        "devin",
        &stub2.base,
        Some("stok"),
        None,
        None,
        None,
        None,
        &stub2.node_args(false),
    )
    .unwrap();
    // Empty first page stops the loop immediately.
    let stub3 = Stub::serve(vec![
        ("GET", "/api/sessions/s1/export", 404, "{}"),
        (
            "GET",
            "/api/sessions/s1/history",
            200,
            r#"{"messages":[],"total":0,"start":3}"#,
        ),
        (
            "POST",
            "/api/sessions/import",
            200,
            r#"{"id":"s9","title":"t","cwd":"/w","agent":"d","updatedAt":"u"}"#,
        ),
    ]);
    node_ops::sessions_resume(
        "s1",
        "devin",
        &stub3.base,
        None,
        None,
        None,
        None,
        None,
        &stub3.node_args(false),
    )
    .unwrap();
    // Export errors non-404 → propagate.
    let stub4 = Stub::serve(vec![("GET", "/api/sessions/s1/export", 500, "{}")]);
    assert!(
        node_ops::sessions_resume(
            "s1",
            "devin",
            &stub4.base,
            None,
            None,
            None,
            None,
            None,
            &stub4.node_args(false)
        )
        .is_err()
    );
    // History errors in the fallback → propagate.
    let stub5 = Stub::serve(vec![
        ("GET", "/api/sessions/s1/export", 404, "{}"),
        ("GET", "/api/sessions/s1/history", 500, "{}"),
    ]);
    assert!(
        node_ops::sessions_resume(
            "s1",
            "devin",
            &stub5.base,
            None,
            None,
            None,
            None,
            None,
            &stub5.node_args(false)
        )
        .is_err()
    );
}

#[test]
fn wire_request_shapes() {
    let stub = node_stub();
    let t = stub.target();
    api::get_checkpoints(&t, "s1", Some("a gent")).unwrap();
    api::get_history(&t, "s1", Some(5), None, Some("a")).unwrap();
    node_ops::sessions_stream("s1", Some("a gent"), &stub.target_args()).unwrap();
    let lines = stub.lines();
    assert!(
        lines
            .iter()
            .any(|l| l.contains("/checkpoints?agent=a%20gent"))
    );
    assert!(lines.iter().any(|l| l.contains("/history?limit=5&agent=a")));
    assert!(lines.iter().any(|l| l.contains("/stream?agent=a%20gent")));
    // list with ?withLocks=1
    api::list_sessions(&t, true).unwrap();
    api::list_sessions(&t, false).unwrap();
    let lines = stub.lines();
    assert!(lines.iter().any(|l| l.contains("withLocks")));
    assert!(lines.iter().any(|l| l == "GET /api/sessions HTTP/1.1"));
}
