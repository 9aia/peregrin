//! `peregrin campaign` — the campaign verbs, ported from the TS
//! `handl` CLI (`apps/cli`). `run` executes a campaign yaml on this
//! machine's campaign plane in-process (`peregrin_node::campaign` —
//! the same composition the daemon hosts); `validate` expands a yaml
//! and summarizes its wiring. The TS `typegen` verb and the
//! otel/metrics observer flags are not ported yet.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use peregrin_core::{Campaign, Event, MaxFires, TriggerMode};
use peregrin_engine::RunOptions;

use crate::CliError;

/// `handl run` — load, execute, print the event log one line per
/// event (`ts source type`), like the TS. `history` swaps the
/// in-memory event log for the durable JSONL store at that path
/// (ADR-0040's `--history` — `peregrin runs`/`explain`/`log` read it
/// back); omitted keeps the ephemeral default. Returns the log so
/// callers (tests, future hub surfaces) can inspect the run.
///
/// # Errors
/// [`CliError`] on replay-log read/parse failure or a campaign-plane
/// failure (loader, engine, store open).
pub fn run(
    workflow: &Path,
    dry_run: bool,
    replay: Option<&Path>,
    history: Option<&Path>,
    queue_store: Option<&Path>,
) -> Result<Vec<Event>, CliError> {
    let replay_events = match replay {
        Some(path) => load_replay(path)?,
        None => Vec::new(),
    };
    let mut options = RunOptions {
        dry_run,
        replay_events,
        ..RunOptions::default()
    };
    // Secret resolution: `RunOptions.secrets` is the explicit record;
    // ambient env resolves next inside `ctx.resolve_secret`, then the
    // fallback stores — OS keyring (declared `capabilities.secrets`
    // names only) and the env file as the last leg
    // (record → env → keyring → file).
    options.secret_stores = peregrin_node::campaign::default_secret_stores();
    let campaign = peregrin_node::campaign::load(workflow).map_err(|e| CliError(e.to_string()))?;
    // Supervisor mode (docs/reference/queue-supersede.md): a declared
    // `supersede:` policy *plus* an ingress-driver node turns
    // `campaign run` into the sequential-run loop the systemd unit
    // wants — the listener nodes persist, every trigger spawns a run
    // over the worker nodes. `dry_run` and `--replay` keep single-run
    // semantics: dry-run can't bind listeners (serve is disabled) and
    // a replay seeds ONE run's log — meaningless across spawned runs.
    if campaign.supersede.is_some()
        && peregrin_node::campaign::has_ingress_nodes(&campaign)
        && !dry_run
        && options.replay_events.is_empty()
    {
        return run_supervised(&campaign, history, queue_store);
    }
    // Ingress variants: `github-webhook`/`sturn-poll` nodes get a
    // run-scoped `Injector`, and a campaign declaring one serves
    // (parks on the channel) instead of exiting at quiet — the
    // systemd unit's daemon mode. Finite campaigns run unchanged.
    let (mut runner, _injector) = match history {
        Some(path) => peregrin_node::campaign::runner_with_log_ingress(path)
            .map_err(|e| CliError(e.to_string()))?,
        None => peregrin_node::campaign::runner_with_ingress(),
    };
    let log = runner
        .run(&campaign, &options)
        .map_err(|e| CliError(e.to_string()))?;
    for event in &log {
        println!("{} {} {}", event.ts, event.source, event.kind);
    }
    Ok(log)
}

/// Supervisor mode for `run` — `peregrin_node::supervisor::supervise`
/// drives the campaign: persistent ingress listeners feed the queue,
/// each trigger spawns a run over the worker nodes, and each finished
/// run's log prints (and appends to `history`, which every spawned
/// run shares — `peregrin runs`/`log`/`explain` read it back). Blocks
/// until the trigger channel closes — the ingress listeners exiting —
/// and the queue drains; returns the logs concatenated for callers.
fn run_supervised(
    campaign: &Campaign,
    history: Option<&Path>,
    queue_store: Option<&Path>,
) -> Result<Vec<Event>, CliError> {
    // `--queue-store` opens the sqlite durable queue — pending
    // triggers + the in-flight marker replay on the next boot
    // (docs/reference/queue-supersede.md). Open happens here so a bad
    // path fails fast, before any listener binds.
    let store = queue_store
        .map(|path| {
            peregrin_queue_sqlite::SqliteQueueStore::open(path).map(|store| {
                std::sync::Arc::new(store) as std::sync::Arc<dyn peregrin_queue::QueueStore>
            })
        })
        .transpose()
        .map_err(|e| CliError(format!("failed to open queue store: {e}")))?;
    let options = peregrin_node::supervisor::SupervisedOptions {
        history: history.map(Path::to_path_buf),
        queue_store: store,
        on_run: Some(Box::new(|log: &[Event]| {
            for event in log {
                println!("{} {} {}", event.ts, event.source, event.kind);
            }
        })),
        ..peregrin_node::supervisor::SupervisedOptions::default()
    };
    let run = peregrin_node::supervisor::supervise(campaign, options);
    // Only the listeners feed the queue — drop our producer so a dead
    // listener disconnects the channel and the supervisor drains and
    // exits (a parked gate that can never see a trigger must not look
    // alive; systemd restarts the unit).
    drop(run.injector);
    run.join
        .join()
        .map_err(|_| CliError("campaign supervisor thread panicked".to_string()))?
        .map_err(|e| CliError(e.to_string()))
}

/// `peregrin campaign serve` — the multi-campaign daemon (PL-007
/// C1b): load every campaign yaml under `paths` (each arg a yaml file
/// or a directory of `*.yaml`/`*.yml`), host one `RunSupervisor`
/// per campaign via [`peregrin_node::serve::serve_campaigns`], and
/// share ONE `github-webhook` listener demuxing verified pushes to
/// each campaign by `repository.full_name` (all-match — see
/// `peregrin_node::serve`). Blocks on `rt`'s SIGINT/SIGTERM wait,
/// then drains every supervisor and exits.
///
/// `history_dir`/`queue_store_dir` give each campaign its own
/// `<name>.jsonl`/`<name>.db` so durable state never crosses
/// campaigns (`campaign run`'s `--history`/`--queue-store` flags,
/// namespaced).
///
/// # Errors
/// [`CliError`] on path expansion, yaml load, duplicate campaign
/// names, listener bind failure, or a campaign's supervisor error
/// during drain.
pub fn serve(
    paths: &[PathBuf],
    listen: &str,
    path: &str,
    history_dir: Option<&Path>,
    queue_store_dir: Option<&Path>,
    rt: &tokio::runtime::Runtime,
) -> Result<(), CliError> {
    let files = expand_campaign_paths(paths)?;
    // `~` expands here (not in the unit's ExecStart — systemd argv is
    // data, no shell) so provider manifests can write portable paths.
    let history_dir = history_dir.map(tilde_path);
    let queue_store_dir = queue_store_dir.map(tilde_path);
    let mut hosted = Vec::with_capacity(files.len());
    for file in &files {
        let campaign = peregrin_node::campaign::load(file).map_err(|e| CliError(e.to_string()))?;
        hosted.push(peregrin_node::serve::HostedCampaign {
            name: peregrin_node::serve::campaign_name(&campaign, file),
            campaign,
            source: file.clone(),
        });
    }
    let config = peregrin_node::serve::ServeConfig {
        listen: listen.to_string(),
        path: path.to_string(),
        history_dir,
        queue_store_dir,
        on_run: Some(std::sync::Arc::new(|name: &str, log: &[Event]| {
            for event in log {
                println!("[{name}] {} {} {}", event.ts, event.source, event.kind);
            }
        })),
        secrets: BTreeMap::new(),
        // Fleet labels (`PEREGRIN_RUNNER_LABELS`) gate `runs_on`
        // campaigns — same env the node daemon advertises on
        // `/api/node`, so `campaign serve` and `peregrin serve`
        // agree on eligibility.
        labels: std::env::var("PEREGRIN_RUNNER_LABELS")
            .map(|v| {
                v.split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default(),
    };
    let served = peregrin_node::serve::serve_campaigns(hosted, &config)
        .map_err(|e| CliError(e.to_string()))?;
    match served.listener {
        Some(addr) => println!(
            "campaign serve: {} campaigns, webhook listener on http://{addr}{path}",
            served.campaigns.len()
        ),
        None => println!(
            "campaign serve: {} campaigns (no github-webhook nodes — no listener)",
            served.campaigns.len()
        ),
    }
    rt.block_on(peregrin_node::shutdown_signal());
    served.join().map_err(|e| CliError(e.to_string()))
}

/// `~`/`~/…` expansion for a CLI path arg — `peregrin_config`'s
/// convention (`~user` and plain paths pass through).
fn tilde_path(path: &Path) -> PathBuf {
    PathBuf::from(peregrin_config::expand_tilde(&path.to_string_lossy()))
}

/// Expand `campaign serve` path args into yaml files — a file passes
/// through, a directory contributes its top-level `*.yaml`/`*.yml`
/// entries (sorted for deterministic order). Nested dirs and other
/// extensions are ignored. `~` expands per arg so provider-rendered
/// units can carry portable paths.
fn expand_campaign_paths(paths: &[PathBuf]) -> Result<Vec<PathBuf>, CliError> {
    let mut files = Vec::new();
    for path in paths {
        let path = tilde_path(path);
        let path = path.as_path();
        if path.is_dir() {
            let mut entries: Vec<PathBuf> = std::fs::read_dir(path)
                .map_err(|e| CliError(format!("read {}: {e}", path.display())))?
                .filter_map(std::result::Result::ok)
                .map(|entry| entry.path())
                .filter(|p| {
                    p.extension()
                        .and_then(|e| e.to_str())
                        .is_some_and(|e| matches!(e, "yaml" | "yml"))
                })
                .collect();
            entries.sort();
            files.extend(entries);
        } else if path.is_file() {
            files.push(path.to_path_buf());
        } else {
            return Err(CliError(format!(
                "campaign path {} does not exist",
                path.display()
            )));
        }
    }
    if files.is_empty() {
        return Err(CliError(
            "no campaign yamls found — pass yaml files or directories of *.yaml/*.yml".to_string(),
        ));
    }
    Ok(files)
}

/// `handl validate` — expand the yaml and print the summary plus
/// wiring warnings; the lines are returned for tests.
///
/// # Errors
/// [`CliError`] on loader failure.
pub fn validate(workflow: &Path) -> Result<Vec<String>, CliError> {
    let campaign = peregrin_node::campaign::load(workflow).map_err(|e| CliError(e.to_string()))?;
    let lines = validate_report(&campaign);
    for line in &lines {
        println!("{line}");
    }
    Ok(lines)
}

/// `loadReplayEvents` — a JSON array of events or one JSON event per
/// line (JSONL).
fn load_replay(path: &Path) -> Result<Vec<Event>, CliError> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        CliError(format!(
            "Failed to read replay log: {}: {e}",
            path.display()
        ))
    })?;
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    let parse_error = |e: serde_json::Error| CliError(format!("Failed to parse replay log: {e}"));
    if trimmed.starts_with('[') {
        return serde_json::from_str(trimmed).map_err(parse_error);
    }
    trimmed
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).map_err(parse_error))
        .collect()
}

/// The `validate` report — pure so tests can assert on the lines.
/// Faithful to the TS `validate` handler: summary, per-node detail,
/// then a warning per `on` pattern nothing emits.
fn validate_report(campaign: &Campaign) -> Vec<String> {
    let mut out = Vec::new();
    let name = if campaign.name.is_empty() {
        "(unnamed)"
    } else {
        campaign.name.as_str()
    };
    out.push(format!("workflow: {name}"));
    out.push(format!("  nodes:  {}", campaign.nodes.len()));
    out.push(format!("  events: {}", campaign.events.len()));
    if !campaign.imports.is_empty() {
        out.push(format!("  includes: {}", campaign.imports.join(", ")));
    }
    if !campaign.stop_on.is_empty() {
        out.push(format!("  stop_on: {}", campaign.stop_on.join(", ")));
    }
    if let Some(supersede) = &campaign.supersede {
        let mode = match supersede.mode {
            peregrin_core::SupersedeMode::LatestWins => "latest-wins",
            peregrin_core::SupersedeMode::Fifo => "fifo",
        };
        out.push(format!(
            "  supersede: key=\"{}\" mode={mode} cancel_running={}",
            supersede.key, supersede.cancel_running
        ));
    }
    for node in &campaign.nodes {
        let max_fires = if node.max_fires == MaxFires::Fires(1) {
            String::new()
        } else {
            format!(" max_fires={}", max_fires_display(node.max_fires))
        };
        let until = if node.until.is_empty() {
            String::new()
        } else {
            format!(" until={}", node.until.join(","))
        };
        let retire = if node.retire_on.is_empty() {
            String::new()
        } else {
            format!(" retire_on={}", node.retire_on.join(","))
        };
        let poll = node.poll.as_ref().map_or_else(String::new, |p| {
            format!(" poll={}", serde_json::Value::Object(p.clone()))
        });
        let mode = match node.mode {
            TriggerMode::Any => "any",
            TriggerMode::All => "all",
        };
        out.push(format!(
            "    [{}] driver={} handler={} on={} emits={} mode={mode}{max_fires}{until}{retire}{poll}",
            node.id,
            node.driver,
            node.handler,
            serde_json::Value::Array(
                node.on.iter().cloned().map(serde_json::Value::String).collect()
            ),
            serde_json::Value::Array(
                node.emits
                    .iter()
                    .cloned()
                    .map(serde_json::Value::String)
                    .collect()
            ),
        ));
    }
    // Every event type anything can emit — node emits, declared
    // events, and the engine-synthetic set the TS hardcodes.
    let mut emit_types: BTreeSet<&str> = BTreeSet::new();
    for node in &campaign.nodes {
        for emit in &node.emits {
            emit_types.insert(emit.as_str());
        }
    }
    for schema in &campaign.events {
        emit_types.insert(schema.kind.as_str());
    }
    for kind in [
        "start",
        "node.failed",
        "node.exhausted",
        "workflow.done",
        "workflow.failed",
        "workflow.cancel",
        "workflow.cancelled",
        "workflow.cycle_detected",
        "event.invalid",
        "dlq.event",
    ] {
        emit_types.insert(kind);
    }
    for node in &campaign.nodes {
        for pattern in &node.on {
            let satisfied = pattern == "*"
                || emit_types
                    .iter()
                    .any(|emit| peregrin_core::subscribes_to(pattern, emit));
            if !satisfied {
                out.push(format!(
                    "  WARNING: node '{}' subscribes to '{pattern}' but no node emits it",
                    node.id
                ));
            }
        }
    }
    out.push("OK".to_string());
    out
}

/// TS `maxFires` rendering — a count or `"unlimited"`.
fn max_fires_display(max_fires: MaxFires) -> String {
    match max_fires {
        MaxFires::Fires(n) => n.to_string(),
        MaxFires::Unlimited => "unlimited".to_string(),
    }
}
