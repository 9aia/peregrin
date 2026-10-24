//! SSR render coverage beyond `ssr_pages.rs` — every route under the
//! alternate fixture worlds: upstream errors, empty collections, the
//! pending-writes badges, all-down nodes, archived-only lists, and a
//! second history page.
#![cfg(feature = "ssr")]
#![allow(clippy::unwrap_used, clippy::pedantic)]

use std::sync::Arc;

use futures::StreamExt;
use hydration_context::SsrSharedContext;
use leptos::prelude::*;
use leptos_router::location::RequestUrl;
use peregrin_web::App;
use peregrin_web::api::NodeApi;
use peregrin_web::testing::FakeNodeApi;

async fn render(path: &str, api: Arc<dyn NodeApi>) -> String {
    let owner = Owner::new_root(Some(Arc::new(SsrSharedContext::new())));
    let stream = owner.with(|| {
        provide_context(RequestUrl::new(path));
        provide_context::<Arc<dyn NodeApi>>(api);
        view! { <App/> }.to_html_stream_in_order()
    });
    let html = stream.collect::<String>().await;
    drop(owner);
    html
}

async fn page(path: &str, api: Arc<dyn NodeApi>) -> String {
    static RENDER_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
    let _serialize = RENDER_LOCK.lock().await;
    let _ = any_spawner::Executor::init_tokio();
    tokio::task::LocalSet::new()
        .run_until(render(path, api))
        .await
}

const ROUTES: &[&str] = &[
    "/",
    "/sessions/s1",
    "/sessions/s2",
    "/sessions/s3",
    "/sessions/s5",
    "/sessions/s6",
    "/sessions/missing",
    "/agents",
    "/projects",
    "/nodes",
    "/settings",
    "/login",
    "/totally-unknown",
];

#[tokio::test]
async fn seeded_all_routes() {
    for route in ROUTES {
        let html = page(route, FakeNodeApi::seeded().shared()).await;
        assert!(!html.is_empty(), "{route} rendered empty");
    }
}

#[tokio::test]
async fn error_all_routes() {
    let api = || {
        FakeNodeApi::seeded()
            .with_error("node unreachable")
            .shared()
    };
    for route in ROUTES {
        let html = page(route, api()).await;
        assert!(!html.is_empty(), "{route} rendered empty");
    }
}

#[tokio::test]
async fn empty_and_degraded_worlds() {
    let empty = || {
        FakeNodeApi::seeded()
            .with_sessions(Vec::new())
            .with_statuses(Vec::new())
            .shared()
    };
    for route in [
        "/",
        "/agents",
        "/projects",
        "/nodes",
        "/settings",
        "/sessions/s1",
    ] {
        let html = page(route, empty()).await;
        assert!(!html.is_empty(), "{route} rendered empty");
    }
    // Every node down → the session list's "no nodes" gate.
    use peregrin_web::dto::{NodeStatusDto, PendingWriteDto};
    let down = NodeStatusDto {
        id: "n".into(),
        url: "u".into(),
        label: "l".into(),
        status: "down".into(),
        last_seen_at: None,
    };
    let pending = PendingWriteDto {
        id: "w1".into(),
        node_id: "n".into(),
        session_id: "s1".into(),
        op: "prompt".into(),
        kind: "turn".into(),
        status: "failed".into(),
        enqueued_at: "t".into(),
        attempts: 2,
        last_error: Some("e".into()),
    };
    let api = || {
        FakeNodeApi::seeded()
            .with_statuses(vec![down.clone()])
            .with_pending(vec![pending.clone()])
            .shared()
    };
    for route in ["/", "/nodes", "/sessions/s1", "/settings"] {
        let html = page(route, api()).await;
        assert!(!html.is_empty(), "{route} rendered empty");
    }
}
