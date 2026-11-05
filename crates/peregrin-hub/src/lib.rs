//! peregrin-hub — the node-API gateway. Bridges client traffic to the
//! node daemons and runs the multi-node sync engine:
//!
//! - `/api/sessions/{id}/stream`, `/api/events` — SSE bridges to
//!   `PEREGRIN_NODE_URL` (EventSource can't carry cross-origin auth
//!   cleanly; proxying keeps the client same-origin).
//! - `/api/push/*` — the hub-owned push subscription store (one
//!   subscription fans out to every paired node).
//! - `/api/*` — generic passthrough for the rest of the node API,
//!   session-scoped calls routed to the owning node on multi-node
//!   setups (`PEREGRIN_NODES`).
//!
//! [`sync_api::SyncEntityApi`] is the hub's [`NodeApi`]: session lists
//! read the merged projection and writes queue in the durable outbox
//! when a node is down.

pub mod auth;
pub mod credentials;
pub mod proxy;
pub mod push_routes;
pub mod sync_api;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::extract::Request;
use axum::middleware::{self, Next};
use axum::response::Response;
use axum::routing::{any, get};
use peregrin_node_api::api::EntityApi;
use tracing::Instrument;

/// Everything the router needs — the node-API port (`EntityApi` is a
/// trait so tests can stub it) and the upstream address for the
/// HTTP/SSE bridges.
/// Entity id → `(url, token)` of the owning node.
pub type EntityRouter = Arc<dyn Fn(&str) -> Option<(String, Option<String>)> + Send + Sync>;

#[derive(Clone)]
pub struct HubState {
    pub node: Arc<dyn EntityApi>,
    pub node_url: Arc<str>,
    pub node_token: Option<Arc<str>>,
    /// Multi-node routing — resolves an entity id to its owning
    /// node's `(url, token)` for entity-scoped proxy calls.
    /// `None` → everything uses `node_url` (single-node mode).
    pub entity_router: Option<EntityRouter>,
    /// Hub-owned push subscription store — `None` → `/api/push/*` 501s.
    pub push: Option<Arc<peregrin_push::PushStore>>,
    /// `PEREGRIN_HUB_TOKEN` — the client→hub gate. `None` → every route
    /// is open (loopback-only deployments).
    pub hub_token: Option<Arc<str>>,
    /// Long-lived streams: no global deadline and no body budget —
    /// ureq's `recv_body` is a total deadline that fails every read
    /// instantly once spent; a half-dead client is instead detected
    /// when the next frame's `blocking_send` fails.
    sse_agent: ureq::Agent,
    /// Bounded request/response passthroughs.
    http_agent: ureq::Agent,
}

impl HubState {
    /// Build the state from parts. `node_url` is the node's origin
    /// (`http://127.0.0.1:18787`); `node_token` is forwarded as a bearer
    /// token on hub → node calls, falling back to the caller's own
    /// `Authorization` header when unset.
    pub fn new(
        node: Arc<dyn EntityApi>,
        node_url: impl Into<Arc<str>>,
        node_token: Option<Arc<str>>,
    ) -> Self {
        let sse_agent = ureq::Agent::new_with_config(
            ureq::Agent::config_builder()
                .http_status_as_error(false)
                .timeout_global(None)
                .timeout_connect(Some(std::time::Duration::from_secs(10)))
                .timeout_recv_body(None)
                .build(),
        );
        let http_agent = ureq::Agent::new_with_config(
            ureq::Agent::config_builder()
                .http_status_as_error(false)
                .timeout_global(Some(std::time::Duration::from_secs(60)))
                .build(),
        );
        Self {
            node,
            node_url: node_url.into(),
            node_token,
            entity_router: None,
            push: None,
            hub_token: None,
            sse_agent,
            http_agent,
        }
    }

    /// Route an entity-scoped request: the owning node's url+token when
    /// a [`entity_router`](Self::entity_router) resolves, else the
    /// default upstream.
    pub(crate) fn entity_upstream(&self, id: &str) -> (String, Option<String>) {
        self.entity_router
            .as_ref()
            .and_then(|r| r(id))
            .unwrap_or_else(|| {
                (
                    self.node_url.to_string(),
                    self.node_token.as_ref().map(ToString::to_string),
                )
            })
    }

    /// Agent configured for open-ended SSE reads.
    pub(crate) fn sse_agent(&self) -> ureq::Agent {
        self.sse_agent.clone()
    }

    /// Agent configured for bounded JSON calls.
    pub(crate) fn json_agent(&self) -> ureq::Agent {
        self.http_agent.clone()
    }
}

/// Runtime configuration, all `PEREGRIN_*` env.
#[derive(Clone, Debug)]
pub struct HubConfig {
    /// `PEREGRIN_HUB_HOST` — default `127.0.0.1`.
    pub host: String,
    /// `PEREGRIN_HUB_PORT` — default `13000`.
    pub port: u16,
    /// `PEREGRIN_NODE_URL` — default `http://127.0.0.1:18787`.
    pub node_url: String,
    /// `PEREGRIN_NODE_TOKEN` — bearer token forwarded hub → node.
    pub node_token: Option<String>,
    /// `PEREGRIN_HUB_TOKEN` — client→hub auth; required on non-loopback
    /// binds, honored as bearer header or `?token=` query.
    pub hub_token: Option<String>,
}

impl Default for HubConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".into(),
            port: 13000,
            node_url: "http://127.0.0.1:18787".into(),
            node_token: None,
            hub_token: None,
        }
    }
}

/// Loopback binds may serve tokenless; anything else requires
/// `PEREGRIN_HUB_TOKEN` (mirrors `peregrin_http::env::is_loopback_host`).
fn is_loopback_host(host: &str) -> bool {
    host == "localhost" || host == "::1" || host == "[::1]" || host.starts_with("127.")
}

impl HubConfig {
    /// Parse from env. `Err` on a malformed port/address or a
    /// non-loopback bind without `PEREGRIN_HUB_TOKEN` — fail fast.
    pub fn from_env() -> Result<Self, String> {
        Self::from_lookup(|k| std::env::var(k).ok())
    }

    /// Test seam — parse from a key/value map instead of `std::env`.
    pub fn from_map(vars: &HashMap<String, String>) -> Result<Self, String> {
        Self::from_lookup(|key| vars.get(key).cloned())
    }

    /// Shared parser behind [`from_env`](Self::from_env).
    pub fn from_lookup(get: impl Fn(&str) -> Option<String>) -> Result<Self, String> {
        let mut cfg = Self::default();
        if let Some(v) = get("PEREGRIN_HUB_HOST") {
            cfg.host = v;
        }
        if let Some(v) = get("PEREGRIN_HUB_PORT").or_else(|| get("PEREGRIN_PORT")) {
            cfg.port = v.parse::<u16>().map_err(|_| {
                format!("PEREGRIN_HUB_PORT/PEREGRIN_PORT must be a port number, got {v:?}")
            })?;
        }
        if let Some(v) = get("PEREGRIN_NODE_URL") {
            cfg.node_url = v.trim_end_matches('/').to_string();
        }
        cfg.node_token = get("PEREGRIN_NODE_TOKEN").filter(|t| !t.is_empty());
        cfg.hub_token = get("PEREGRIN_HUB_TOKEN").filter(|t| !t.is_empty());
        // The node's bind guard, applied to the client surface: a
        // wildcard/LAN bind with no hub token would expose the API.
        if !is_loopback_host(&cfg.host) && cfg.hub_token.is_none() {
            return Err(format!(
                "peregrin-hub refuses to bind {} without PEREGRIN_HUB_TOKEN. Set PEREGRIN_HUB_TOKEN or bind a loopback address (PEREGRIN_HUB_HOST=127.0.0.1).",
                cfg.host
            ));
        }
        Ok(cfg)
    }

    pub fn socket_addr(&self) -> Result<SocketAddr, String> {
        format!("{}:{}", self.host, self.port)
            .parse()
            .map_err(|_| format!("invalid hub address {}:{}", self.host, self.port))
    }
}

/// Build the axum router. `HubConfig` → `HubState` happens in
/// [`hub_state`]; tests construct `HubState` directly with a stub
/// `NodeApi`.
pub fn router(state: HubState) -> Router {
    Router::new()
        // SSE bridges — must be registered before the `/api/*` wildcard.
        .route("/api/events", get(proxy::sse_events))
        .route("/api/sessions/{id}/stream", get(proxy::sse_session_stream))
        // Hub-owned push — intercepted before the passthrough so every
        // node notifies through one subscription store.
        .route("/api/push/vapid", any(push_routes::vapid))
        .route("/api/push/subscribe", any(push_routes::subscribe))
        .route("/api/{*rest}", any(proxy::passthrough))
        // Hard ceiling for *every* request body. Sized at the
        // passthrough cap so legit node-bound bodies never hit it.
        .layer(tower_http::limit::RequestBodyLimitLayer::new(
            proxy::MAX_PROXY_BODY,
        ))
        // Client gate — a no-op until `hub_token` is set.
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::auth_gate,
        ))
        // OTEL server span — outermost: client→hub requests get an
        // `http_request` span the proxy's `traceparent` injection links
        // hub→node calls into. Inert when export is off.
        .layer(middleware::from_fn(trace_layer))
        .with_state(state)
}

/// OTEL server span — extracts any inbound `traceparent` (federated
/// callers), records the response status onto the span.
async fn trace_layer(req: Request<Body>, next: Next) -> Response {
    let span = peregrin_telemetry::server_span(req.method(), req.uri().path(), req.headers());
    let for_status = span.clone();
    async move {
        let res = next.run(req).await;
        peregrin_telemetry::record_response(&for_status, res.status().as_u16());
        res
    }
    .instrument(span)
    .await
}

/// HubState for a running hub: the sync engine + [`SyncEntityApi`].
/// `PEREGRIN_NODES` (`id=url[@token];…`) wins; `PEREGRIN_NODE_URL`/`PEREGRIN_NODE_TOKEN`
/// is the single-node shorthand. Nodes listed without a token resolve
/// one from the [`credentials`] store (OS keyring → file fallback —
/// that store is also where the pair flow persists minted tokens), and
/// an env-provided token seeds an empty store slot so the plaintext
/// can leave the env. The projection + outbox live under
/// `$PEREGRIN_HOME/hub/` so they survive restarts.
///
/// # Errors
/// `String` on a missing/empty node list or a projection/outbox open failure.
pub fn hub_state(config: &HubConfig) -> Result<HubState, String> {
    hub_state_from(|k| std::env::var_os(k), config)
}

/// [`hub_state`] with the environment injected — the test seam that
/// drives every env arm (`PEREGRIN_HOME`/`HOME`/`PEREGRIN_NODES`)
/// without touching the process env.
#[doc(hidden)]
pub fn hub_state_from(
    get: impl Fn(&str) -> Option<std::ffi::OsString>,
    config: &HubConfig,
) -> Result<HubState, String> {
    let home = get("PEREGRIN_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| get("HOME").map(|h| std::path::PathBuf::from(h).join(".local/share/peregrin")))
        .unwrap_or_else(|| std::path::PathBuf::from(".peregrin"))
        .join("hub");
    std::fs::create_dir_all(&home).map_err(|e| format!("create {}: {e}", home.display()))?;
    let creds = credentials::open(&home);
    let mut nodes = get("PEREGRIN_NODES")
        .and_then(|v| v.into_string().ok())
        .filter(|v| !v.trim().is_empty())
        .map(|raw| sync_api::parse_nodes(&raw))
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| {
            vec![sync_api::HubNode {
                id: "node".to_string(),
                url: config.node_url.clone(),
                token: config.node_token.clone(),
            }]
        });
    // Env tokens are explicit config — they win, and seed an empty
    // store slot so the plaintext can leave the env. Tokenless nodes
    // resolve a stored credential.
    let (stored, seeded) = sync_api::resolve_node_tokens(&mut nodes, creds.as_ref());
    // Hoisted — `tracing` fields evaluate only when a subscriber is
    // attached, which coverage runs don't guarantee.
    let backend = creds.backend();
    tracing::info!(backend, stored, seeded, "node credential store");
    let projection = peregrin_sync::ProjectionStore::open(&home.join("projection.db"))
        .map_err(|e| format!("projection open: {e}"))?;
    // Rows from node ids no longer configured (renamed, removed, or the
    // legacy `node` shorthand) have no sync loop — purge them once at
    // boot so dead entities stop surfacing in the index.
    let keep: Vec<String> = nodes.iter().map(|n| n.id.clone()).collect();
    if let Err(e) = projection.purge_unregistered(&keep) {
        tracing::warn!("purge_unregistered failed: {e}");
    }
    let outbox =
        peregrin_outbox::Outbox::open(&home.join("outbox.db")).map_err(|e| e.to_string())?;
    let targets = nodes.iter().map(|n| {
        let node = peregrin_sync::NodeRef {
            id: n.id.clone(),
            url: n.url.clone(),
            label: n.id.clone(),
        };
        match &n.token {
            Some(t) => peregrin_sync::engine::NodeTarget::with_token(node, t.clone()),
            None => peregrin_sync::engine::NodeTarget::new(node),
        }
    });
    let push = Arc::new(peregrin_push::PushStore::open(sepia_meta::MetaStore::open(
        &home.join("push.json"),
    )));
    let options = peregrin_sync::engine::SyncOptions {
        on_event: Some(feed_hook(projection.clone(), Arc::clone(&push))),
        ..peregrin_sync::engine::SyncOptions::default()
    };
    let engine = peregrin_sync::SyncEngine::spawn(projection, outbox, targets, options);
    // `unwrap_or_else` above guarantees at least the single-node row.
    let upstream = nodes.first().cloned().unwrap_or_default();
    let api = Arc::new(sync_api::SyncEntityApi::new(engine, nodes, creds));
    let mut state = HubState::new(
        Arc::clone(&api) as Arc<dyn EntityApi>,
        upstream.url.as_str(),
        upstream.token.clone().map(Into::into),
    );
    state.push = Some(push);
    state.hub_token = config.hub_token.clone().map(Into::into);
    let router_api = Arc::clone(&api);
    state.entity_router = Some(Arc::new(move |id| router_api.node_url_for(id, None)));
    Ok(state)
}

/// The hub's push-notification feed hook — entity diffs carrying
/// `runFinished`/`permissionRequested` patch flags become Web Push
/// sends. Extracted so tests can drive the body directly (the live
/// hook only fires on real node feeds).
#[doc(hidden)]
pub fn feed_hook(
    projection: peregrin_sync::ProjectionStore,
    push: Arc<peregrin_push::PushStore>,
) -> peregrin_sync::engine::FeedHook {
    Arc::new(
        move |node_id: &str, event: &peregrin_sync::client::FeedEvent| {
            let peregrin_sync::client::FeedEvent::Diff {
                kind, id, patch, ..
            } = event
            else {
                return;
            };
            if kind != "session" {
                return;
            }
            // INTERIM: push rules read `indexDoc.title` — a
            // `sepia-session` field. Entity-level notify rules are
            // the integration's job (PL-004 phase 4).
            let title = projection.entity(node_id, id).ok().flatten().map_or_else(
                || id.clone(),
                |r| {
                    let doc = r.raw.get("indexDoc").unwrap_or(&r.raw);
                    doc.get("title")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or(id)
                        .to_string()
                },
            );
            let url = format!("/?session={id}");
            if patch.get("runFinished") == Some(&serde_json::Value::Bool(true)) {
                push.send(
                    peregrin_push::Kind::Done,
                    "Session finished",
                    &format!("{title} finished its run."),
                    &url,
                );
            }
            if patch.get("permissionRequested") == Some(&serde_json::Value::Bool(true)) {
                push.send(
                    peregrin_push::Kind::Permission,
                    "Approval needed",
                    &format!("{title} is waiting for you."),
                    &url,
                );
            }
        },
    )
}
