//! peregrin-http — the composition adapter for the peregrin node API.
//! The generic substrate surface (`/api/entities*`, `/api/events`,
//! `/api/node`, `/api/pair`, `/api/auth/*`, `/api/push/*`, `/api/kv*`,
//! `/api/fs`, `/api/user`, `/api/health`) is `hypha-http`'s; the
//! session surface (`/api/sessions*`, agents, projects, config) is
//! `sepia-http`'s since PL-027 — this crate keeps [`AppState`] (the
//! composition-facing state: [`SessionState`] plus the orchestration
//! seams — kind registry, pair limiter, node path, event ingest),
//! [`Env`], the ingest route, the push backends, and [`compose`],
//! which merges the three routers under one listener.
//!
//! The `kind="sepia-session"` entity plane is sepia's own
//! [`sepia_peregrin::SepiaEntityPlane`], registered by [`compose`]
//! over [`SessionPlaneBackend`] — the node [`SessionPlane`] plus the
//! `sepia-http` side-channels (meta overlay, feed, watches), so
//! entity ops keep the `/api/sessions*` semantics without re-entering
//! the routes.

pub mod env;
pub mod ingest;
pub mod node;
pub mod push;
pub mod routes;

use std::ops::Deref;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::middleware;
use axum::response::Response;
use axum::routing::any;
use axum::{Router, serve as axum_serve};
use futures::future::BoxFuture;
use hypha_http::entity_plane::KindRegistry;
use peregrin_session::SessionPlane;
use sepia_http::SessionState;
use sepia_meta::MetaStore;
use sepia_session::NodeIdentity as SepiaNodeIdentity;

// The session surface moved to `sepia-http` (PL-027) — re-exported
// so `peregrin_http::{EventFeed, SessionPlaneBackend, routes::*, …}`
// keeps resolving for embedders and tests.
pub use env::{Env, EnvError};
pub use feed::{EventFeed, HeldWatches, InstrumentedMeta, LiveListeners};
pub use hypha_http::entity_plane;
pub use hypha_http::entity_plane::{EntityContext, EntityPlane};
pub use hypha_http::pair;
pub use hypha_http::{Pairing, PairingStore, Quota, RateLimiter};
pub use ingest::{DeliverError, EventDeliver, EventIngest, IngestDelivery, IngestOutcome};
pub use node::{
    NodeIdentity, PEREGRIN_VERSION, PROTOCOL_VERSION, load_node_identity, save_node_identity,
};
pub use routes::sessions::{ConvertSession, ImportSession, ImportTarget};
pub use sepia_http::kv;
pub use sepia_http::session_backend::SessionPlaneBackend;
pub use sepia_http::{auth, feed, session_backend};

/// Shared state threaded through every handler — the composition
/// root's [`SessionState`] (session plane + meta/feed/kv/pairing/push
/// side-channels) plus the orchestration seams this crate owns: the
/// extra kind planes [`compose`] merges behind `/api/entities*`, the
/// `/api/pair` throttle, the `PATCH /api/node` persistence path, and
/// the `POST /api/events/inject` campaign fan-out.
#[derive(Clone)]
pub struct AppState {
    /// The session surface's state — derefs so `state.plane`,
    /// `state.feed`, `state.meta`, … read unchanged.
    pub session: SessionState,
    /// `kind` → [`EntityPlane`] staging registry — the *extra* planes
    /// [`compose`] merges into the substrate's [`hypha_http::KindRegistry`]
    /// behind `/api/entities*` (`sepia-session` is added by the
    /// composer itself, always first so it stays the default plane).
    pub kinds: KindRegistry,
    /// Per-client-IP throttle on `POST /api/pair` — the one route that
    /// mints a bearer credential with no prior auth, so it's the brute-
    /// force surface. Keyed by `ConnectInfo` socket IP (never
    /// `X-Forwarded-For` — the node doesn't run behind a trusted proxy).
    pub pair_limit: RateLimiter,
    /// Where the identity persists (`PEREGRIN_NODE`); `None` keeps a
    /// `PATCH /api/node` rename in-memory only.
    pub node_path: Option<PathBuf>,
    /// `POST /api/events/inject` backend — the campaign plane's
    /// live-run fan-out plus the idempotency dedup ring
    /// ([`crate::ingest`]). `None` → the route answers 501 (a node
    /// without a campaign plane has nothing to feed).
    pub event_ingest: Option<EventIngest>,
}

impl Deref for AppState {
    type Target = SessionState;

    fn deref(&self) -> &SessionState {
        &self.session
    }
}

impl AppState {
    /// Minimal state — tokenless, no meta store, ephemeral node id.
    pub fn new(plane: Arc<dyn SessionPlane>) -> Self {
        Self {
            session: SessionState::new(plane),
            kinds: KindRegistry::new(),
            pair_limit: RateLimiter::pair(
                env::DEFAULT_PAIR_LIMIT_PER_MIN,
                env::DEFAULT_PAIR_LIMIT_PER_DAY,
            ),
            node_path: None,
            event_ingest: None,
        }
    }

    /// Assemble state from a parsed [`Env`] — the `serve.ts` wiring:
    /// meta store at `PEREGRIN_META`, node identity at `PEREGRIN_NODE`,
    /// pairing at `<home>/pair-code` + `<home>/tokens.json`.
    pub fn from_env(env: &Env, plane: Arc<dyn SessionPlane>) -> Self {
        let feed = EventFeed::new();
        let meta = MetaStore::open(&env.meta_path);
        // P-006 — projects/config/push-secrets live in the node-local
        // KV (`<home>/kv.json`); sessions stay in meta.json.
        // `migrate_meta` performs the one-shot import (marker-guarded,
        // reading the legacy sections off meta.json directly —
        // sepia-meta is sessions-only since phase 3) and declares the
        // internal namespaces before anything serves.
        let kv: Arc<dyn hypha_kv::KvStore> =
            Arc::new(hypha_kv_file::JsonKvStore::open(&env.kv_path));
        kv::migrate_meta(&env.meta_path, &kv);
        let identity = load_node_identity(&env.node_path, &env.node_name);
        let mut session = SessionState::new(plane);
        // The meta instrumentation and the state share ONE feed
        // broadcast — assign before `with_meta` captures it.
        session.feed = feed.clone();
        Self {
            session: session
                .with_meta(meta)
                .with_kv(Some(kv.clone()))
                .with_node(SepiaNodeIdentity {
                    id: identity.id.clone(),
                    name: identity.name.clone(),
                })
                // `sepia_`-prefixed issued tokens — the wire contract
                // `/api/pair` callers and the auth gate both expect.
                .with_pairing(
                    Pairing::open(env.pair_code_path.clone(), env.tokens_path.clone())
                        .with_token_prefix("sepia_"),
                )
                .with_token(env.token.clone())
                .with_origins(env.origins.clone())
                .with_keep_alive(env.sse_keep_alive)
                .with_held_watch(env.held_watch)
                // `PEREGRIN_PUSH_BACKEND` picks the persistence: `kv`
                // (default — the `push` ns of kv.json) or `cuora` (the
                // broker, kv-mirrored during the migration).
                .with_push(Some(Arc::new(hypha_push::PushStore::open(
                    push::NodePushBackend::for_kind(kv, env.push_backend),
                )))),
            kinds: KindRegistry::new(),
            pair_limit: RateLimiter::pair(env.pair_limit_per_min, env.pair_limit_per_day),
            node_path: Some(env.node_path.clone()),
            // The campaign plane's fan-out is wired by the
            // composition root (`peregrin-node::app_state`) — `Env`
            // alone doesn't know it.
            event_ingest: None,
        }
    }

    /// Register an extra entity kind plane — staged on
    /// [`Self::kinds`] and merged into the substrate registry by
    /// [`compose`] (which always puts `sepia-session` first).
    #[must_use]
    pub fn with_kind(mut self, plane: Arc<dyn EntityPlane>) -> Self {
        self.kinds.register(plane);
        self
    }

    /// Override the `/api/pair` throttle (tests inject tiny quotas).
    #[must_use]
    pub fn with_pair_limit(mut self, pair_limit: RateLimiter) -> Self {
        self.pair_limit = pair_limit;
        self
    }

    /// Persist `PATCH /api/node` renames to `path` (`PEREGRIN_NODE`).
    #[must_use]
    pub fn with_node_path(mut self, path: PathBuf) -> Self {
        self.node_path = Some(path);
        self
    }

    /// Wire the `POST /api/events/inject` backend — the campaign
    /// plane's live-run fan-out ([`crate::ingest::EventIngest`]).
    #[must_use]
    pub fn with_event_ingest(mut self, ingest: EventIngest) -> Self {
        self.event_ingest = Some(ingest);
        self
    }

    #[must_use]
    pub fn with_token(mut self, token: Option<String>) -> Self {
        self.session = self.session.clone().with_token(token);
        self
    }

    #[must_use]
    pub fn with_meta(mut self, meta: MetaStore) -> Self {
        self.session = self.session.clone().with_meta(meta);
        self
    }

    /// Install the node-local KV store (P-006) — `None` leaves the
    /// `/api/projects*` + `/api/config*` compat surfaces answering 501.
    #[must_use]
    pub fn with_kv(mut self, kv: Option<Arc<dyn hypha_kv::KvStore>>) -> Self {
        self.session = self.session.clone().with_kv(kv);
        self
    }

    #[must_use]
    pub fn with_pairing(mut self, pairing: Pairing) -> Self {
        self.session = self.session.clone().with_pairing(pairing);
        self
    }

    #[must_use]
    pub fn with_node(mut self, node: NodeIdentity) -> Self {
        self.session = self.session.clone().with_node(SepiaNodeIdentity {
            id: node.id,
            name: node.name,
        });
        self
    }

    /// Snapshot of the node identity — one lock, one clone.
    pub fn node_identity(&self) -> NodeIdentity {
        let id = self.session.node_identity();
        NodeIdentity {
            id: id.id,
            name: id.name,
            version: PEREGRIN_VERSION.to_string(),
        }
    }

    #[must_use]
    pub fn with_origins(mut self, origins: Vec<String>) -> Self {
        self.session = self.session.clone().with_origins(origins);
        self
    }

    #[must_use]
    pub fn with_keep_alive(mut self, keep_alive: Duration) -> Self {
        self.session = self.session.clone().with_keep_alive(keep_alive);
        self
    }

    #[must_use]
    pub fn with_held_watch(mut self, interval: Duration) -> Self {
        self.session = self.session.clone().with_held_watch(interval);
        self
    }

    #[must_use]
    pub fn with_import_session(mut self, f: ImportSession) -> Self {
        self.session = self.session.clone().with_import_session(f);
        self
    }

    /// Override the push service (tests wire a stub store).
    #[must_use]
    pub fn with_push(mut self, push: Option<Arc<hypha_push::PushStore>>) -> Self {
        self.session = self.session.clone().with_push(push);
        self
    }

    #[must_use]
    pub fn with_convert(mut self, f: ConvertSession) -> Self {
        self.session = self.session.clone().with_convert(f);
        self
    }
}

/// The `capabilities` list `GET /api/node` reports — the session
/// surface's wire contract, delegated to `sepia-http`.
#[must_use]
pub fn node_capabilities(state: &AppState) -> Vec<String> {
    sepia_http::node_capabilities(&state.session)
}

/// `GET /api/health` probe — the session-plane liveness check: a
/// bounded `list_sessions` decides `{ok,db}` and the 503. Installed on
/// the substrate state as `HyphaState::health_probe` at composition.
fn health_probe(plane: Arc<dyn SessionPlane>) -> hypha_http::HealthProbe {
    Arc::new(move || {
        let plane = Arc::clone(&plane);
        Box::pin(async move {
            /// `GET /api/health` — deliberately ahead of the auth gate.
            /// The TS health check is a 1.5s-bounded session list.
            const HEALTH_TIMEOUT: Duration = Duration::from_millis(1_500);
            let healthy =
                match tokio::time::timeout(HEALTH_TIMEOUT, plane.list_sessions(false)).await {
                    Ok(Ok(_)) => true,
                    Ok(Err(_)) | Err(_) => false,
                };
            if healthy {
                routes::json_response(
                    serde_json::json!({ "ok": true, "db": true }),
                    axum::http::StatusCode::OK,
                )
            } else {
                routes::json_response(
                    serde_json::json!({ "ok": false, "db": false }),
                    axum::http::StatusCode::SERVICE_UNAVAILABLE,
                )
            }
        }) as BoxFuture<'static, Response>
    })
}

/// The live agent-id roster `GET /api/node` reports — `plane.list_agents`
/// mapped to ids, installed as the substrate's `agent_lister`.
fn agent_lister(plane: &Arc<dyn SessionPlane>) -> hypha_http::AgentLister {
    let plane = Arc::clone(plane);
    Arc::new(move || {
        let plane = Arc::clone(&plane);
        Box::pin(async move {
            plane
                .list_agents()
                .await
                .into_iter()
                .map(|a| a.id)
                .collect()
        }) as BoxFuture<'static, Vec<String>>
    })
}

/// Align the substrate state with the session state at composition —
/// the singletons both sides must share so the wire behaves as one
/// node did before the split:
///
/// - `feed` — one broadcast; session/meta/project emissions reach
///   `/api/events` (and `subscriber_count` gates the held watch).
/// - `pairing` — one `Pairing` instance; credentials `/api/pair`
///   mints are visible to the session routes' auth gate.
/// - `push` — one `PushStore`; `/api/push/*` subscriptions are what
///   the live listeners notify.
/// - `entity_host` — `Arc<AppState>` for any foreign plane that still
///   downcasts its host.
/// - `kinds` — rebuilt with `sepia-session` FIRST (the default plane
///   for bare ops/kind-less creates), then the caller's pre-registered
///   planes, then [`AppState::kinds`] extras. The session kind is
///   sepia's own [`sepia_peregrin::SepiaEntityPlane`] over
///   [`SessionPlaneBackend`] — the node `SessionPlane` plus the
///   `/api/sessions*` side-channels (meta overlay, feed, watches).
/// - `auth_cookie`/`health_probe` — the Peregrin contract
///   (`sepia_token`, the session-plane liveness probe).
fn align_substrate(state: &AppState, hypha: &mut hypha_http::HyphaState) {
    hypha.feed = state.feed.substrate();
    if let Some(pairing) = &state.pairing {
        // Peregrin's credential prefix is `sepia_` — force it here so
        // every `Pairing` source (`from_env`, `with_pairing`, test
        // fixtures) mints the wire contract, whatever prefix the
        // `hypha_http::Pairing` default carried.
        hypha.pairing = Some(pairing.clone().with_token_prefix("sepia_"));
    }
    if let Some(push) = &state.push {
        hypha.push = Some(Arc::clone(push));
    }
    // One KV store for both routers — the session state's instance
    // wins over the substrate's own `HYPHA_KV` open (same file, but a
    // single `JsonKvStore` keeps one in-memory map; two instances
    // would lose each other's writes). The hook maps `project`-ns
    // writes onto `project` feed events + the session-meta cascade;
    // every other namespace stays silent.
    if state.kv.is_some() {
        hypha.kv.clone_from(&state.kv);
    }
    hypha.kv_hook = Some(kv::write_hook(state.meta.clone()));
    hypha.entity_host = Some(Arc::new(state.clone()));
    hypha.auth_cookie = Arc::from("sepia_token");
    hypha.health_probe = Some(health_probe(Arc::clone(&state.plane)));

    hypha.kinds = merged_kinds(state, hypha);
}

/// The kind registry [`compose`] installs on the substrate state —
/// `sepia-session` FIRST (the default plane for bare ops/kind-less
/// creates/imports), then the substrate state's own planes, then
/// [`AppState::kinds`] extras. Public so shutdown paths that need the
/// same live planes the router serves (the ephemeral snapshot flush)
/// don't re-derive the merge order.
pub fn merged_kinds(state: &AppState, hypha: &hypha_http::HyphaState) -> KindRegistry {
    let mut kinds = KindRegistry::new();
    kinds.register(Arc::new(sepia_peregrin::SepiaEntityPlane::new(Arc::new(
        SessionPlaneBackend::new(&state.session),
    ))));
    for plane in hypha.kinds.planes() {
        kinds.register(Arc::clone(plane));
    }
    for plane in state.kinds.planes() {
        kinds.register(Arc::clone(plane));
    }
    kinds
}

/// The `POST /api/events/inject` route — peregrin's campaign-plane
/// ingest, kept out of `sepia-http` (it feeds `kind="campaign"`
/// entities, not sessions). Mounted with the same middleware stack
/// the session router applies (auth gate, log, CORS, trace) so the
/// wire contract is unchanged.
fn ingest_app(state: AppState) -> Router {
    Router::new()
        .route("/api/events/inject", any(routes::events::inject))
        .layer(middleware::from_fn_with_state(
            state.session.clone(),
            sepia_http::auth::auth_gate,
        ))
        .layer(middleware::from_fn(sepia_http::log_layer))
        .layer(middleware::from_fn_with_state(
            state.session.clone(),
            sepia_http::cors_layer,
        ))
        .layer(middleware::from_fn(sepia_http::trace_layer))
        .with_state(state)
}

/// Compose the full node surface — the substrate routes
/// (`hypha_http::app_routes`), the session routes
/// (`sepia_http::app_routes`), and the campaign ingest route merged
/// under ONE listener and ONE 404 fallback (the substrate router's).
/// `hypha` is the caller-built substrate state (env-mapped in
/// `serve`); `align_substrate` then wires the shared singletons.
///
/// The returned router is `Router<()>` — serve it with
/// `into_make_service_with_connect_info::<SocketAddr>()`.
pub fn compose(state: AppState, mut hypha: hypha_http::HyphaState) -> Router {
    align_substrate(&state, &mut hypha);
    let substrate = hypha_http::app_routes(&hypha).with_state(hypha);
    let session = sepia_http::app_routes(&state.session).with_state(state.session.clone());
    session
        .merge(substrate)
        .merge(ingest_app(state))
        // Neither `app_routes` carries a fallback — the composer owns
        // the single 404, same `{"error":"Not found"}` shape the
        // session surface always answered.
        .fallback(async || sepia_http::routes::not_found())
}

/// A substrate state derived from `AppState`'s own knobs — token,
/// origins, keep-alive, pair limiter, node identity, push, pairing,
/// live agents, capabilities. The test/back-compat path behind
/// [`app`]; `serve` builds its `HyphaState` from `PEREGRIN_*` env
/// instead (same fields, explicit sources).
fn hypha_for(state: &AppState) -> hypha_http::HyphaState {
    let identity = state.node_identity();
    let mut hypha = hypha_http::HyphaState::new()
        .with_token(state.token.clone())
        .with_origins(state.origins.clone())
        .with_keep_alive(state.keep_alive)
        .with_agent_lister(agent_lister(&state.plane))
        .with_node_capabilities(node_capabilities(state))
        .with_push(state.push.clone());
    hypha.pair_limit = state.pair_limit.clone();
    hypha.node = Arc::new(Mutex::new(hypha_http::NodeIdentity {
        id: identity.id,
        name: identity.name,
        version: identity.version,
    }));
    hypha.node_path.clone_from(&state.node_path);
    hypha
}

/// The env-mapped substrate state — `PEREGRIN_*` values translated
/// into `HyphaState` via [`Env::to_hypha`], plus the session-plane
/// seams (live agents, node capabilities). [`compose`] still unifies
/// the shared singletons (feed, pairing, push, kinds, cookie) via
/// `align_substrate`, so the `HyphaState::from_env` pairing/push
/// instances opened on the same paths are superseded, not duplicated.
#[must_use]
pub fn hypha_state(env: &Env, state: &AppState) -> hypha_http::HyphaState {
    let mut capabilities = node_capabilities(state);
    // `PEREGRIN_EPHEMERAL`/`--ephemeral` advertises the disposable
    // lifecycle — a hub reading `GET /api/node` learns the node is a
    // CI runner and reaps its projected state once it stays down.
    if env.ephemeral {
        capabilities.push("ephemeral".into());
    }
    // Fleet runner labels (`PEREGRIN_RUNNER_LABELS`) ride the same
    // capability list — campaign `runs_on:` matchers and the hub's
    // sync projection read them off `GET /api/node`.
    capabilities.extend(env.runner_labels.iter().cloned());
    hypha_http::HyphaState::from_env(&env.to_hypha())
        .with_agent_lister(agent_lister(&state.plane))
        .with_node_capabilities(capabilities)
}

/// Build the composed axum router with a substrate state derived from
/// `AppState` — every route the node serves, for tests and embedders
/// that don't need the env-mapped variant. `serve` callers wanting
/// the explicit `PEREGRIN_*` mapping use [`compose`] with a
/// `HyphaState::from_env` instead.
pub fn app(state: AppState) -> Router {
    let hypha = hypha_for(&state);
    compose(state, hypha)
}

/// `startServer` — bind `env.host:env.port` and serve the app. The bind
/// guard already ran in [`Env::parse`]; callers wanting graceful
/// shutdown own the listener themselves.
///
/// # Errors
/// `std::io::Error` on bind/serve failure.
pub async fn serve(env: &Env, state: AppState) -> std::io::Result<()> {
    let listener = tokio::net::TcpListener::bind((env.host.as_str(), env.port)).await?;
    tracing::info!("peregrin-server listening on {}:{}", env.host, env.port);
    // One listener, one router: session routes + the hypha substrate
    // over the env-mapped state. `ConnectInfo` feeds the /api/pair
    // limiter its client-IP key.
    let hypha = hypha_state(env, &state);
    let serving = axum_serve(
        listener,
        compose(state, hypha).into_make_service_with_connect_info::<std::net::SocketAddr>(),
    );
    #[cfg(not(coverage))]
    {
        serving.await
    }
    #[cfg(coverage)]
    {
        // The accept loop never returns; under coverage a ready arm
        // lets `serve` complete so the function tail counts.
        tokio::select! {
            r = serving => r,
            () = std::future::ready(()) => Ok(()),
        }
    }
}
