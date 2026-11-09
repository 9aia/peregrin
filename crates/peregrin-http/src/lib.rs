//! peregrin-http — the axum driving adapter for the peregrin node API.
//! The generic substrate surface (`/api/entities*`, `/api/events`,
//! `/api/node`, `/api/pair`, `/api/auth/*`, `/api/push/*`, `/api/fs`,
//! `/api/user`, `/api/health`) is `hypha-http`'s; this crate keeps the
//! session plane — `/api/sessions*`, agents, projects, config, client
//! keypair — plus the [`AppState`] both compose over. [`compose`]
//! merges the two routers under one listener and [`app`] builds the
//! composed surface with a substrate state derived from `AppState`
//! (the test/back-compat path); `peregrin-node`'s `serve` passes an
//! env-mapped [`hypha_http::HyphaState`] instead.
//!
//! The `kind="sepia-session"` entity plane is sepia's own
//! [`sepia_peregrin::SepiaEntityPlane`], registered by [`compose`]
//! over [`SessionPlaneBackend`] — the node [`SessionPlane`] plus this
//! crate's meta/feed/watch side-channels, so entity ops keep the
//! `/api/sessions*` semantics without re-entering the routes.

pub mod auth;
pub mod env;
pub mod feed;
pub mod node;
pub mod push;
pub mod routes;
pub mod session_backend;

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::{HeaderValue, Method, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::any;
use axum::{Router, serve as axum_serve};
use peregrin_session::SessionPlane;
use sepia_meta::MetaStore;
use tracing::Instrument;

// The entity-plane port + pairing backend moved to `hypha-http`
// (PL-005 P4b) — re-exported so `peregrin_http::{EntityPlane,
// Pairing, …}` and `peregrin_http::pair::*` keep resolving.
pub use env::{Env, EnvError};
pub use feed::{EventFeed, HeldWatches, InstrumentedMeta, LiveListeners};
pub use hypha_http::entity_plane;
pub use hypha_http::entity_plane::{EntityContext, EntityPlane, KindRegistry};
pub use hypha_http::pair;
pub use hypha_http::{Quota, RateLimiter};
pub use node::{
    NodeIdentity, PEREGRIN_VERSION, PROTOCOL_VERSION, load_node_identity, save_node_identity,
};
pub use pair::{Pairing, PairingStore};
pub use routes::sessions::{ConvertSession, ImportSession, ImportTarget};
pub use session_backend::SessionPlaneBackend;

/// Shared state threaded through every handler — the `createApp`
/// options object, minus the seams this crate doesn't port (transfer,
/// SSH tunnels, servers registry, push, telemetry, UI assets, AG-UI).
#[derive(Clone)]
pub struct AppState {
    pub plane: Arc<dyn SessionPlane>,
    /// `kind` → [`EntityPlane`] staging registry — the *extra* planes
    /// [`compose`] merges into the substrate's [`hypha_http::KindRegistry`]
    /// behind `/api/entities*` (`sepia-session` is added by the
    /// composer itself, always first so it stays the default plane).
    pub kinds: KindRegistry,
    /// The instrumented meta overlay — `None` → meta routes answer 501.
    pub meta: Option<InstrumentedMeta>,
    /// Process-wide node event feed behind `GET /api/events`.
    pub feed: EventFeed,
    /// Identity reported by `GET /api/node`; `PATCH /api/node` renames
    /// it in place (a mutex because `AppState` clones per request).
    pub node: Arc<Mutex<NodeIdentity>>,
    /// Where the identity persists (`PEREGRIN_NODE`); `None` keeps a
    /// `PATCH /api/node` rename in-memory only.
    pub node_path: Option<PathBuf>,
    /// Pairing backend for `POST /api/pair` + issued-credential auth —
    /// the `hypha_http::Pairing` instance, shared with the substrate
    /// state at composition so credentials minted by `/api/pair` are
    /// visible to the session routes' auth gate.
    pub pairing: Option<Pairing>,
    /// Per-client-IP throttle on `POST /api/pair` — the one route that
    /// mints a bearer credential with no prior auth, so it's the brute-
    /// force surface. Keyed by `ConnectInfo` socket IP (never
    /// `X-Forwarded-For` — the node doesn't run behind a trusted proxy).
    pub pair_limit: RateLimiter,
    /// `PEREGRIN_TOKEN` — `None`/empty means bearer auth is off entirely.
    pub token: Option<String>,
    /// CORS allowlist — `*` echoes any Origin back (bearer auth, not
    /// CORS, is the gate).
    pub origins: Vec<String>,
    /// SSE keep-alive cadence; `0` disables.
    pub keep_alive: Duration,
    /// Held-session watch state (re-probe cadence inside).
    pub held: HeldWatches,
    /// Per-session live listeners emitting `busy` feed events.
    pub live: LiveListeners,
    /// Store-write seam for `POST /api/sessions/import`.
    pub import_session: Option<ImportSession>,
    /// Store-write seam for `POST /api/sessions/:id/convert`.
    pub convert: Option<ConvertSession>,
    /// Web Push service — subscriptions persist in the meta overlay
    /// (`crate::push`'s `PushBackend for MetaStore`); shared with the
    /// substrate's `/api/push/*` at composition. `None` → 501s.
    pub push: Option<Arc<hypha_push::PushStore>>,
}

impl AppState {
    /// Minimal state — tokenless, no meta store, ephemeral node id.
    pub fn new(plane: Arc<dyn SessionPlane>) -> Self {
        Self {
            plane,
            kinds: KindRegistry::new(),
            meta: None,
            feed: EventFeed::new(),
            node: Arc::new(Mutex::new(NodeIdentity {
                id: format!("node_{}", &uuid::Uuid::new_v4().simple().to_string()[..16]),
                name: hostname_or_unknown(),
                version: PEREGRIN_VERSION.to_string(),
            })),
            node_path: None,
            pairing: None,
            pair_limit: RateLimiter::pair(
                env::DEFAULT_PAIR_LIMIT_PER_MIN,
                env::DEFAULT_PAIR_LIMIT_PER_DAY,
            ),
            token: None,
            origins: vec![
                "http://localhost:13000".to_string(),
                "http://127.0.0.1:13000".to_string(),
            ],
            keep_alive: Duration::from_millis(env::DEFAULT_SSE_KEEPALIVE_MS),
            held: HeldWatches::new(Duration::from_millis(env::DEFAULT_HELD_WATCH_MS)),
            live: LiveListeners::default(),
            import_session: None,
            convert: None,
            push: None,
        }
    }

    /// Assemble state from a parsed [`Env`] — the `serve.ts` wiring:
    /// meta store at `PEREGRIN_META`, node identity at `PEREGRIN_NODE`,
    /// pairing at `<home>/pair-code` + `<home>/tokens.json`.
    pub fn from_env(env: &Env, plane: Arc<dyn SessionPlane>) -> Self {
        let feed = EventFeed::new();
        let meta = MetaStore::open(&env.meta_path);
        Self {
            plane,
            kinds: KindRegistry::new(),
            meta: Some(InstrumentedMeta::new(meta, feed.clone())),
            feed,
            node: Arc::new(Mutex::new(load_node_identity(
                &env.node_path,
                &env.node_name,
            ))),
            node_path: Some(env.node_path.clone()),
            // `sepia_`-prefixed issued tokens — the wire contract
            // `/api/pair` callers and the auth gate both expect.
            pairing: Some(
                Pairing::open(env.pair_code_path.clone(), env.tokens_path.clone())
                    .with_token_prefix("sepia_"),
            ),
            pair_limit: RateLimiter::pair(env.pair_limit_per_min, env.pair_limit_per_day),
            token: env.token.clone(),
            origins: env.origins.clone(),
            keep_alive: env.sse_keep_alive,
            held: HeldWatches::new(env.held_watch),
            live: LiveListeners::default(),
            import_session: None,
            convert: None,
            push: Some(Arc::new(hypha_push::PushStore::open(
                crate::push::MetaPushBackend(MetaStore::open(&env.meta_path)),
            ))),
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

    #[must_use]
    pub fn with_token(mut self, token: Option<String>) -> Self {
        self.token = token;
        self
    }

    #[must_use]
    pub fn with_meta(mut self, meta: MetaStore) -> Self {
        self.meta = Some(InstrumentedMeta::new(meta, self.feed.clone()));
        self
    }

    #[must_use]
    pub fn with_pairing(mut self, pairing: Pairing) -> Self {
        self.pairing = Some(pairing);
        self
    }

    /// Override the `/api/pair` throttle (tests inject tiny quotas).
    #[must_use]
    pub fn with_pair_limit(mut self, pair_limit: RateLimiter) -> Self {
        self.pair_limit = pair_limit;
        self
    }

    #[must_use]
    pub fn with_node(mut self, node: NodeIdentity) -> Self {
        self.node = Arc::new(Mutex::new(node));
        self
    }

    /// Persist `PATCH /api/node` renames to `path` (`PEREGRIN_NODE`).
    #[must_use]
    pub fn with_node_path(mut self, path: PathBuf) -> Self {
        self.node_path = Some(path);
        self
    }

    /// Snapshot of the node identity — one lock, one clone.
    pub fn node_identity(&self) -> NodeIdentity {
        self.node
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    #[must_use]
    pub fn with_origins(mut self, origins: Vec<String>) -> Self {
        self.origins = origins;
        self
    }

    #[must_use]
    pub fn with_keep_alive(mut self, keep_alive: Duration) -> Self {
        self.keep_alive = keep_alive;
        self
    }

    #[must_use]
    pub fn with_held_watch(mut self, interval: Duration) -> Self {
        self.held = HeldWatches::new(interval);
        self
    }

    #[must_use]
    pub fn with_import_session(mut self, f: ImportSession) -> Self {
        self.import_session = Some(f);
        self
    }

    /// Override the push service (tests wire a stub store).
    #[must_use]
    pub fn with_push(mut self, push: Option<Arc<hypha_push::PushStore>>) -> Self {
        self.push = push;
        self
    }

    #[must_use]
    pub fn with_convert(mut self, f: ConvertSession) -> Self {
        self.convert = Some(f);
        self
    }
}

/// The `capabilities` list `GET /api/node` reports — the session
/// plane's wire contract (`sessions`/`projects`/`push`/`events`/
/// `export`, plus `pairing`/`import` when those backends are wired).
/// The substrate honors it via `HyphaState::node_capabilities`.
#[must_use]
pub fn node_capabilities(state: &AppState) -> Vec<String> {
    let mut capabilities = vec![
        "sessions".to_string(),
        "projects".to_string(),
        "push".to_string(),
        "events".to_string(),
        "export".to_string(),
    ];
    if state.pairing.is_some() {
        capabilities.push("pairing".into());
    }
    // The store-write seam (`POST /api/sessions/import`) is only wired
    // when a convertible store exists — the hub's transfer pre-flight
    // reads this flag instead of eating a 501.
    if state.convert.is_some() {
        capabilities.push("import".into());
    }
    capabilities
}

/// `GET /api/health` probe — the session-plane liveness check the
/// old route ran inline: a bounded `list_sessions` decides `{ok,db}`
/// and the 503. Installed on the substrate state as
/// `HyphaState::health_probe` at composition.
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
                    StatusCode::OK,
                )
            } else {
                routes::json_response(
                    serde_json::json!({ "ok": false, "db": false }),
                    StatusCode::SERVICE_UNAVAILABLE,
                )
            }
        }) as futures::future::BoxFuture<'static, Response>
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
        }) as futures::future::BoxFuture<'static, Vec<String>>
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
///   downcasts its host (the sepia-session plane no longer needs it —
///   [`sepia_peregrin::SepiaEntityPlane`] takes its backend at
///   construction).
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
    hypha.entity_host = Some(Arc::new(state.clone()));
    hypha.auth_cookie = Arc::from("sepia_token");
    hypha.health_probe = Some(health_probe(Arc::clone(&state.plane)));

    let mut kinds = KindRegistry::new();
    kinds.register(Arc::new(sepia_peregrin::SepiaEntityPlane::new(Arc::new(
        SessionPlaneBackend::new(state),
    ))));
    for plane in hypha.kinds.planes() {
        kinds.register(Arc::clone(plane));
    }
    for plane in state.kinds.planes() {
        kinds.register(Arc::clone(plane));
    }
    hypha.kinds = kinds;
}

/// Compose the full node surface — the substrate routes
/// (`hypha_http::app_routes`, fallback-free) merged with the session
/// routes under ONE listener and ONE 404 fallback (the session
/// router's). `hypha` is the caller-built substrate state (env-mapped
/// in `serve`); [`align_substrate`] then wires the shared singletons.
///
/// The returned router is `Router<()>` — serve it with
/// `into_make_service_with_connect_info::<SocketAddr>()`.
pub fn compose(state: AppState, mut hypha: hypha_http::HyphaState) -> Router {
    align_substrate(&state, &mut hypha);
    let substrate = hypha_http::app_routes(&hypha).with_state(hypha);
    session_app(state).merge(substrate)
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
/// [`align_substrate`], so the `HyphaState::from_env` pairing/push
/// instances opened on the same paths are superseded, not duplicated.
#[must_use]
pub fn hypha_state(env: &Env, state: &AppState) -> hypha_http::HyphaState {
    hypha_http::HyphaState::from_env(&env.to_hypha())
        .with_agent_lister(agent_lister(&state.plane))
        .with_node_capabilities(node_capabilities(state))
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

fn hostname_or_unknown() -> String {
    nix::unistd::gethostname()
        .ok()
        .map(|h| h.to_string_lossy().into_owned())
        .filter(|h| !h.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

/// CORS — port of `corsHeaders` + the OPTIONS preflight: `vary: Origin`
/// on every response, and the allow-* triple only when the request's
/// Origin is in the allowlist (or the allowlist carries `*`).
async fn cors_layer(State(state): State<AppState>, req: Request<Body>, next: Next) -> Response {
    let origin = req
        .headers()
        .get(header::ORIGIN)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    let allowed: HashSet<&str> = state.origins.iter().map(String::as_str).collect();
    let mut res = if req.method() == Method::OPTIONS {
        StatusCode::NO_CONTENT.into_response()
    } else {
        next.run(req).await
    };
    let headers = res.headers_mut();
    headers.insert(header::VARY, HeaderValue::from_static("Origin"));
    if let Some(origin) = origin {
        if allowed.contains(origin.as_str()) || allowed.contains("*") {
            if let Ok(v) = HeaderValue::from_str(&origin) {
                headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, v);
            }
            // PATCH/DELETE cover session meta, projects, config and
            // deletes for remote (federated) callers.
            headers.insert(
                header::ACCESS_CONTROL_ALLOW_METHODS,
                HeaderValue::from_static("GET,POST,PATCH,DELETE,OPTIONS"),
            );
            headers.insert(
                header::ACCESS_CONTROL_ALLOW_HEADERS,
                HeaderValue::from_static("content-type,authorization"),
            );
        }
    }
    res
}

/// OTEL server span — the outermost layer. Extracts the W3C remote
/// parent (`traceparent`/`tracestate`) so proxied hub→node calls join
/// the hub's trace; requests without context get a local-root
/// `http_request` span. Inert (plain span) when export is off.
async fn trace_layer(req: Request<Body>, next: Next) -> Response {
    let span = hypha_telemetry::server_span(req.method(), req.uri().path(), req.headers());
    let for_status = span.clone();
    async move {
        let res = next.run(req).await;
        hypha_telemetry::record_response(&for_status, res.status().as_u16());
        res
    }
    .instrument(span)
    .await
}

/// One line per request — `/api/health` excluded, like the TS logger.
async fn log_layer(req: Request<Body>, next: Next) -> Response {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let started = Instant::now();
    let res = next.run(req).await;
    if path != "/api/health" {
        // Hoisted — `tracing` fields evaluate only when a subscriber
        // is attached, which coverage runs don't guarantee.
        let status = res.status().as_u16();
        let elapsed_ms = started.elapsed().as_millis();
        tracing::info!("{method} {path} {status} {elapsed_ms}ms");
    }
    res
}

/// The session-plane router — `/api/sessions*`, agents, projects,
/// config, client keypair — with the auth gate, request log, CORS and
/// trace layers. Every route is `any()` + an internal method guard so
/// wrong-method requests fall through to the 404 (rather than axum's
/// bare 405). The generic substrate routes (`/api/entities*`,
/// `/api/events`, `/api/node`, `/api/pair`, `/api/auth/*`,
/// `/api/push/*`, `/api/fs`, `/api/user`, `/api/health`) are
/// `hypha-http`'s — [`compose`] merges the two under one listener.
fn session_app(state: AppState) -> Router {
    // The held-session watch ticker — re-probes lock state for sessions
    // last seen read-only while the feed has listeners.
    state
        .held
        .spawn_ticker(Arc::clone(&state.plane), state.feed.clone());

    Router::new()
        .route("/api/agents", any(routes::agents::agents))
        .route("/api/client/keypair", any(routes::client::keypair))
        .route("/api/config", any(routes::config::get))
        .route("/api/config/{key}", any(routes::config::set))
        .route("/api/projects", any(routes::projects::collection))
        .route("/api/projects/{id}", any(routes::projects::item))
        .route("/api/sessions", any(routes::sessions::collection))
        .route(
            "/api/sessions/import",
            any(routes::sessions::import_session),
        )
        .route("/api/sessions/{id}", any(routes::sessions::item))
        .route("/api/sessions/{id}/meta", any(routes::sessions::meta_alias))
        .route("/api/sessions/{id}/history", any(routes::sessions::history))
        .route(
            "/api/sessions/{id}/checkpoints",
            any(routes::sessions::checkpoints),
        )
        .route("/api/sessions/{id}/export", any(routes::sessions::export))
        .route(
            "/api/sessions/{id}/stream",
            any(routes::sessions::stream_events),
        )
        .route("/api/sessions/{id}/attach", any(routes::sessions::attach))
        .route("/api/sessions/{id}/detach", any(routes::sessions::detach))
        .route("/api/sessions/{id}/prompt", any(routes::sessions::prompt))
        .route("/api/sessions/{id}/cancel", any(routes::sessions::cancel))
        .route(
            "/api/sessions/{id}/permission",
            any(routes::sessions::permission),
        )
        .route(
            "/api/sessions/{id}/permissions",
            any(routes::sessions::permission),
        )
        .route("/api/sessions/{id}/restore", any(routes::sessions::restore))
        .route("/api/sessions/{id}/rewind", any(routes::sessions::rewind))
        .route("/api/sessions/{id}/convert", any(routes::sessions::convert))
        .fallback(async || routes::not_found())
        // Every body read goes through `read_json_body` with an
        // explicit per-route cap (SMALL/PROMPT/MAX_JSON_BODY); axum's
        // 2MiB `DefaultBodyLimit` stays on as the backstop for any
        // future extractor-based read (Json/Bytes/String/Multipart
        // all honor it).
        // Innermost → outermost: auth gate, request log, CORS, trace.
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::auth_gate,
        ))
        .layer(middleware::from_fn(log_layer))
        .layer(middleware::from_fn_with_state(state.clone(), cors_layer))
        .layer(middleware::from_fn(trace_layer))
        .with_state(state)
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
