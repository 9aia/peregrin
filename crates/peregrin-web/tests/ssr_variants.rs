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
    // `?session=` — the palette's pin/archive action rows read it:
    // s1 plain, s4 pinned (Unpin), s5 archived (Unarchive), s7 untitled
    // (id fallback).
    "/?session=s1&agent=devin",
    "/?session=s4&agent=devin",
    "/?session=s5&agent=devin",
    "/?session=s7&agent=claude",
    // `?closed` — menus pinned shut → the "hidden" class arms.
    "/?closed=1",
    "/sessions/s1",
    "/sessions/s2",
    "/sessions/s3",
    "/sessions/s5",
    "/sessions/s6",
    "/sessions/s7",
    "/sessions/missing",
    "/agents",
    "/projects",
    "/nodes",
    // Seeded-pair Err arm (`result = Some(Err(..))`).
    "/nodes?pair-err=1",
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

/// The single-resource empty arms — "No agents configured.", "No
/// projects yet.", "No checkpoints recorded.", "No settings
/// exposed." — plus the `?nodetail` sheet fallback and the empty-name
/// node card.
#[tokio::test]
async fn empty_resource_arms() {
    use peregrin_web::dto::NodeInfoDto;
    use std::collections::BTreeMap;
    let empty = || {
        FakeNodeApi::seeded()
            .with_agents(Vec::new())
            .with_projects(Vec::new())
            .with_checkpoints(Vec::new())
            .with_config(BTreeMap::new())
            .with_info(NodeInfoDto {
                id: "workbench".into(),
                name: String::new(),
                ..Default::default()
            })
            .shared()
    };
    for route in [
        "/",
        "/agents",
        "/projects",
        "/projects?nodetail=1",
        "/nodes",
        "/settings",
        "/sessions/s1",
        "/sessions/s7",
    ] {
        let html = page(route, empty()).await;
        assert!(!html.is_empty(), "{route} rendered empty");
    }
}

/// Odd-shaped rows — unlabeled agents/nodes, cross-node badges, the
/// audio capability chip, and plural/singular pending-write badges.
#[tokio::test]
async fn odd_row_variants() {
    use peregrin_web::dto::{
        AgentCapabilitiesDto, AgentDto, NodeStatusDto, PendingWriteDto, PromptCapabilitiesDto,
        SessionCapabilitiesDto,
    };
    let api = || {
        FakeNodeApi::seeded()
            .with_agents(vec![
                AgentDto {
                    id: "unlabeled".into(),
                    label: String::new(),
                    capabilities: None,
                    node: Some("tower".into()),
                },
                AgentDto {
                    id: "talker".into(),
                    label: "Talker".into(),
                    capabilities: Some(AgentCapabilitiesDto {
                        prompt_capabilities: PromptCapabilitiesDto {
                            audio: true,
                            ..PromptCapabilitiesDto::default()
                        },
                        session_capabilities: SessionCapabilitiesDto::default(),
                        ..AgentCapabilitiesDto::default()
                    }),
                    node: None,
                },
            ])
            .with_projects(vec![peregrin_web::dto::ProjectDto {
                id: "p9".into(),
                name: "Remote".into(),
                node: Some("tower".into()),
            }])
            .with_statuses(vec![
                NodeStatusDto {
                    id: "n-unlabeled".into(),
                    url: "u".into(),
                    label: String::new(),
                    status: "down".into(),
                    last_seen_at: None,
                },
                NodeStatusDto {
                    id: "n2".into(),
                    url: "u2".into(),
                    label: "two".into(),
                    status: "stale".into(),
                    last_seen_at: None,
                },
            ])
            .with_pending(vec![
                PendingWriteDto {
                    id: "w1".into(),
                    node_id: "workbench".into(),
                    session_id: "s1".into(),
                    op: "prompt".into(),
                    kind: "turn".into(),
                    status: "queued".into(),
                    enqueued_at: "t".into(),
                    attempts: 1,
                    last_error: None,
                },
                PendingWriteDto {
                    id: "w2".into(),
                    node_id: "workbench".into(),
                    session_id: "s1".into(),
                    op: "prompt".into(),
                    kind: "turn".into(),
                    status: "queued".into(),
                    enqueued_at: "t".into(),
                    attempts: 1,
                    last_error: None,
                },
                PendingWriteDto {
                    id: "w3".into(),
                    node_id: "workbench".into(),
                    session_id: "s1".into(),
                    op: "cancel".into(),
                    kind: "metadata".into(),
                    status: "failed".into(),
                    enqueued_at: "t".into(),
                    attempts: 3,
                    last_error: Some("dead".into()),
                },
                // Two failures → the `failed ×N` plural badge arm.
                PendingWriteDto {
                    id: "w4".into(),
                    node_id: "workbench".into(),
                    session_id: "s1".into(),
                    op: "cancel".into(),
                    kind: "metadata".into(),
                    status: "failed".into(),
                    enqueued_at: "t".into(),
                    attempts: 2,
                    last_error: Some("dead again".into()),
                },
            ])
            .with_history(peregrin_web::dto::HistoryPageDto {
                start: 0,
                total: 0,
                messages: Vec::new(),
            })
            .shared()
    };
    for route in [
        "/",
        "/agents",
        "/projects",
        "/nodes",
        "/settings",
        "/sessions/s1",
        "/sessions/s7",
    ] {
        let html = page(route, api()).await;
        assert!(!html.is_empty(), "{route} rendered empty");
    }
}
