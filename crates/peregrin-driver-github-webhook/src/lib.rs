//! peregrin-driver-github-webhook — `driver: github-webhook`, the
//! GitHub push ingress driver (ADR-0080 trigger plane; ADR-0082 §4 —
//! under the unified registry an ingress driver is just a driver that
//! emits asynchronously for the life of the run instead of returning
//! its events).
//!
//! A `github-webhook` node binds an HTTP listener when it fires and
//! returns immediately; for the rest of the run every verified push
//! delivery is pushed into the engine through the [`Injector`] the
//! driver was built with — the run drains it on the next loop
//! iteration (see `docs/reference/queue-supersede.md`). The daemon
//! constructs the driver with `pending.injector.clone()` off the
//! `RunSupervisor`'s `PendingRun`; standalone tests use
//! `peregrin_engine::ingress()` directly.
//!
//! Request handling mirrors bupha's `WebhookSource` so
//! `peregrin deploy webhook` can later just repoint the tunnel:
//! `POST <path>` only (anything else 404/405), `X-GitHub-Event`
//! `ping` → `200 pong` unverified, other event types → `400`,
//! `repository.full_name` outside the watched `repos` → `404`,
//! `X-Hub-Signature-256` mismatch → `401`, replayed
//! `X-GitHub-Delivery` → `200 duplicate`, deleted/zero-sha pushes →
//! `200 ignored`, malformed signed payloads → `400`, verified pushes
//! → `202 queued` with `Event{type:"push", payload:{repo, ref, sha,
//! before, actor, clone_url}, source: <node id>}` injected.
//!
//! Params (template-resolved like the other impls):
//!
//! - `listen` — bind address, default `127.0.0.1:29173` (bupha's
//!   port).
//! - `path` — hook path, default `/hook` (literal route — no `:`
//!   captures, `*` globs or `{name}` params).
//! - `repos` (required) — list of `{name: "owner/repo", secret_env:
//!   "ENV_NAME"}` mirroring bupha's watch config; `secret_env` names
//!   a *secret key* resolved through the `capabilities.secrets` gate
//!   + `ctx.get_secret` — never a literal secret in params
//!     (`peregrin-sink-github-status`'s `token_env` convention).
//! - `api_base` — forge API base (default `https://api.github.com`);
//!   derives the web base used for the `clone_url` fallback when a
//!   payload omits `repository.clone_url` (GHE `/api/v3` suffix or
//!   `api.` subdomain are stripped).
//!
//! Emitted (returned) events: `{node.id}.listening` on bind (or with
//! `dry_run: true` under `ctx.dry_run`), `node.failed` when the bind
//! fails — the v1 `webhook` driver conventions.
//!
//! # Shared ingress (`campaign serve`)
//!
//! [`SharedWebhook`] is the same verify/dedup/parse pipeline behind a
//! repo→routes *demux*: [`SharedWebhookDriver`] (registered as
//! `github-webhook` by `peregrin_node::campaign::registry_with_ingress_shared`)
//! never binds a socket — its `invoke` resolves the node's `repos`
//! secrets exactly like the solo driver and registers one [`Route`]
//! per repo (secret + the campaign's trigger [`Injector`] + the node
//! id as event `source` + the `api_base`-derived web base). The daemon
//! binds ONE listener via [`SharedWebhook::serve`] and every verified
//! push is delivered to *all* routes whose secret validated — so N
//! campaigns watching the same repo each get the event (a repo
//! registered with different secrets per campaign delivers only to
//! the campaigns whose secret signed the delivery). Non-matching
//! repos still 404; the per-node `listen`/`path` params are ignored —
//! the daemon's bind/path flags govern the single socket.

use std::collections::{HashMap, HashSet, VecDeque};
use std::net::{SocketAddr, TcpListener};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::JoinHandle;

use axum::Router;
use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::post;
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

use peregrin_core::{Event, EventSchema, Node, capability_matches};
use peregrin_driver::{Driver, DriverError, ExecutionContext};
use peregrin_engine::Injector;
use peregrin_template::{
    Context, Effects, EventScope, NodeScope, Options, ParamsScope, Scope, SystemEffects,
    resolve_value_opts,
};

/// `params.listen` default — bupha's webhook port, so the same
/// tunnel/agent config keeps working.
const DEFAULT_LISTEN: &str = "127.0.0.1:29173";
/// `params.path` default — bupha's hook path.
const DEFAULT_PATH: &str = "/hook";
/// `params.api_base` default — the public GitHub REST endpoint.
const DEFAULT_API_BASE: &str = "https://api.github.com";
/// Request-body cap — bupha's 2 MiB.
const MAX_BODY: usize = 2 * 1024 * 1024;
/// Delivery-id dedup ring bound — replays older than this re-emit.
const SEEN_CAP: usize = 1024;

/// One `params.repos[]` entry — the repo's `full_name` plus the
/// **name** of the secret holding its webhook secret.
struct RepoSpec {
    name: String,
    secret_env: String,
}

/// Delivery-id dedup ring (`bupha_core::Seen`): `insert` returns
/// `false` for a replayed id, and evicts oldest-first past `cap`.
struct Seen {
    ids: HashSet<String>,
    order: VecDeque<String>,
    cap: usize,
}

impl Seen {
    fn new(cap: usize) -> Self {
        Self {
            ids: HashSet::new(),
            order: VecDeque::new(),
            cap,
        }
    }

    /// `true` = fresh delivery, `false` = replay.
    fn insert(&mut self, id: &str) -> bool {
        if !self.ids.insert(id.to_string()) {
            return false;
        }
        self.order.push_back(id.to_string());
        while self.order.len() > self.cap {
            if let Some(old) = self.order.pop_front() {
                self.ids.remove(&old);
            }
        }
        true
    }
}

/// What a verified request asks for — `parse_push`'s three-way split
/// (bupha's `PushOutcome` plus the malformed arm).
enum Delivery {
    /// Well-formed event with nothing to run (deleted branch, zero sha).
    Skip,
    /// Signed but outside GitHub's push contract.
    Malformed,
    /// Fields for the `push` event payload.
    Push(Push),
}

/// The normalized push — everything the injected `push` event's
/// payload carries. `clone_url` stays `Option`: the payload's own
/// `repository.clone_url` wins, else each delivery route derives the
/// fallback from *its* `api_base` web base (routes can disagree).
struct Push {
    repo: String,
    git_ref: String,
    sha: String,
    before: String,
    actor: String,
    clone_url: Option<String>,
}

/// One watched repo's delivery target — where a verified push for
/// `repo` lands. The solo [`GithubWebhookDriver`] registers exactly
/// one route per repo; [`SharedWebhook`] accumulates routes across
/// campaigns (`campaign serve`'s demux).
#[derive(Clone)]
pub struct Route {
    /// The resolved webhook secret for this repo (this campaign's
    /// `secret_env` resolution — never a literal in params).
    pub secret: String,
    /// The campaign's trigger channel — verified pushes land here as
    /// `push` events.
    pub injector: Injector,
    /// `source` stamped on injected events — the owning node id.
    pub source: String,
    /// Web base derived from the node's `api_base` — the `clone_url`
    /// fallback when a payload omits `repository.clone_url`.
    pub web_base: String,
}

/// Per-listener state shared into the request handler — the repo→
/// routes demux plus the dedup ring request threads mutate. Routes
/// sit behind a mutex because `campaign serve` registers them from
/// listener-run threads while request threads read.
struct Shared {
    /// Watched repo `full_name` → delivery routes (all-match demux).
    routes: Mutex<HashMap<String, Vec<Route>>>,
    /// `X-GitHub-Delivery` dedup ring.
    seen: Mutex<Seen>,
}

/// The running listener — dropping it signals the serve loop and
/// joins the thread so no socket outlives its driver.
struct ListenerHandle {
    /// `true` flips the `stop.changed()` arm inside `serve_thread`.
    shutdown: tokio::sync::watch::Sender<bool>,
    /// The bound address (post-`:0` resolution) — `bound_addr()`.
    addr: Option<SocketAddr>,
    /// `Option` so `Drop` can `join` by value (the thread's
    /// `io::Result` payload is observed by nobody — the run already
    /// moved on).
    thread: Option<JoinHandle<std::io::Result<()>>>,
}

impl Drop for ListenerHandle {
    fn drop(&mut self) {
        let _ = self.shutdown.send(true);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// `driver: github-webhook`. `effects` backs the driver-side param
/// re-resolve the other impls perform inside `invoke`; `injector` is
/// the run's ingress channel the listener pushes verified pushes into.
pub struct GithubWebhookDriver {
    effects: Arc<dyn Effects>,
    injector: Injector,
    output: Vec<EventSchema>,
    /// The live listener, if the node fired — replaced on re-fire,
    /// dropped (and joined) with the driver.
    listener: Mutex<Option<ListenerHandle>>,
}

impl GithubWebhookDriver {
    /// A driver over [`SystemEffects`] pushing into `injector`.
    #[must_use]
    pub fn new(injector: Injector) -> Self {
        Self::with_effects(injector, Arc::new(SystemEffects))
    }

    /// A driver over custom effects (tests).
    #[must_use]
    pub fn with_effects(injector: Injector, effects: Arc<dyn Effects>) -> Self {
        Self {
            effects,
            injector,
            output: output_schemas(),
            listener: Mutex::new(None),
        }
    }

    /// The bound address of the live listener — `None` until the node
    /// fires successfully. With `listen: "…:0"` this is how callers
    /// (and tests) learn the assigned port.
    pub fn bound_addr(&self) -> Option<SocketAddr> {
        self.listener
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_ref()
            .and_then(|handle| handle.addr)
    }
}

/// Re-resolve `node.params` the way `driver: http`/`shell` do —
/// `event.*` binds the first trigger so `repos`/`listen` can
/// template off whatever woke the node. Shared by the solo and the
/// shared-ingress drivers.
fn resolve_params(
    effects: &dyn Effects,
    node: &Node,
    triggers: &[Event],
    ctx: &ExecutionContext,
) -> Result<Map<String, Value>, DriverError> {
    let tctx = Context {
        strict: ctx.strict,
        missing_secrets: ctx.missing_secrets.clone(),
        capabilities: (*ctx.capabilities).clone(),
        effects,
    };
    let state_view = |key: &str| ctx.state.borrow().get(key);
    let secret_view = |name: &str| ctx.resolve_secret(name);
    let resolved = resolve_value_opts(
        &node.params,
        &tctx,
        &Options {
            node: Some(node),
            functions: &[],
            scope: Scope {
                event: triggers.first().map(|event| EventScope {
                    event,
                    patterns: &node.on,
                }),
                config: Some(&ctx.config),
                params: ParamsScope::Resolved(&ctx.params),
                state: Some(&state_view),
                node: Some(NodeScope {
                    id: &node.id,
                    fires: ctx.fires,
                }),
                secrets: Some(&secret_view),
            },
        },
    )
    .map_err(|error| DriverError::new(error.to_string()))?;
    Ok(resolved.as_object().cloned().unwrap_or_default())
}

/// `params.<key>` as a non-empty string, else `default`.
fn str_param(params: &Map<String, Value>, key: &str, default: &str) -> String {
    params
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map_or_else(|| default.to_string(), str::to_string)
}

/// `params.path` must be a literal axum route — leading `/`, and no
/// `:`/`*`/`{`/`}` that would turn into a capture pattern.
fn valid_path(path: &str) -> bool {
    path.starts_with('/') && !path.contains([':', '*', '{', '}'])
}

/// `params.repos` → per-repo specs. Mirrors bupha's `[[repos]]`
/// checks: non-empty list, every entry `{name, secret_env}`, no
/// duplicate names.
fn parse_repo_specs(
    params: &Map<String, Value>,
    node: &Node,
) -> Result<Vec<RepoSpec>, DriverError> {
    let err =
        |reason: &str| DriverError::new(format!("github-webhook node '{}': {reason}", node.id));
    let Some(repos) = params
        .get("repos")
        .and_then(Value::as_array)
        .filter(|repos| !repos.is_empty())
    else {
        return Err(err(
            "missing 'params.repos' — a non-empty list of {name, secret_env}",
        ));
    };
    let mut seen = HashSet::new();
    let mut specs = Vec::with_capacity(repos.len());
    for entry in repos {
        let Some(name) = entry
            .get("name")
            .and_then(Value::as_str)
            .filter(|name| !name.is_empty())
        else {
            return Err(err("'params.repos[]' entries need a non-empty 'name'"));
        };
        if !seen.insert(name.to_string()) {
            return Err(err(&format!("duplicate repo '{name}' in 'params.repos'")));
        }
        let Some(secret_env) = entry
            .get("secret_env")
            .and_then(Value::as_str)
            .filter(|name| !name.is_empty())
        else {
            return Err(err(&format!(
                "repo '{name}' missing 'secret_env' — the secret's name, not its value"
            )));
        };
        specs.push(RepoSpec {
            name: name.to_string(),
            secret_env: secret_env.to_string(),
        });
    }
    Ok(specs)
}

/// Resolve each repo's `secret_env` to its secret — gated the way
/// `${secret.*}` is (`capabilities.secrets` present and not `false`;
/// under `strict` the name must match the grant), read through
/// `ctx.get_secret` so strict-mode misses land on the shared ledger.
/// The `github-status` `token_env` convention, per repo.
fn resolve_secrets(
    specs: &[RepoSpec],
    node: &Node,
    ctx: &ExecutionContext,
) -> Result<HashMap<String, String>, DriverError> {
    let deny =
        |reason: &str| DriverError::new(format!("github-webhook node '{}': {reason}", node.id));
    let mut secrets = HashMap::with_capacity(specs.len());
    for spec in specs {
        let env_name = spec.secret_env.as_str();
        let denied = match ctx.capabilities.get("secrets") {
            None | Some(Value::Bool(false)) => true,
            Some(grant) => ctx.strict && !capability_matches(env_name, grant),
        };
        if denied {
            return Err(deny(&format!(
                "capability 'secrets' not granted for '{env_name}'"
            )));
        }
        let secret = ctx.get_secret(env_name);
        if secret.is_empty() {
            return Err(deny(&format!("secret '{env_name}' unresolved")));
        }
        secrets.insert(spec.name.clone(), secret);
    }
    Ok(secrets)
}

/// `params.api_base` → the web base for the `clone_url` fallback:
/// a GHE `…/api/v3` suffix or an `api.` subdomain strips off, so
/// `https://api.github.com` → `https://github.com` and
/// `https://ghe.acme/api/v3` → `https://ghe.acme`.
fn web_base(api_base: &str) -> String {
    let base = api_base.trim_end_matches('/');
    if let Some(prefix) = base.strip_suffix("/api/v3") {
        return prefix.to_string();
    }
    base.replacen("://api.", "://", 1)
}

/// A revision is path- and argv-safe (bupha's `is_valid_sha`) —
/// rejects path-injection payloads before the value reaches worktree
/// paths, and option-injection payloads before a `git` argv position.
fn is_valid_sha(sha: &str) -> bool {
    (7..=64).contains(&sha.len())
        && !sha.starts_with('-')
        && !sha.contains("..")
        && !sha.contains('/')
        && sha
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}

/// HMAC-SHA256 over `body` keyed by `secret` — the RFC 2104
/// construction on `sha2` directly, keeping the dep tree to crates
/// already in the workspace (the `hmac` crate's only fallible edge —
/// `new_from_slice` — cannot fail, so a hand-rolled keyed hash also
/// keeps every line testable).
fn hmac_sha256(secret: &[u8], body: &[u8]) -> [u8; 32] {
    const BLOCK: usize = 64;
    let mut key = [0u8; BLOCK];
    if secret.len() > BLOCK {
        key[..32].copy_from_slice(&Sha256::digest(secret));
    } else {
        key[..secret.len()].copy_from_slice(secret);
    }
    let padded = |xor: u8, data: &[u8]| {
        let mut hasher = Sha256::new();
        hasher.update(key.map(|b| b ^ xor));
        hasher.update(data);
        hasher.finalize()
    };
    let inner = padded(0x36, body);
    padded(0x5c, &inner).into()
}

/// The `sha256=<hex>` `X-Hub-Signature-256` value for a body — the
/// signing half of the verify path. Tests sign deliveries with it;
/// `peregrin deploy webhook` probes can too.
pub fn signature_header(secret: &str, body: &[u8]) -> String {
    format!(
        "sha256={}",
        hex::encode(hmac_sha256(secret.as_bytes(), body))
    )
}

/// `sha256=<hex>` constant-time check — the `X-Hub-Signature-256`
/// contract. Any shape deviation (missing header, wrong prefix, bad
/// hex, wrong length) is just a non-match.
fn signature_valid(secret: &str, body: &[u8], signature: Option<&str>) -> bool {
    let Some(hex_sig) = signature.and_then(|s| s.strip_prefix("sha256=")) else {
        return false;
    };
    let Ok(sig) = hex::decode(hex_sig) else {
        return false;
    };
    let mac = hmac_sha256(secret.as_bytes(), body);
    sig.len() == mac.len()
        && sig
            .iter()
            .zip(mac.iter())
            .fold(0u8, |acc, (a, b)| acc | (a ^ b))
            == 0
}

/// Parse a verified `push` delivery's already-decoded JSON — the
/// `Event.payload` fields, bupha's skip rules (branch deletion,
/// all-zero `after`) and sha safety check included. `repo` is the
/// `repository.full_name` hint [`respond`] already matched against
/// the route table.
fn parse_push(v: &Value, repo: &str) -> Delivery {
    let (Some(git_ref), Some(sha)) = (
        v.get("ref").and_then(Value::as_str),
        v.get("after").and_then(Value::as_str),
    ) else {
        return Delivery::Malformed;
    };
    // Branch deletions arrive with after = "000…" (and deleted:true)
    // — well-formed, but there is nothing to build.
    if v.get("deleted").and_then(Value::as_bool) == Some(true) || sha.bytes().all(|b| b == b'0') {
        return Delivery::Skip;
    }
    // Signed but malformed for GitHub's contract — reject as a bad
    // payload so it shows up in the delivery log, never enqueued.
    if !is_valid_sha(sha) {
        return Delivery::Malformed;
    }
    let text = |value: Option<&Value>| {
        value
            .and_then(Value::as_str)
            .map_or_else(String::new, str::to_string)
    };
    Delivery::Push(Push {
        repo: repo.to_string(),
        git_ref: git_ref.to_string(),
        sha: sha.to_string(),
        before: text(v.get("before")),
        actor: text(v.pointer("/pusher/name")),
        clone_url: v
            .pointer("/repository/clone_url")
            .and_then(Value::as_str)
            .map(str::to_string),
    })
}

/// `Option<&str>` view of a header (non-UTF8 values read as absent).
fn header<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name).and_then(|value| value.to_str().ok())
}

/// The whole request contract — one POST in, one status+message out.
/// Ordered exactly like bupha's `WebhookSource`: the hook path and
/// method are enforced by the route (404/405), then ping → event
/// check → repo hint → verify → dedup → parse → inject *to every
/// matched route* (the `campaign serve` all-match demux — a repo two
/// campaigns watch delivers to both; routes whose secret didn't
/// validate the signature are skipped, so a repo registered under
/// different per-campaign secrets only reaches the signer).
fn respond(shared: &Shared, headers: &HeaderMap, body: &[u8]) -> (StatusCode, &'static str) {
    let event_kind = header(headers, "x-github-event");
    if event_kind == Some("ping") {
        // Hook-registration probe — no signature, no queue.
        return (StatusCode::OK, "pong");
    }
    if event_kind != Some("push") {
        return (StatusCode::BAD_REQUEST, "not a push event");
    }
    // The body parse and `repository.full_name` peek are untrusted
    // until verify passes — the name only selects which routes (and
    // their secrets) apply. An unwatched repo can never verify (no
    // secret of ours signed it): reject, don't ignore.
    let Ok(v) = serde_json::from_slice::<Value>(body) else {
        return (StatusCode::NOT_FOUND, "unwatched repo");
    };
    let Some(repo) = v.pointer("/repository/full_name").and_then(Value::as_str) else {
        return (StatusCode::NOT_FOUND, "unwatched repo");
    };
    let routes: Vec<Route> = shared
        .routes
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(repo)
        .cloned()
        .unwrap_or_default();
    if routes.is_empty() {
        return (StatusCode::NOT_FOUND, "unwatched repo");
    }
    let signature = header(headers, "x-hub-signature-256");
    let matched: Vec<&Route> = routes
        .iter()
        .filter(|route| signature_valid(&route.secret, body, signature))
        .collect();
    if matched.is_empty() {
        return (StatusCode::UNAUTHORIZED, "bad signature");
    }
    if let Some(id) = header(headers, "x-github-delivery") {
        let fresh = shared
            .seen
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(id);
        if !fresh {
            return (StatusCode::OK, "duplicate delivery");
        }
    }
    match parse_push(&v, repo) {
        Delivery::Skip => (StatusCode::OK, "ignored: no-op event"),
        Delivery::Malformed => (StatusCode::BAD_REQUEST, "unparseable payload"),
        Delivery::Push(push) => {
            // One event per matched route — `source` and the
            // `clone_url` fallback are per-route (each campaign's
            // node id and `api_base`).
            let mut delivered = false;
            for route in &matched {
                let clone_url = push
                    .clone_url
                    .clone()
                    .unwrap_or_else(|| format!("{}/{repo}.git", route.web_base));
                let event = Event::new(
                    "push",
                    serde_json::json!({
                        "repo": push.repo,
                        "ref": push.git_ref,
                        "sha": push.sha,
                        "before": push.before,
                        "actor": push.actor,
                        "clone_url": clone_url,
                    }),
                    &route.source,
                );
                // A route whose campaign already ended drops the
                // event — the others still count as delivered.
                delivered |= route.injector.inject(event).is_ok();
            }
            if delivered {
                // bupha's `Verdict::Queued` → 202.
                (StatusCode::ACCEPTED, "queued")
            } else {
                // Every matched run ended under the listener.
                (StatusCode::SERVICE_UNAVAILABLE, "run ended")
            }
        }
    }
}

/// The axum handler — a thin async shell over [`respond`] so the
/// contract itself stays a plain (unit-testable) function.
async fn handle(
    State(shared): State<Arc<Shared>>,
    headers: HeaderMap,
    body: Bytes,
) -> (StatusCode, &'static str) {
    respond(&shared, &headers, &body)
}

/// The one-route app — `post(handle)` makes non-POST methods on
/// `path` a 405 and every other path a 404, matching bupha's ordering.
fn app(path: &str, shared: Arc<Shared>) -> Router {
    Router::new()
        .route(path, post(handle))
        .layer(DefaultBodyLimit::max(MAX_BODY))
        .with_state(shared)
}

/// The listener thread's whole life: a runtime serving `app` until
/// `stop` flips (driver dropped/replaced — `ListenerHandle`'s drop),
/// then aborts whatever connection tasks remain. Failures ride `?` —
/// they are OS-level inconsistencies no test can stage after a
/// successful bind, and an `else { return }` would be an unexercisable
/// line under the coverage gate. Hyper's graceful shutdown closes
/// keep-alive conns after their in-flight request, so `serve` always
/// returns; `shutdown_background` is the belt for stragglers.
fn serve_thread(
    listener: TcpListener,
    shared: Arc<Shared>,
    path: String,
    mut stop: tokio::sync::watch::Receiver<bool>,
) -> std::io::Result<()> {
    let runtime = tokio::runtime::Runtime::new()?;
    let served = runtime.block_on(async move {
        let listener = tokio::net::TcpListener::from_std(listener)?;
        axum::serve(listener, app(&path, shared))
            .with_graceful_shutdown(async move {
                let _ = stop.changed().await;
            })
            .await
    });
    runtime.shutdown_background();
    served
}

/// Spawn the listener thread. The socket is already bound (invoke did
/// the `TcpListener::bind` so bind failures surface as `node.failed`);
/// `set_nonblocking` runs here — a failure is discarded the same way
/// `from_std` treats it (the OS call that can't fail after a
/// successful bind).
fn spawn_listener(
    listener: TcpListener,
    shared: Arc<Shared>,
    path: &str,
    addr: Option<SocketAddr>,
) -> ListenerHandle {
    let _ = listener.set_nonblocking(true);
    let (tx, rx) = tokio::sync::watch::channel(false);
    let path = path.to_string();
    let thread = std::thread::spawn(move || serve_thread(listener, shared, path, rx));
    ListenerHandle {
        shutdown: tx,
        addr,
        thread: Some(thread),
    }
}

/// `output` contracts — only `node.failed` is statically nameable;
/// the `{id}.listening` confirmation's type is per-node so its
/// contract rides the node's `output:` (the `github-status`
/// convention for `.status-posted`).
fn output_schemas() -> Vec<EventSchema> {
    vec![EventSchema {
        kind: "node.failed".to_string(),
        fields: Map::from_iter([
            ("error".to_string(), Value::String("string".to_string())),
            ("node".to_string(), Value::String("string".to_string())),
        ]),
        schema: Map::new(),
        annotations: Map::new(),
    }]
}

impl Driver for GithubWebhookDriver {
    fn name(&self) -> &'static str {
        "github-webhook"
    }

    fn is_ingress(&self) -> bool {
        true
    }

    fn output(&self) -> &[EventSchema] {
        &self.output
    }

    /// Bind the listener, spawn the serve thread, return the
    /// `{id}.listening` readiness event — the v1 `webhook` driver's
    /// signal. Delivery events never come back through `invoke`; they
    /// flow through the `Injector` for the rest of the run.
    fn invoke(
        &self,
        node: &Node,
        triggers: &[Event],
        ctx: &ExecutionContext,
    ) -> Result<Vec<Event>, DriverError> {
        let params = resolve_params(&*self.effects, node, triggers, ctx)?;
        let listen = str_param(&params, "listen", DEFAULT_LISTEN);
        let path = str_param(&params, "path", DEFAULT_PATH);
        if !valid_path(&path) {
            return Err(DriverError::new(format!(
                "github-webhook node '{}': 'params.path' must be a literal route \
                 (leading '/', no captures): '{path}'",
                node.id
            )));
        }
        let api_base = str_param(&params, "api_base", DEFAULT_API_BASE);
        let specs = parse_repo_specs(&params, node)?;
        let names: Vec<Value> = specs
            .iter()
            .map(|spec| Value::String(spec.name.clone()))
            .collect();
        let kind = format!("{}.listening", node.id);

        if ctx.dry_run {
            return Ok(vec![Event::new(
                kind,
                serde_json::json!({
                    "dry_run": true,
                    "listen": listen,
                    "path": path,
                    "repos": names,
                }),
                &node.id,
            )]);
        }

        let secrets = resolve_secrets(&specs, node, ctx)?;
        let web_base = web_base(&api_base);

        let listener = match TcpListener::bind(&listen) {
            Ok(listener) => listener,
            Err(error) => {
                return Ok(vec![Event::node_failed(
                    &node.id,
                    &format!("github-webhook failed to bind {listen}: {error}"),
                )]);
            }
        };
        let addr = listener.local_addr().ok();
        // One route per watched repo — the solo driver's whole route
        // table (the demux generalizes this to N campaigns).
        let mut routes: HashMap<String, Vec<Route>> = HashMap::new();
        for spec in &specs {
            routes.entry(spec.name.clone()).or_default().push(Route {
                secret: secrets[&spec.name].clone(),
                injector: self.injector.clone(),
                source: node.id.clone(),
                web_base: web_base.clone(),
            });
        }
        let shared = Arc::new(Shared {
            routes: Mutex::new(routes),
            seen: Mutex::new(Seen::new(SEEN_CAP)),
        });
        let handle = spawn_listener(listener, shared, &path, addr);
        let listen = addr.map_or(listen, |addr| addr.to_string());
        *self.listener.lock().unwrap_or_else(PoisonError::into_inner) = Some(handle);

        Ok(vec![Event::new(
            kind,
            serde_json::json!({
                "listen": listen,
                "path": path,
                "repos": names,
                "api_base": api_base,
            }),
            &node.id,
        )])
    }
}

// ---------------------------------------------------------------------------
// Shared ingress — `campaign serve`'s one-listener-N-campaigns demux.
// ---------------------------------------------------------------------------

/// The shared webhook ingress `campaign serve` binds once for every
/// hosted campaign: a repo→routes demux over the same
/// verify/dedup/parse pipeline [`GithubWebhookDriver`] uses solo.
///
/// [`SharedWebhookDriver`]s (one per campaign's `github-webhook`
/// node, inside that campaign's persistent listener run) call
/// [`register`](Self::register) when they fire; [`serve`](Self::serve)
/// binds the socket — the order doesn't matter, requests simply 404
/// until a route exists. [`stop`](Self::stop) (or drop) ends the
/// listener thread; routes may stay registered — deliveries to a
/// dead campaign's channel just fail inject and count toward the
/// all-or-nothing status like the solo driver's `run ended` arm.
pub struct SharedWebhook {
    /// The route table + dedup ring the request handler reads —
    /// `Arc` so `register` and `serve` share it with the listener
    /// thread.
    shared: Arc<Shared>,
    /// The live listener — `Some` between [`serve`](Self::serve) and
    /// [`stop`](Self::stop)/drop.
    listener: Mutex<Option<ListenerHandle>>,
}

impl SharedWebhook {
    /// An empty demux — no routes, not yet serving.
    #[must_use]
    pub fn new() -> Self {
        Self {
            shared: Arc::new(Shared {
                routes: Mutex::new(HashMap::new()),
                seen: Mutex::new(Seen::new(SEEN_CAP)),
            }),
            listener: Mutex::new(None),
        }
    }

    /// Register `route` for `repo` (`repository.full_name`) — called
    /// by [`SharedWebhookDriver::invoke`] once per `params.repos[]`
    /// entry. Re-registering appends: two campaigns watching the same
    /// repo both receive deliveries (all-match), and a repo listed
    /// twice *within* one campaign delivers twice (the supervisor's
    /// supersede dedupe absorbs the duplicate trigger).
    pub fn register(&self, repo: &str, route: Route) {
        self.shared
            .routes
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .entry(repo.to_string())
            .or_default()
            .push(route);
    }

    /// The watched repo names — status surfaces and tests.
    #[must_use]
    pub fn repos(&self) -> Vec<String> {
        let mut repos: Vec<String> = self
            .shared
            .routes
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .keys()
            .cloned()
            .collect();
        repos.sort();
        repos
    }

    /// Total registered routes across all repos.
    #[must_use]
    pub fn route_count(&self) -> usize {
        self.shared
            .routes
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .values()
            .map(Vec::len)
            .sum()
    }

    /// Bind `listen` and spawn the serve thread on `path` — the same
    /// socket lifecycle [`GithubWebhookDriver`] gets from `invoke`,
    /// minus the run. `listen: "…:0"` resolves to the assigned port
    /// via [`bound_addr`](Self::bound_addr). Calling twice is an
    /// error — one demux, one socket.
    ///
    /// # Errors
    /// `path` failing the literal-route rule, a bind failure, or a
    /// second `serve` call — as a displayable message.
    pub fn serve(&self, listen: &str, path: &str) -> Result<SocketAddr, String> {
        if !valid_path(path) {
            return Err(format!(
                "shared webhook 'path' must be a literal route \
                 (leading '/', no captures): '{path}'"
            ));
        }
        let mut slot = self.listener.lock().unwrap_or_else(PoisonError::into_inner);
        if slot.is_some() {
            return Err("shared webhook listener already serving".to_string());
        }
        let listener = TcpListener::bind(listen)
            .map_err(|e| format!("shared webhook failed to bind {listen}: {e}"))?;
        let addr = listener
            .local_addr()
            .map_err(|e| format!("shared webhook local_addr: {e}"))?;
        *slot = Some(spawn_listener(
            listener,
            Arc::clone(&self.shared),
            path,
            Some(addr),
        ));
        Ok(addr)
    }

    /// The bound address — `None` before [`serve`](Self::serve) or
    /// after [`stop`](Self::stop).
    #[must_use]
    pub fn bound_addr(&self) -> Option<SocketAddr> {
        self.listener
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_ref()
            .and_then(|handle| handle.addr)
    }

    /// Stop the listener — signals the serve thread and joins it, so
    /// no socket outlives the call. Routes stay registered (deliveries
    /// to dead channels just fail inject); the slot clears, so a
    /// later [`serve`](Self::serve) re-binds.
    pub fn stop(&self) {
        self.listener
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
    }
}

impl Default for SharedWebhook {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for SharedWebhook {
    fn drop(&mut self) {
        // Last `Arc` gone — `ListenerHandle`'s own Drop signals +
        // joins the serve thread.
        let _ = self
            .listener
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
    }
}

/// `driver: github-webhook` under `campaign serve` — resolves the
/// node's `repos`/secrets exactly like [`GithubWebhookDriver`] but
/// registers routes into a [`SharedWebhook`] demux instead of binding
/// its own listener (the daemon owns the one socket; the node's
/// `listen`/`path` params are ignored). Still `is_ingress` — the
/// campaign's listener run fires it once and it stays parked.
pub struct SharedWebhookDriver {
    effects: Arc<dyn Effects>,
    /// The campaign's queue-facing trigger channel — becomes every
    /// registered route's delivery target.
    injector: Injector,
    /// The demux this driver's `invoke` registers into.
    demux: Arc<SharedWebhook>,
    output: Vec<EventSchema>,
}

impl SharedWebhookDriver {
    /// A driver over [`SystemEffects`] registering into `demux`.
    #[must_use]
    pub fn new(injector: Injector, demux: Arc<SharedWebhook>) -> Self {
        Self::with_effects(injector, demux, Arc::new(SystemEffects))
    }

    /// A driver over custom effects (tests).
    #[must_use]
    pub fn with_effects(
        injector: Injector,
        demux: Arc<SharedWebhook>,
        effects: Arc<dyn Effects>,
    ) -> Self {
        Self {
            effects,
            injector,
            demux,
            output: output_schemas(),
        }
    }
}

impl Driver for SharedWebhookDriver {
    fn name(&self) -> &'static str {
        "github-webhook"
    }

    fn is_ingress(&self) -> bool {
        true
    }

    fn output(&self) -> &[EventSchema] {
        &self.output
    }

    /// Resolve `repos` + secrets like the solo driver, register one
    /// [`Route`] per repo into the demux, return `{id}.listening`
    /// marked `shared: true`. No socket is bound — the daemon's
    /// [`SharedWebhook::serve`] owns it.
    fn invoke(
        &self,
        node: &Node,
        triggers: &[Event],
        ctx: &ExecutionContext,
    ) -> Result<Vec<Event>, DriverError> {
        let params = resolve_params(&*self.effects, node, triggers, ctx)?;
        let api_base = str_param(&params, "api_base", DEFAULT_API_BASE);
        let specs = parse_repo_specs(&params, node)?;
        let names: Vec<Value> = specs
            .iter()
            .map(|spec| Value::String(spec.name.clone()))
            .collect();
        let kind = format!("{}.listening", node.id);

        if ctx.dry_run {
            return Ok(vec![Event::new(
                kind,
                serde_json::json!({
                    "dry_run": true,
                    "shared": true,
                    "repos": names,
                }),
                &node.id,
            )]);
        }

        let secrets = resolve_secrets(&specs, node, ctx)?;
        let web_base = web_base(&api_base);
        for spec in &specs {
            self.demux.register(
                &spec.name,
                Route {
                    secret: secrets[&spec.name].clone(),
                    injector: self.injector.clone(),
                    source: node.id.clone(),
                    web_base: web_base.clone(),
                },
            );
        }

        Ok(vec![Event::new(
            kind,
            serde_json::json!({
                "shared": true,
                "repos": names,
                "api_base": api_base,
            }),
            &node.id,
        )])
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    /// The dedup ring evicts oldest-first past `cap` and reports
    /// replays — exercised directly since `SEEN_CAP` deliveries would
    /// just waste test time.
    #[test]
    fn seen_ring_dedups_and_evicts() {
        let mut seen = Seen::new(2);
        assert!(seen.insert("a"));
        assert!(seen.insert("b"));
        assert!(!seen.insert("a"), "replay while in the ring");
        assert!(seen.insert("c"), "third id evicts 'a'");
        assert!(seen.insert("a"), "evicted id counts fresh again");
        assert!(!seen.insert("a"));
    }
}
