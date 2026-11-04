//! `peregrin campaign` — the campaign verbs, ported from the TS
//! `handl` CLI (`apps/cli`). `run` executes a campaign yaml on this
//! machine's campaign plane in-process (`peregrin_node::campaign` —
//! the same composition the daemon hosts); `validate` expands a yaml
//! and summarizes its wiring. The TS `typegen` verb and the
//! otel/metrics observer flags are not ported yet.

use std::collections::BTreeSet;
use std::path::Path;

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
) -> Result<Vec<Event>, CliError> {
    let replay_events = match replay {
        Some(path) => load_replay(path)?,
        None => Vec::new(),
    };
    let options = RunOptions {
        dry_run,
        replay_events,
        ..RunOptions::default()
    };
    let campaign = peregrin_node::campaign::load(workflow).map_err(|e| CliError(e.to_string()))?;
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
            if pattern != "*" && !emit_types.contains(pattern.as_str()) {
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
