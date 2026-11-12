//! Boot-time configuration — port of `apps/server/src/env.ts` minus the
//! surfaces this crate doesn't serve (UI assets, the servers registry,
//! the Devin DB — the driver binary opens that itself via
//! `SEPIA_DEVIN_DB`). Plane tunables `sepia-session`/`sepia-acp` read
//! directly (`SEPIA_LOCK_TTL_MS`, `SEPIA_HISTORY_LIMIT`,
//! `SEPIA_INHERIT_ENV`, `SEPIA_DEBUG`) deliberately don't ride `Env`;
//! neither do the OTEL knobs `hypha-telemetry` reads at boot
//! (`HYPHA_OTEL_ENDPOINT`, `HYPHA_OTEL_SERVICE_NAME`, and the standard
//! `OTEL_EXPORTER_OTLP_*` vars — see `DEPLOY.md` "Telemetry").
//! `Env::parse` fails fast on bad values, like the TS schema.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

pub const DEFAULT_PORT: u16 = 18787;
pub const DEFAULT_HOST: &str = "127.0.0.1";
pub const DEFAULT_SSE_KEEPALIVE_MS: u64 = 15_000;
pub const DEFAULT_HELD_WATCH_MS: u64 = 5_000;
pub const DEFAULT_IDLE_TTL_MS: u64 = 600_000;
pub const DEFAULT_SWEEP_MS: u64 = 30_000;
/// `POST /api/pair` redeem attempts per client IP: ~40-bit codes make
/// online guessing infeasible anyway, so these only need to throttle
/// abuse on nodes exposed off-LAN — not gate legit pairing.
pub const DEFAULT_PAIR_LIMIT_PER_MIN: u32 = 5;
pub const DEFAULT_PAIR_LIMIT_PER_DAY: u32 = 20;

const DEFAULT_ORIGINS: [&str; 2] = ["http://localhost:13000", "http://127.0.0.1:13000"];

/// A boot-time configuration error — message mirrors the TS throw text.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct EnvError(pub String);

/// Parsed `PEREGRIN_*` environment — every field `env.ts`/`serve.ts` reads.
#[derive(Clone, Debug)]
pub struct Env {
    /// `PEREGRIN_PORT` (falls back to the TS `PORT`).
    pub port: u16,
    /// `PEREGRIN_HOST`.
    pub host: String,
    /// `PEREGRIN_TOKEN` — bearer auth; required on non-loopback binds.
    pub token: Option<String>,
    /// `PEREGRIN_META`/`PEREGRIN_META_PATH` — the overlay JSON file.
    pub meta_path: PathBuf,
    /// `PEREGRIN_HOME` — peregrin-owned data dir (node.json, meta.json, tokens).
    pub home: PathBuf,
    /// `PEREGRIN_NODE` — stable node identity file.
    pub node_path: PathBuf,
    /// `PEREGRIN_NAME` — display name reported by `GET /api/node`.
    pub node_name: String,
    /// `PEREGRIN_ORIGINS` — CORS allowlist (`*` echoes any origin).
    pub origins: Vec<String>,
    /// `PEREGRIN_CLINE_DIR` (default `~/.cline/data`) — the convert target.
    pub cline_dir: PathBuf,
    /// `PEREGRIN_CLAUDE_DIR` (default `~/.claude`) — file-history restore.
    pub claude_dir: PathBuf,
    /// `PEREGRIN_SSE_KEEPALIVE_MS` — keep-alive cadence; `0` disables.
    pub sse_keep_alive: Duration,
    /// `PEREGRIN_HELD_WATCH_MS` — held-session re-probe cadence; `0` disables.
    pub held_watch: Duration,
    /// `PEREGRIN_IDLE_TTL_MS` — idle live-session detach (plane option).
    pub idle_ttl: Duration,
    /// `PEREGRIN_SWEEP_MS` — idle sweep cadence (plane option).
    pub sweep: Duration,
    /// `PEREGRIN_PAIR_LIMIT_PER_MIN` — `/api/pair` attempts/min per client
    /// IP (sliding window); `0` disables this window.
    pub pair_limit_per_min: u32,
    /// `PEREGRIN_PAIR_LIMIT_PER_DAY` — `/api/pair` attempts/24h per client
    /// IP; `0` disables. Both at `0` turns the throttle off entirely.
    pub pair_limit_per_day: u32,
    /// `<home>/pair-code` — written by `peregrin pair`, consumed on read.
    pub pair_code_path: PathBuf,
    /// `<home>/tokens.json` — sha256 hashes of issued pair credentials.
    pub tokens_path: PathBuf,
    /// `PEREGRIN_KV`/`PEREGRIN_KV_PATH` — the node-local KV document
    /// (`/api/kv*` + the projects/config compat routes, P-006);
    /// `<home>/kv.json` by default. Shared with the substrate state —
    /// one `JsonKvStore` instance serves both routers.
    pub kv_path: PathBuf,
    /// `PEREGRIN_EPHEMERAL` — elastic/CI node mode (`peregrin serve
    /// --ephemeral` sets the same flag): SIGTERM-aware graceful
    /// shutdown plus a snapshot flush of live/locked entities on exit
    /// (see `peregrin_node::ephemeral`). Node identity, auth, and the
    /// `/api/*` surface are unchanged — ephemeral is a lifecycle
    /// mode, not a trust downgrade.
    pub ephemeral: bool,
    /// `PEREGRIN_SNAPSHOT_DIR` — the shutdown flush's output dir
    /// (default `<home>/snapshots`): `<kind>/<id>.snapshot.json`
    /// files plus an `index.json` manifest.
    pub snapshot_dir: PathBuf,
    /// `PEREGRIN_SNAPSHOT_TARGET` — optional peer node (or hub) base
    /// URL that receives each flushed snapshot via
    /// `POST /api/entities/snapshot` during shutdown. With the `bupha`
    /// feature it doubles as the boot-time ci-job rehydrate source
    /// (T-072): `GET /api/entity-index` rebuilds this node's
    /// `kind="ci-job"` plane before the listener binds.
    pub snapshot_target: Option<String>,
    /// `PEREGRIN_SNAPSHOT_TOKEN` — bearer for `snapshot_target`;
    /// falls back to `PEREGRIN_TOKEN` (single-secret deployments
    /// reuse it across nodes).
    pub snapshot_token: Option<String>,
    /// `PEREGRIN_SNAPSHOT_NODE` — optional `?node=` selector sent with
    /// the handoff POST; meaningful when `snapshot_target` is a hub
    /// (routes the import to that hub-registered node). `None` = the
    /// target's default/primary node.
    pub snapshot_node: Option<String>,
    /// `PEREGRIN_RUNNER_LABELS` — comma-separated runner/fleet labels
    /// (`runner:linux,gpu`) appended to the `/api/node` capability
    /// list. Campaign `runs_on:` requirements match against these —
    /// the hub's sync projection persists them for fleet routing
    /// (PL-007 C4).
    pub runner_labels: Vec<String>,
    /// `PEREGRIN_PUSH_BACKEND` — where Web Push state (VAPID keys +
    /// subscriptions) lives: `kv` (default — the `push` namespace of
    /// `kv.json`) or `cuora` (the cuora broker, refs
    /// `cuora://peregrin/push/<key>`, dual-writing the kv namespace
    /// as a warm fallback during the migration).
    pub push_backend: PushBackendKind,
}

/// `PEREGRIN_PUSH_BACKEND` values — see [`Env::push_backend`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PushBackendKind {
    /// The node-local KV `push` namespace (`kv.json`).
    #[default]
    Kv,
    /// The cuora secrets broker (`cuora://peregrin/push/*`), with a
    /// write-through mirror into the KV namespace.
    Cuora,
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map_or_else(|| PathBuf::from("/"), PathBuf::from)
}

fn hostname_or_default() -> String {
    nix::unistd::gethostname()
        .ok()
        .map(|h| h.to_string_lossy().into_owned())
        .filter(|h| !h.is_empty())
        .unwrap_or_else(|| "localhost".to_string())
}

/// `serve.ts`'s bind guard — loopback binds may go tokenless.
pub fn is_loopback_host(host: &str) -> bool {
    host == "localhost" || host == "::1" || host == "[::1]" || host.starts_with("127.")
}

/// `PEREGRIN_SSE_KEEPALIVE_MS`/`PEREGRIN_HELD_WATCH_MS` style parse: absent →
/// default; a non-finite or negative value falls back to the default;
/// `0` disables.
fn ms_or_default(raw: Option<&String>, default: u64) -> u64 {
    let Some(raw) = raw else { return default };
    if raw.is_empty() {
        return default;
    }
    match raw.parse::<f64>() {
        Ok(v) if v.is_finite() && v >= 0.0 => v.floor() as u64,
        _ => default,
    }
}

fn millis_u64(raw: Option<&String>, default: u64) -> u64 {
    raw.and_then(|v| v.parse::<u64>().ok()).unwrap_or(default)
}

/// Truthy env flag — `1`/`true`/`yes`/`on` (case-insensitive) set it;
/// anything else (including absent) leaves it off. Deliberately
/// permissive: a typo must not flip lifecycle semantics, and the
/// explicit-off spellings all read false anyway.
fn env_flag(get: &impl Fn(&str) -> Option<String>, key: &str) -> bool {
    get(key).is_some_and(|v| {
        matches!(
            v.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
}

/// Count-valued knob: absent/unparseable → default (never silently 0 —
/// a typo must not disable the knob), `0` is the explicit off switch.
fn count_or_default(raw: Option<&String>, default: u32) -> u32 {
    raw.filter(|v| !v.is_empty())
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(default)
}

impl Env {
    /// Parse from the process environment; fails fast on bad values.
    ///
    /// # Errors
    /// `EnvError` on a bad port or a non-loopback bind without
    /// `PEREGRIN_TOKEN`.
    pub fn parse() -> Result<Self, EnvError> {
        Self::from_lookup(|key| std::env::var(key).ok())
    }

    /// Test seam — parse from a key/value map instead of `std::env`.
    pub fn from_map(vars: &HashMap<String, String>) -> Result<Self, EnvError> {
        Self::from_lookup(|key| vars.get(key).cloned())
    }

    /// Shared parser behind [`Env::parse`]/[`Env::from_map`].
    ///
    /// # Errors
    /// See [`Env::parse`].
    pub fn from_lookup(get: impl Fn(&str) -> Option<String>) -> Result<Self, EnvError> {
        let home_dir = home_dir();

        // TS reads PORT; the Rust surface prefers the namespaced var.
        let raw_port = get("PEREGRIN_PORT").or_else(|| get("PORT"));
        let port: u16 = match raw_port.as_deref() {
            None | Some("") => DEFAULT_PORT,
            Some(raw) => match raw.parse::<f64>() {
                Ok(v) if v.fract() == 0.0 && (1.0..=65535.0).contains(&v) => v as u16,
                _ => {
                    return Err(EnvError(format!(
                        "PORT must be an integer between 1 and 65535, got \"{raw}\""
                    )));
                }
            },
        };

        let origins: Vec<String> = get("PEREGRIN_ORIGINS")
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect();
        let origins = if origins.is_empty() {
            DEFAULT_ORIGINS.iter().map(|s| (*s).to_string()).collect()
        } else {
            origins
        };

        let home = get("PEREGRIN_HOME")
            .map_or_else(|| home_dir.join(".local/share/peregrin"), PathBuf::from);

        let host = get("PEREGRIN_HOST").unwrap_or_else(|| DEFAULT_HOST.to_string());
        let token = get("PEREGRIN_TOKEN");

        // serve.ts's bind guard, enforced at parse so a bad deployment
        // fails fast at construction rather than mid-request.
        if !is_loopback_host(&host) && token.as_deref().unwrap_or("").is_empty() {
            return Err(EnvError(format!(
                "peregrin-server refuses to bind {host} without PEREGRIN_TOKEN. Set PEREGRIN_TOKEN or bind a loopback address (PEREGRIN_HOST=127.0.0.1)."
            )));
        }

        let node_name = get("PEREGRIN_NAME")
            .filter(|n| !n.trim().is_empty())
            .unwrap_or_else(hostname_or_default);

        // The handoff reuses the node's own bearer when no
        // target-specific token is set — one secret guards the whole
        // ephemeral fleet in the common deployment.
        let snapshot_token = get("PEREGRIN_SNAPSHOT_TOKEN")
            .filter(|t| !t.is_empty())
            .or_else(|| token.clone().filter(|t| !t.is_empty()));

        Ok(Self {
            port,
            host,
            token,
            meta_path: get("PEREGRIN_META_PATH")
                .or_else(|| get("PEREGRIN_META"))
                .map_or_else(|| home.join("meta.json"), PathBuf::from),
            node_path: get("PEREGRIN_NODE").map_or_else(|| home.join("node.json"), PathBuf::from),
            node_name,
            origins,
            cline_dir: get("PEREGRIN_CLINE_DIR")
                .map_or_else(|| home_dir.join(".cline/data"), PathBuf::from),
            claude_dir: get("PEREGRIN_CLAUDE_DIR")
                .map_or_else(|| home_dir.join(".claude"), PathBuf::from),
            sse_keep_alive: Duration::from_millis(ms_or_default(
                get("PEREGRIN_SSE_KEEPALIVE_MS").as_ref(),
                DEFAULT_SSE_KEEPALIVE_MS,
            )),
            held_watch: Duration::from_millis(ms_or_default(
                get("PEREGRIN_HELD_WATCH_MS").as_ref(),
                DEFAULT_HELD_WATCH_MS,
            )),
            idle_ttl: Duration::from_millis(millis_u64(
                get("PEREGRIN_IDLE_TTL_MS").as_ref(),
                DEFAULT_IDLE_TTL_MS,
            )),
            sweep: Duration::from_millis(millis_u64(
                get("PEREGRIN_SWEEP_MS").as_ref(),
                DEFAULT_SWEEP_MS,
            )),
            pair_limit_per_min: count_or_default(
                get("PEREGRIN_PAIR_LIMIT_PER_MIN").as_ref(),
                DEFAULT_PAIR_LIMIT_PER_MIN,
            ),
            pair_limit_per_day: count_or_default(
                get("PEREGRIN_PAIR_LIMIT_PER_DAY").as_ref(),
                DEFAULT_PAIR_LIMIT_PER_DAY,
            ),
            pair_code_path: home.join("pair-code"),
            tokens_path: home.join("tokens.json"),
            kv_path: get("PEREGRIN_KV")
                .or_else(|| get("PEREGRIN_KV_PATH"))
                .map_or_else(|| home.join("kv.json"), PathBuf::from),
            ephemeral: env_flag(&get, "PEREGRIN_EPHEMERAL"),
            snapshot_dir: get("PEREGRIN_SNAPSHOT_DIR")
                .map_or_else(|| home.join("snapshots"), PathBuf::from),
            snapshot_target: get("PEREGRIN_SNAPSHOT_TARGET")
                .map(|t| t.trim_end_matches('/').to_string())
                .filter(|t| !t.is_empty()),
            snapshot_token,
            snapshot_node: get("PEREGRIN_SNAPSHOT_NODE")
                .map(|n| n.trim().to_string())
                .filter(|n| !n.is_empty()),
            runner_labels: get("PEREGRIN_RUNNER_LABELS")
                .map(|v| {
                    v.split(',')
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default(),
            push_backend: match get("PEREGRIN_PUSH_BACKEND")
                .map(|v| v.trim().to_ascii_lowercase())
                .as_deref()
            {
                None | Some("" | "kv") => PushBackendKind::Kv,
                Some("cuora") => PushBackendKind::Cuora,
                Some(other) => {
                    tracing::warn!(
                        "PEREGRIN_PUSH_BACKEND={other:?} unknown — expected \
                         'kv' or 'cuora'; using 'kv'"
                    );
                    PushBackendKind::Kv
                }
            },
            home,
        })
    }

    /// The `PEREGRIN_*` → `HYPHA_*` mapping — builds the substrate
    /// [`hypha_http::Env`] explicitly so `peregrin serve` never asks
    /// the operator for `HYPHA_*` variables. Every field lands where
    /// the old in-process wiring read it:
    ///
    /// | `PEREGRIN_*`            | hypha env field    |
    /// |-------------------------|--------------------|
    /// | `PEREGRIN_HOST`         | `host`             |
    /// | `PEREGRIN_PORT`         | `port`             |
    /// | `PEREGRIN_TOKEN`        | `token`            |
    /// | `PEREGRIN_HOME`         | `home`             |
    /// | `PEREGRIN_NODE`         | `node_path`        |
    /// | `PEREGRIN_NAME`         | `node_name`        |
    /// | `PEREGRIN_ORIGINS`      | `origins`          |
    /// | `PEREGRIN_SSE_KEEPALIVE_MS` | `sse_keep_alive` |
    /// | `PEREGRIN_PAIR_LIMIT_*` | `pair_limit_*`     |
    /// | `<home>/pair-code`      | `pair_code_path`   |
    /// | `<home>/tokens.json`    | `tokens_path`      |
    /// | `<home>/push.json`      | `push_path`        |
    /// | `PEREGRIN_KV[_PATH]`    | `kv_path`          |
    ///
    /// Note the push/pairing *files* are shared with `AppState`'s own
    /// backends at composition — `compose` installs the session
    /// state's instances so there is exactly one of each.
    #[must_use]
    pub fn to_hypha(&self) -> hypha_http::Env {
        hypha_http::Env {
            port: self.port,
            host: self.host.clone(),
            token: self.token.clone(),
            home: self.home.clone(),
            node_path: self.node_path.clone(),
            node_name: self.node_name.clone(),
            origins: self.origins.clone(),
            sse_keep_alive: self.sse_keep_alive,
            pair_limit_per_min: self.pair_limit_per_min,
            pair_limit_per_day: self.pair_limit_per_day,
            pair_code_path: self.pair_code_path.clone(),
            tokens_path: self.tokens_path.clone(),
            push_path: self.home.join("push.json"),
            kv_path: self.kv_path.clone(),
        }
    }
}
