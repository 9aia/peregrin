//! The campaign plane — the orchestration half of the node (the
//! session plane being the other). Composition mirrors the TS
//! `ac-cli-defaults`: a driver registry pre-populated with the ported
//! impls, a `LocalBus` over the memory event store, a memory state
//! store + DLQ, and the sequential `Runner` engine — all in-process.
//!
//! Persistence defaults to ephemeral — [`run`] starts every run on a
//! fresh `MemoryStore` log/state and `MemoryDlq` (the TS `handl run`
//! semantics — the CLI prints the returned log). [`runner_with_log`]
//! swaps the event log for the durable `peregrin-store-json` JSONL
//! store (the `ac-store-json` port); a daemon-side run surface
//! (`/api/campaigns/*`) and store selection via config are later
//! items. Nothing here binds the plane to axum, so hub/web can
//! compose the same pieces when that lands.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use peregrin_bus::{LocalBus, NoopObserver};
use peregrin_core::{Campaign, Event};
use peregrin_driver::{DriverError, NoopDriver, RunnerDriver, SecretSource};
use peregrin_engine::{Injector, Registry, RunOptions, Runner, ingress};
use peregrin_loader::{LoadError, Loader};
use peregrin_loader_yaml::YamlLoader;
use peregrin_store::MemoryDlq;
use peregrin_store_json::JsonStoreError;

/// Campaign-plane failure — the two error channels a run crosses:
/// the loader rejecting the source, then the engine failing the run.
#[derive(Debug, thiserror::Error)]
pub enum CampaignError {
    /// Yaml parse, include/snippet resolution, discovery, or model
    /// normalization failed (TS `LoaderError`).
    #[error("{0}")]
    Load(#[from] LoadError),
    /// Strict preflight, an unresolvable driver, or a template failure
    /// (TS `DriverError`).
    #[error("{0}")]
    Run(#[from] DriverError),
    /// Durable event-log open failed (TS `JsonStoreError`) — only
    /// reachable via [`runner_with_log`].
    #[error("{0}")]
    Store(#[from] JsonStoreError),
}

/// `DefaultDriverRegistryLive` — the drivers a stock node resolves:
/// the four ported `ac-driver-*` impls, the `github-status` egress
/// sink, the `delay`/`schedule` temporal drivers (ADR-0018), the
/// `subworkflow` composition driver (ADR-0022), plus the
/// `runner`/`noop` builtins. `cli`/`polling`/`python`/`stdin`/
/// `streaming`/`webhook` and the litellm/apprise plugins are not
/// ported yet.
#[must_use]
pub fn registry() -> Registry {
    let base = Registry::default()
        .with("agent", Arc::new(peregrin_driver_agent::AgentDriver::new()))
        .with("http", Arc::new(peregrin_driver_http::HttpDriver::new()))
        .with("human", Arc::new(peregrin_driver_human::HumanDriver::new()))
        .with(
            "delay",
            Arc::new(peregrin_driver_schedule::DelayDriver::new()),
        )
        .with(
            "schedule",
            Arc::new(peregrin_driver_schedule::ScheduleDriver::new()),
        )
        .with(
            "mock-agent",
            Arc::new(peregrin_driver_agent::MockAgentDriver::new()),
        )
        .with("shell", Arc::new(peregrin_driver_shell::ShellDriver::new()))
        .with(
            "subworkflow",
            Arc::new(peregrin_driver_subworkflow::SubworkflowDriver::new()),
        )
        .with(
            "github-status",
            Arc::new(peregrin_sink_github_status::GithubStatusSink::new()),
        )
        .with("noop", Arc::new(NoopDriver))
        .with("runner", Arc::new(RunnerDriver));
    let inner = base.clone();
    // `git` resolves inner drivers against the base set — an inner
    // `driver: git` is deliberately not supported.
    base.with(
        "git",
        Arc::new(peregrin_driver_git::GitDriver::new(move |name: &str| {
            inner.resolve(name).ok()
        })),
    )
}

/// A runner over [`registry`] with the node defaults — `Runner::new`
/// already wires `LocalBus::memory()` + `MemoryStore` state; the DLQ
/// is upgraded to [`MemoryDlq`] so dead-lettered events are kept for
/// the life of the run instead of dropped (ADR-0002).
#[must_use]
pub fn runner() -> Runner {
    Runner::new(registry()).with_dlq(Box::<MemoryDlq>::default())
}

/// A runner like [`runner`] but with a live ingress pair: the
/// run-scoped ingress drivers (`github-webhook`, `sturn-poll`) are
/// registered bound to the returned `Injector`, and injected
/// events drain into the loop each step. Keep the `Injector` to
/// feed the run from outside (supervisor queue, daemon triggers,
/// `Event::cancel` from supersede).
#[must_use]
pub fn runner_with_ingress() -> (Runner, Injector) {
    let (injector, ingress) = ingress();
    let registry = registry()
        .with(
            "github-webhook",
            Arc::new(peregrin_driver_github_webhook::GithubWebhookDriver::new(
                injector.clone(),
            )),
        )
        .with(
            "sturn-poll",
            Arc::new(peregrin_driver_sturn::SturnDriver::new(injector.clone())),
        );
    let runner = Runner::new(registry)
        .with_dlq(Box::<MemoryDlq>::default())
        .with_ingress(ingress);
    (runner, injector)
}

/// A runner like [`runner_with_ingress`] but with the durable JSONL
/// log at `log_path` too.
///
/// # Errors
/// [`CampaignError::Store`] when the log can't be read or a line
/// fails to parse.
pub fn runner_with_log_ingress(log_path: &Path) -> Result<(Runner, Injector), CampaignError> {
    let (runner, injector) = runner_with_ingress();
    let store = peregrin_store_json::JsonStore::open(log_path)?;
    let bus = LocalBus::new(Box::new(store), Arc::new(NoopObserver));
    Ok((runner.with_bus(Box::new(bus)), injector))
}

/// A runner like [`runner`] but with the event log backed by the
/// durable JSONL store at `log_path` (`ac-store-json` semantics): the
/// log survives process exit, and a store reopened on the same path
/// resumes the run's publish/first-seen order. Run state stays
/// in-memory — the `JsonStateStore` half of `ac-store-json` is a
/// later item (ADR-0013).
///
/// # Errors
/// [`CampaignError::Store`] when the log can't be read or a line
/// fails to parse.
pub fn runner_with_log(log_path: &Path) -> Result<Runner, CampaignError> {
    let store = peregrin_store_json::JsonStore::open(log_path)?;
    let bus = LocalBus::new(Box::new(store), Arc::new(NoopObserver));
    Ok(runner().with_bus(Box::new(bus)))
}

/// TS `loadWorkflow` — yaml file → [`Campaign`], with `include:`/
/// `snippet:` expansion and `.handl/` discovery anchored at the
/// file's directory.
///
/// # Errors
/// [`CampaignError::Load`] on read, parse, expansion, or model
/// normalization failure.
pub fn load(source: &Path) -> Result<Campaign, CampaignError> {
    Ok(YamlLoader.load(source)?)
}

/// TS `handl run <workflow>` — load the campaign file and execute it
/// on a fresh [`runner`]. Returns the run's event log; the last entry
/// is the terminal event (`workflow.done`, `workflow.failed`, or
/// `workflow.cancelled`). The runner is synchronous — callers on an
/// async surface should `spawn_blocking`.
///
/// # Errors
/// [`CampaignError::Load`] for loader failures,
/// [`CampaignError::Run`] for engine failures.
/// The run's secrets env file — `XDG_CONFIG_HOME/peregrin/env` →
/// `~/.config/peregrin/env` (`PEREGRIN_ENV` overrides). `create`
/// seeds this with the secret NAMES the operator fills in.
#[must_use]
pub fn env_file() -> PathBuf {
    if let Some(p) = std::env::var_os("PEREGRIN_ENV") {
        return PathBuf::from(p);
    }
    std::env::var_os("XDG_CONFIG_HOME")
        .map(|d| PathBuf::from(d).join("peregrin/env"))
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config/peregrin/env")))
        .unwrap_or_else(|| PathBuf::from(".config/peregrin/env"))
}

/// `KEY=VAL` file → secrets record. Absent/malformed lines are
/// skipped. Callers that want the file folded *into* the record
/// (explicit `RunOptions.secrets` shape) use this; the stock chain
/// keeps the file as a fallback store instead ([`default_secret_stores`])
/// so the documented `record → env → keyring → file` precedence holds.
#[must_use]
pub fn secrets_from_env_file(path: &Path) -> BTreeMap<String, String> {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter_map(|l| l.split_once('='))
        .filter(|(k, v)| !k.is_empty() && !v.is_empty() && !k.starts_with('#'))
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

/// The default fallback secret stores for `RunOptions.secret_stores`
/// — the tail of the documented `record → env → keyring → file`
/// chain (`ExecutionContext::resolve_secret` runs the record and the
/// ambient environment first; these legs only see names that missed
/// both):
///
/// 1. **OS keyring** (`secret-tool`, service `peregrin`) —
///    `declared_only`: a lookup spawns a process and reaches into OS
///    credentials, so only names the merged `capabilities.secrets`
///    grant declares are ever looked up.
/// 2. **The env file** (`~/.config/peregrin/env`, `PEREGRIN_ENV`
///    override) — the always-available last leg.
///
/// Values are only ever returned into the run's ctx — names, never
/// values, cross into events or logs.
#[must_use]
pub fn default_secret_stores() -> Vec<SecretSource> {
    vec![
        SecretSource {
            store: std::rc::Rc::new(peregrin_keyring::KeyringSecrets::default()),
            declared_only: true,
        },
        SecretSource {
            store: std::rc::Rc::new(peregrin_dotenv::DotenvSecrets { path: env_file() }),
            declared_only: false,
        },
    ]
}

/// TS `handl run <workflow>` — load the campaign file and execute it
/// on a fresh [`runner`]. Returns the run's event log; the last entry
/// is the terminal event (`workflow.done`, `workflow.failed`, or
/// `workflow.cancelled`). The runner is synchronous — callers on an
/// async surface should `spawn_blocking`.
///
/// # Errors
/// [`CampaignError::Load`] for loader failures,
/// [`CampaignError::Run`] for engine failures.
pub fn run(source: &Path, options: &RunOptions) -> Result<Vec<Event>, CampaignError> {
    let campaign = load(source)?;
    let mut options = RunOptions {
        dry_run: options.dry_run,
        secrets: options.secrets.clone(),
        secret_stores: options.secret_stores.clone(),
        replay_events: options.replay_events.clone(),
    };
    // The keyring + env-file legs fill in when the caller didn't
    // wire its own fallback chain — explicit stores win wholesale.
    if options.secret_stores.is_empty() {
        options.secret_stores = default_secret_stores();
    }
    let (mut runner, _injector) = runner_with_ingress();
    runner.run(&campaign, &options).map_err(CampaignError::Run)
}
