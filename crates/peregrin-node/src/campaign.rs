//! The campaign plane — the orchestration half of the node (the
//! session plane being the other). Composition mirrors the TS
//! `ac-cli-defaults`: a driver registry pre-populated with the ported
//! impls, a `LocalBus` over the memory event store, a memory state
//! store + DLQ, and the sequential `Runner` engine — all in-process.
//!
//! Persistence is deliberately ephemeral for now: every run starts a
//! fresh `MemoryStore` log/state and `MemoryDlq` (the TS `handl run`
//! semantics — the CLI prints the returned log). A durable
//! `ac-store-json` port plus a daemon-side run surface
//! (`/api/campaigns/*`) are later items; nothing here binds the plane
//! to axum, so hub/web can compose the same pieces when that lands.

use std::path::Path;
use std::sync::Arc;

use peregrin_core::{Campaign, Event};
use peregrin_driver::{DriverError, NoopDriver, RunnerDriver};
use peregrin_engine::{Registry, RunOptions, Runner};
use peregrin_loader::{LoadError, Loader};
use peregrin_loader_yaml::YamlLoader;
use peregrin_store::MemoryDlq;

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
}

/// `DefaultDriverRegistryLive` — the drivers a stock node resolves:
/// the four ported `ac-driver-*` impls plus the `runner`/`noop`
/// builtins. `cli`/`polling`/`python`/`stdin`/`streaming`/`webhook`
/// and the litellm/apprise plugins are not ported yet.
#[must_use]
pub fn registry() -> Registry {
    Registry::default()
        .with("agent", Arc::new(peregrin_driver_agent::AgentDriver::new()))
        .with("http", Arc::new(peregrin_driver_http::HttpDriver::new()))
        .with("human", Arc::new(peregrin_driver_human::HumanDriver::new()))
        .with("shell", Arc::new(peregrin_driver_shell::ShellDriver::new()))
        .with("noop", Arc::new(NoopDriver))
        .with("runner", Arc::new(RunnerDriver))
}

/// A runner over [`registry`] with the node defaults — `Runner::new`
/// already wires `LocalBus::memory()` + `MemoryStore` state; the DLQ
/// is upgraded to [`MemoryDlq`] so dead-lettered events are kept for
/// the life of the run instead of dropped (ADR-0002).
#[must_use]
pub fn runner() -> Runner {
    Runner::new(registry()).with_dlq(Box::<MemoryDlq>::default())
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
pub fn run(source: &Path, options: &RunOptions) -> Result<Vec<Event>, CampaignError> {
    let campaign = load(source)?;
    runner().run(&campaign, options).map_err(CampaignError::Run)
}
