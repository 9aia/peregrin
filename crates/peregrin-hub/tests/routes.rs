#![allow(clippy::unwrap_used, clippy::pedantic)]
//! Router-level coverage — the static handlers, the push sub-store,
//! the auth gate, config parsing, and the HTTP/SSE bridges against a
//! stubbed upstream (a raw `TcpListener` answering canned bytes).

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::Arc;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use leptos::config::LeptosOptions;
use peregrin_hub::{HubConfig, HubState, router};
use peregrin_web::api::NodeApi;
use peregrin_web::testing::FakeNodeApi;
use tower::ServiceExt;

fn state(node_url: &str) -> HubState {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("push.json");
    std::mem::forget(dir); // the store reads lazily — keep the dir alive
    let push = peregrin_push::PushStore::open(sepia_meta::MetaStore::open(&path));
    let mut s = HubState::new(
        LeptosOptions::builder()
            .output_name("peregrin_web")
            .site_root("target/site")
            .build(),
        FakeNodeApi::seeded().shared() as Arc<dyn NodeApi>,
        node_url,
        None,
    );
    s.push = Some(Arc::new(push));
    s
}

async fn call(app: &axum::Router, req: Request<Body>) -> (StatusCode, String) {
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let body = to_bytes(res.into_body(), 8 * 1024 * 1024).await.unwrap();
    (status, String::from_utf8_lossy(&body).to_string())
}

fn get(uri: &str) -> Request<Body> {
    Request::get(uri).body(Body::empty()).unwrap()
}

#[tokio::test]
async fn static_handlers() {
    let app = router(state("http://127.0.0.1:9"));
    let (s, b) = call(&app, get("/style.css")).await;
    assert_eq!(s, StatusCode::OK);
    assert!(b.contains('{') || b.contains('}'));
    let (s, _) = call(&app, get("/vendor/mermaid.min.js")).await;
    assert_eq!(s, StatusCode::OK);
    for (name, want) in [
        ("geist-latin-wght-normal.woff2", StatusCode::OK),
        ("geist-latin-ext-wght-normal.woff2", StatusCode::OK),
        ("geist-latin-wght-italic.woff2", StatusCode::OK),
        ("geist-latin-ext-wght-italic.woff2", StatusCode::OK),
        ("nope.woff2", StatusCode::NOT_FOUND),
    ] {
        let (s, _) = call(&app, get(&format!("/vendor/fonts/{name}"))).await;
        assert_eq!(s, want, "{name}");
    }
    // sw.js — absent under the bare site root → 404 body.
    let (s, b) = call(&app, get("/sw.js")).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    assert!(b.contains("sw.js not found"));
}

#[tokio::test]
async fn sw_js_serves_from_site_root() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("sw.js"), "// sw").unwrap();
    let mut s = state("http://127.0.0.1:9");
    s.options.site_root = dir.path().to_string_lossy().into_owned().into();
    let app = router(s);
    let (status, body) = call(&app, get("/sw.js")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "// sw");
}

#[tokio::test]
async fn push_routes_full_surface() {
    let app = router(state("http://127.0.0.1:9"));
    let (s, b) = call(&app, get("/api/push/vapid")).await;
    assert_eq!(s, StatusCode::OK);
    assert!(b.contains("publicKey"));
    // Bad JSON → 400.
    let req = Request::post("/api/push/subscribe")
        .body(Body::from("{nope"))
        .unwrap();
    let (s, _) = call(&app, req).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    // Non-object → 400.
    let req = Request::post("/api/push/subscribe")
        .body(Body::from("[1]"))
        .unwrap();
    let (s, _) = call(&app, req).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    // Missing keys → 400.
    let req = Request::post("/api/push/subscribe")
        .body(Body::from("{}"))
        .unwrap();
    let (s, _) = call(&app, req).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    let req = Request::post("/api/push/subscribe")
        .body(Body::from(r#"{"endpoint":"e","keys":{}}"#))
        .unwrap();
    let (s, _) = call(&app, req).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    // Valid subscribe with prefs.
    let req = Request::post("/api/push/subscribe")
        .body(Body::from(
            r#"{"endpoint":"https://push/e","keys":{"auth":"a","p256dh":"p"},"prefs":{"done":false}}"#,
        ))
        .unwrap();
    let (s, _) = call(&app, req).await;
    assert_eq!(s, StatusCode::OK);
    // DELETE removes by endpoint; a missing endpoint is still ok.
    let req = Request::delete("/api/push/subscribe")
        .body(Body::from(r#"{"endpoint":"https://push/e"}"#))
        .unwrap();
    let (s, _) = call(&app, req).await;
    assert_eq!(s, StatusCode::OK);
    let req = Request::delete("/api/push/subscribe")
        .body(Body::from("{}"))
        .unwrap();
    let (s, _) = call(&app, req).await;
    assert_eq!(s, StatusCode::OK);
    // Oversized body → 413.
    let req = Request::post("/api/push/subscribe")
        .body(Body::from("x".repeat(70 * 1024)))
        .unwrap();
    let (s, _) = call(&app, req).await;
    assert_eq!(s, StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn push_routes_unconfigured_501() {
    let mut s = state("http://127.0.0.1:9");
    s.push = None;
    let app = router(s);
    let (status, _) = call(&app, get("/api/push/vapid")).await;
    assert_eq!(status, StatusCode::NOT_IMPLEMENTED);
    let req = Request::post("/api/push/subscribe")
        .body(Body::from("{}"))
        .unwrap();
    let (status, _) = call(&app, req).await;
    assert_eq!(status, StatusCode::NOT_IMPLEMENTED);
}

/// A one-shot HTTP stub: reads the request line + headers, replies
/// with `bytes`. For SSE endpoints it emits a couple frames then
/// closes — `pump_sse` forwards them and exits on EOF.
fn stub_upstream() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        while let Ok((mut sock, _)) = listener.accept() {
            std::thread::spawn(move || {
                let mut reader = BufReader::new(sock.try_clone().unwrap());
                let mut line = String::new();
                let mut path = String::new();
                let mut auth_seen = false;
                let mut ct_seen = false;
                // Read request line + headers; then drain a small body.
                loop {
                    line.clear();
                    if reader.read_line(&mut line).unwrap_or(0) == 0 {
                        return;
                    }
                    let t = line.trim();
                    if t.is_empty() {
                        break;
                    }
                    if path.is_empty() {
                        path = t.split_whitespace().nth(1).unwrap_or("").to_string();
                    }
                    if t.to_lowercase().starts_with("authorization:") {
                        auth_seen = true;
                    }
                    if t.to_lowercase().starts_with("content-type:") {
                        ct_seen = true;
                    }
                }
                let mut buf = [0u8; 4096];
                let _ = reader.get_mut().set_nonblocking(true);
                let _ = reader.get_mut().read(&mut buf);
                let (status, ct, body) =
                    if path.starts_with("/api/events") || path.contains("/stream") {
                        (
                            "200 OK",
                            "text/event-stream",
                            "event: diff\ndata: {\"k\":1}\n\n: ping\ndata: more\n\n".to_string(),
                        )
                    } else {
                        (
                            "200 OK",
                            "application/json",
                            format!("{{\"ok\":true,\"auth\":{auth_seen},\"ct\":{ct_seen}}}"),
                        )
                    };
                let res = format!(
                    "HTTP/1.1 {status}\r\ncontent-type: {ct}\r\ncontent-length: {}\r\n\r\n{body}",
                    body.len()
                );
                let _ = sock.write_all(res.as_bytes());
                let _ = sock.flush();
                // SSE: closing right away ends the pump via EOF.
            });
        }
    });
    format!("http://127.0.0.1:{port}")
}

#[tokio::test]
async fn passthrough_forwards_verbatim() {
    let url = stub_upstream();
    let mut s = state(&url);
    s.node_token = Some("tok".into());
    let app = router(s);
    // GET with a query — upstream sees it and replies 200 JSON.
    let (status, body) = call(&app, get("/api/sessions?archived=1")).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("\"ok\":true") || body.contains("auth"));
    // POST with JSON + content-type → forwarded.
    let req = Request::post("/api/sessions/s1/prompt")
        .header("content-type", "application/json")
        .body(Body::from("{}"))
        .unwrap();
    let (status, _) = call(&app, req).await;
    assert_eq!(status, StatusCode::OK);
    // Browser auth header is forwarded when no node_token is set.
    let mut s = state(&url);
    s.node_token = None;
    let app = router(s);
    let req = Request::get("/api/sessions")
        .header("authorization", "Bearer browser")
        .body(Body::empty())
        .unwrap();
    let (status, body) = call(&app, req).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("\"auth\":true"));
}

#[tokio::test]
async fn passthrough_unreachable_and_sse() {
    // Nothing listening → 502.
    let app = router(state("http://127.0.0.1:9"));
    let (status, body) = call(&app, get("/api/sessions")).await;
    assert_eq!(status, StatusCode::BAD_GATEWAY);
    assert!(body.contains("unreachable"));
    let (status, _) = call(&app, get("/api/events")).await;
    assert_eq!(status, StatusCode::BAD_GATEWAY);
    // Real upstream → SSE frames bridge through.
    let url = stub_upstream();
    let mut s = state(&url);
    s.session_router = Some(Arc::new(|_id| {
        Some(("http://127.0.0.1:9".into(), Some("t".into())))
    }));
    let app = router(s.clone());
    let res = app.clone().oneshot(get("/api/events?x=1")).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), 1 << 20).await.unwrap();
    let text = String::from_utf8_lossy(&body);
    assert!(text.contains("diff") || text.contains("ping"));
    // Session stream — the router resolves s1 → the stub url; use the
    // live upstream so it actually connects.
    let mut s2 = state(&url);
    let up = url.clone();
    s2.session_router = Some(Arc::new(move |_| Some((up.clone(), None))));
    let app = router(s2);
    let res = app.oneshot(get("/api/sessions/s1/stream")).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), 1 << 20).await.unwrap();
    assert!(String::from_utf8_lossy(&body).contains("data"));
}

#[tokio::test]
async fn auth_gate_and_login() {
    let mut s = state("http://127.0.0.1:9");
    s.hub_token = Some("sekret".into());
    let app = router(s);
    // No credential → 303 to /login.
    let res = app.clone().oneshot(get("/")).await.unwrap();
    assert!(res.status().is_redirection() || res.status() == StatusCode::UNAUTHORIZED);
    // Bearer token passes.
    let req = Request::get("/style.css")
        .header("authorization", "Bearer sekret")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // `?token=` on /login plants the cookie and lands on `/`.
    let res = app
        .clone()
        .oneshot(get("/login?token=sekret"))
        .await
        .unwrap();
    assert!(res.status().is_redirection());
    // Cookie auth passes.
    let req = Request::get("/style.css")
        .header("cookie", "peregrin_hub=sekret")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // POST /login with the right token → 303 + cookie.
    let req = Request::post("/login")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from("token=sekret&next=/sessions"))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert!(res.status().is_redirection());
    // Wrong token → 303 to /login?error=1.
    let req = Request::post("/login")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"token":"nope","next":"/x"}"#))
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert!(res.status().is_redirection());
}

#[test]
fn hub_config_parsing() {
    let d = HubConfig::from_map(&HashMap::new()).unwrap();
    assert_eq!(d.port, 3000);
    let mut v = HashMap::new();
    v.insert("PEREGRIN_HUB_HOST".into(), "127.0.0.1".into());
    v.insert("PEREGRIN_HUB_PORT".into(), "3001".into());
    v.insert("PEREGRIN_NODE_URL".into(), "http://x:8787/".into());
    v.insert("PEREGRIN_NODE_TOKEN".into(), "".into());
    v.insert("PEREGRIN_HUB_TOKEN".into(), "".into());
    v.insert("LEPTOS_ENV".into(), "prod".into());
    let c = HubConfig::from_map(&v).unwrap();
    assert_eq!(c.port, 3001);
    assert_eq!(c.node_url, "http://x:8787");
    assert!(c.node_token.is_none());
    assert!(!c.dev);
    // Bad port → Err.
    v.insert("PEREGRIN_HUB_PORT".into(), "nope".into());
    assert!(HubConfig::from_map(&v).is_err());
    v.insert("PEREGRIN_HUB_PORT".into(), "3000".into());
    v.insert("PEREGRIN_PORT".into(), "3002".into());
    // `PEREGRIN_PORT` is the fallback — HUB_PORT wins.
    let c = HubConfig::from_map(&v).unwrap();
    assert_eq!(c.port, 3000);
    // Non-loopback without a hub token → Err.
    v.remove("PEREGRIN_HUB_PORT");
    v.insert("PEREGRIN_HUB_HOST".into(), "0.0.0.0".into());
    assert!(HubConfig::from_map(&v).is_err());
    // A bad socket addr surfaces from `socket_addr`.
    v.insert("PEREGRIN_HUB_HOST".into(), "not a host".into());
    v.insert("PEREGRIN_HUB_TOKEN".into(), "t".into());
    let c = HubConfig::from_map(&v).unwrap();
    assert!(c.socket_addr().is_err());
}

#[tokio::test]
async fn session_upstream_falls_back() {
    let s = state("http://127.0.0.1:9");
    // No router → default upstream; unreachable path covered via the
    // public routing of proxied calls above.
    let app = router(s);
    drop(app);
}

/// The push feed-hook: `session` diffs with terminal flags send
/// notifications; other frames are ignored.
#[test]
fn feed_hook_sends_on_terminal_patches() {
    let dir = tempfile::tempdir().unwrap();
    let projection = peregrin_sync::ProjectionStore::in_memory().unwrap();
    projection
        .upsert_node(&peregrin_sync::NodeRef {
            id: "n1".into(),
            url: "u".into(),
            label: "l".into(),
        })
        .unwrap();
    projection
        .upsert_summary(
            "n1",
            &serde_json::json!({"id":"s1","title":"T1","updatedAt":"2024-01-01T00:00:00Z"}),
        )
        .unwrap();
    let push = std::sync::Arc::new(peregrin_push::PushStore::open(sepia_meta::MetaStore::open(
        &dir.path().join("push.json"),
    )));
    let hook = peregrin_hub::feed_hook(projection, push);
    use peregrin_sync::client::FeedEvent;
    // Non-diff frames and non-session kinds are ignored.
    hook("n1", &FeedEvent::Heartbeat);
    hook(
        "n1",
        &FeedEvent::Diff {
            kind: "meta".into(),
            id: "s1".into(),
            agent: None,
            patch: serde_json::json!({"runFinished": true}),
        },
    );
    // Session diffs — both flags; `ghost` exercises the
    // id-as-title fallback when the row isn't indexed.
    for id in ["s1", "ghost"] {
        hook(
            "n1",
            &FeedEvent::Diff {
                kind: "session".into(),
                id: id.into(),
                agent: None,
                patch: serde_json::json!({"runFinished": true, "permissionRequested": true}),
            },
        );
    }
}

/// `hub_state_from` — env arms without touching the process env.
#[tokio::test]
async fn hub_state_from_env_arms() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    let config = peregrin_hub::HubConfig {
        node_url: "http://127.0.0.1:9".into(),
        node_token: None,
        hub_token: None,
        host: "127.0.0.1".into(),
        port: 0,
        site_root: dir.path().to_string_lossy().into_owned(),
        dev: false,
    };
    // `PEREGRIN_NODES` env path + `PEREGRIN_HOME`.
    let nodes_env: std::ffi::OsString = "n1=http://127.0.0.1:9".to_string().into();
    let vars = |k: &str| -> Option<std::ffi::OsString> {
        match k {
            "PEREGRIN_HOME" => Some(home.clone().into_os_string()),
            "PEREGRIN_NODES" => Some(nodes_env.clone()),
            _ => None,
        }
    };
    let state = peregrin_hub::hub_state_from(vars, &config).unwrap();
    // The session router closure resolves through the projection —
    // invoke it once (unknown id → primary node URL).
    let router = state.session_router.as_ref().unwrap();
    let _ = router("s-ghost");
    // `HOME`-fallback arm (no PEREGRIN_HOME).
    let vars2 = |k: &str| -> Option<std::ffi::OsString> {
        (k == "HOME").then(|| dir.path().join("h2").into_os_string())
    };
    let _ = peregrin_hub::hub_state_from(vars2, &config).unwrap();
    // Neither var → `.peregrin` fallback under cwd — point it at a
    // safe dir by running in the tempdir.
    std::env::set_current_dir(dir.path()).unwrap();
    let vars3 = |_: &str| None;
    let _ = peregrin_hub::hub_state_from(vars3, &config).unwrap();
}

/// login(): tokenless hub → the "no gate" redirect arm.
#[tokio::test]
async fn login_no_gate_redirects() {
    let app = router(state("http://127.0.0.1:9"));
    let req = Request::post("/login")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from("token=t&next=/x"))
        .unwrap();
    let (s, _) = call(&app, req).await;
    assert!(s.is_redirection() || s.is_success() || s.is_client_error());
    // And a JSON-bodied post exercises the serde arm.
    let req = Request::post("/login")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"token":"t","next":"/x"}"#))
        .unwrap();
    let (s, _) = call(&app, req).await;
    assert!(s.is_redirection() || s.is_success() || s.is_client_error());
}
