//! `RunResult` — the machine-readable verdict document a finished
//! campaign run projects into (`campaign run --result`, or the
//! `<history>.result.json` sibling `--history` implies). CI gates
//! read this instead of scraping the `ts source type` event log.
//!
//! The document follows the GitHub Checks model: **`status`** is the
//! lifecycle (`running` | `completed` | `cancelled`) and
//! **`conclusion`** is the canonical outcome
//! (`success` | `failure` | `neutral` | `skipped` | `timed_out` |
//! `cancelled` | `action_required`, [`peregrin_core::Conclusion`]).
//! `verdict` stays as the campaign's own coarse label
//! (`pass` | `fail` | `error`) — `error` has no GH-Checks spelling
//! (the REST enum tops out at `action_required`; `startup_failure`
//! is GraphQL-only), so an engine/loader failure maps to
//! `conclusion: "failure"` while `verdict: "error"` preserves the
//! campaign-plane distinction.
//!
//! The projection is a *view over existing state* — the run's event
//! log plus the runner's per-node fire accounting — never a parallel
//! tracker. [`RunResult::from_log`] needs only the log (statuses,
//! `rc`, timestamps derive from it); [`Runner::run_result`] adds
//! `secs`/`fires` from the fire stats the walker already measures.
//!
//! Conclusion semantics:
//!
//! - A campaign's `result:` rules ([`Campaign::result`]) declare
//!   event→conclusion mappings. Scanning the log in order, **the
//!   last decisive event wins** — a declared event asserts its rule's
//!   conclusion, an undeclared event falls back to the heuristic
//!   (`*.failed` segment, `circuit.open`, nonzero `payload.rc`,
//!   `workflow.cancelled`). Later events supersede earlier ones, so
//!   `gate.fail` then `gate.pass` concludes `success`.
//! - `cancelled` dominates: a `workflow.cancelled` terminator sets
//!   both `status: "cancelled"` and `conclusion: "cancelled"`.
//! - `verdict`: `error` only via [`RunResult::error`] /
//!   [`crate::Runner::run_result_error`] (campaign-plane `Err`);
//!   otherwise `pass` for `success`/`neutral`/`skipped` conclusions,
//!   `fail` for `failure`/`timed_out`/`action_required`/`cancelled`.
//!
//! `campaign run`'s process exit code stays `0` on a *completed* run
//! regardless of verdict — the artifact is the contract.

use std::collections::{BTreeSet, HashMap};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use peregrin_core::{Campaign, Conclusion, Event};

/// The document schema tag — bump on any shape change.
pub const RUN_RESULT_SCHEMA: &str = "peregrin/run-result/1";

/// Per-node fire accounting the runner accumulates while walking —
/// `fires` counts `execute_node` invocations, `secs` their summed
/// wall time. Crate-internal: only [`crate::Runner::run_result`]
/// consumes it.
#[derive(Default)]
pub(crate) struct NodeFireStats {
    /// How many times the node fired this run.
    pub fires: u64,
    /// Summed wall-clock seconds inside `execute_node`.
    pub secs: f64,
}

/// The run verdict — the campaign's own coarse label, serialized
/// lowercase (`pass`/`fail`/`error`). See the module docs for the
/// `conclusion` mapping (`error` → `failure` — GH Checks has no
/// error conclusion).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RunVerdict {
    /// The run completed with a non-failing conclusion.
    Pass,
    /// A node failed, a declared/heuristic failure event fired, or
    /// the run was cancelled.
    Fail,
    /// Campaign-plane failure — load/preflight/engine `Err`; the
    /// document may carry a partial projection.
    Error,
}

/// Run lifecycle — serialized lowercase. The artifact is only written
/// at run end, so `running` never appears in practice; the variant
/// keeps the vocabulary GH-complete for consumers that reuse the type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RunStatus {
    /// In flight — not emitted by the current writers.
    Running,
    /// The run reached a terminal state (any conclusion).
    Completed,
    /// The run was cancelled before finishing.
    Cancelled,
}

/// Node lifecycle — serialized lowercase. Like [`RunStatus`], a
/// finished run only ever reports `completed`/`pending`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeStatus {
    /// The node fired at least once and every fire returned.
    Completed,
    /// Declared (or seen in stats) but never fired.
    Pending,
}

/// The GH Checks `output` block — markdown-friendly surfaces. `title`
/// is the campaign name; `summary`/`text` ship empty today — campaigns
/// can populate them later via a convention (the seam is documented,
/// the filling machinery is deliberately not built yet).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RunOutput {
    /// Short headline — the campaign name.
    pub title: String,
    /// Markdown summary — empty until the filling convention lands.
    pub summary: String,
    /// Markdown body — empty until the filling convention lands.
    pub text: String,
}

/// One node's outcome — serialized fields are all present except
/// `fires`/`secs`/`rc`/`conclusion`, which are omitted when unknown
/// (a log-only projection has no timing; a pending node has no
/// conclusion).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NodeResult {
    /// The node id (declared `nodes[].id`, or an injected node's).
    pub id: String,
    /// Lifecycle: `completed` (fired ≥once) | `pending` (never fired).
    pub status: NodeStatus,
    /// Outcome in the GH Checks vocabulary — `success`/`failure`/
    /// `skipped`/… — absent while `status` is `pending`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conclusion: Option<Conclusion>,
    /// Fire count — present only on runner-projected results.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fires: Option<u64>,
    /// Summed fire wall-time in seconds (millisecond-rounded) —
    /// present only on runner-projected results.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secs: Option<f64>,
    /// The last numeric `payload.rc` the node emitted (the shell
    /// driver's exit-code contract) — `0` on success paths that
    /// carry it, nonzero/`-1` on failures.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rc: Option<i64>,
}

/// The run-result document — one JSON object per finished run.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RunResult {
    /// Schema tag — `"peregrin/run-result/1"`.
    pub schema: String,
    /// Stable per-run identifier — FNV-1a over
    /// `campaign|first-event-ts|first-event-id`, so the same run's
    /// document reads identically wherever it's consumed.
    pub run_id: String,
    /// Campaign name (`campaign.name`; `"(unnamed)"` when unset).
    pub campaign: String,
    /// Lifecycle: `completed` | `cancelled` (`running` unused — the
    /// document is only written at run end).
    pub status: RunStatus,
    /// The campaign's own verdict label: `pass` | `fail` | `error`.
    pub verdict: RunVerdict,
    /// Canonical GH Checks conclusion.
    pub conclusion: Conclusion,
    /// RFC3339 run start — the first log event's `ts`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    /// RFC3339 run end — the last log event's `ts`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
    /// GH Checks `output` block — see [`RunOutput`].
    pub output: RunOutput,
    /// Per-node outcomes — campaign declaration order, then any
    /// injected/extra ids sorted.
    pub nodes: Vec<NodeResult>,
    /// Event kinds that forced a non-`pass` verdict, deduplicated
    /// in first-seen log order.
    pub failed_events: Vec<String>,
}

/// `*.<failed>` terminal segment — `node.failed`, `workflow.failed`,
/// `verify.report.failed`, … — the same convention `ops::run_status`
/// reads off routed event types.
fn kind_is_failed(kind: &str) -> bool {
    kind == "failed" || kind.ends_with(".failed")
}

/// Event kinds that are node/run *meta* rather than work product —
/// a node emitting only these concludes `skipped`, not `success`.
// Event kinds aren't file paths — the `.tick` suffix check trips
// clippy's extension lint.
#[allow(clippy::case_sensitive_file_extension_comparisons)]
fn kind_is_meta(kind: &str) -> bool {
    matches!(kind, "node.skipped" | "node.exhausted") || kind.ends_with(".tick")
}

/// What one event contributes to the run conclusion — `Decisive`
/// carries the conclusion it asserts (declared rule or heuristic),
/// `Neutral` events leave the running conclusion untouched.
enum EventOutcome {
    /// This event asserts a conclusion (declared rule or heuristic
    /// failure/cancel).
    Decisive(Conclusion),
    /// No conclusion contribution.
    Neutral,
}

/// The heuristic half of [`event_outcome`] — the `*.failed` segment,
/// `circuit.open`, and nonzero numeric `payload.rc` checks that apply
/// to events with no `result:` rule.
fn heuristic_outcome(event: &Event) -> EventOutcome {
    if kind_is_failed(&event.kind) || event.kind == "circuit.open" {
        return EventOutcome::Decisive(Conclusion::Failure);
    }
    if event.kind == "workflow.cancelled" {
        return EventOutcome::Decisive(Conclusion::Cancelled);
    }
    // A nonzero numeric `rc` is a failure even when an `emit` map
    // routed it onto a neutral kind (`emit: {"1": "verify.report"}`).
    if event
        .payload
        .get("rc")
        .and_then(Value::as_f64)
        .is_some_and(|rc| rc != 0.0)
    {
        return EventOutcome::Decisive(Conclusion::Failure);
    }
    EventOutcome::Neutral
}

/// One event's conclusion contribution — a declared `result:` rule
/// wins over the heuristic for that event kind.
fn event_outcome<'a>(rules: &'a HashMap<&'a str, Conclusion>, event: &Event) -> EventOutcome {
    if let Some(conclusion) = rules.get(event.kind.as_str()) {
        return EventOutcome::Decisive(*conclusion);
    }
    heuristic_outcome(event)
}

/// The last numeric `payload.rc` an event carries, if any.
fn event_rc(event: &Event) -> Option<i64> {
    event.payload.get("rc").and_then(Value::as_i64).or_else(|| {
        event
            .payload
            .get("rc")
            .and_then(Value::as_f64)
            .map(|rc| rc as i64)
    })
}

/// Conclusions that keep `verdict` at `pass` — GH `neutral`/`skipped`
/// are non-blocking outcomes.
fn conclusion_passes(conclusion: Conclusion) -> bool {
    matches!(
        conclusion,
        Conclusion::Success | Conclusion::Neutral | Conclusion::Skipped
    )
}

/// RFC3339 from epoch milliseconds — empty string on out-of-range.
fn rfc3339(ts_ms: i64) -> Option<String> {
    chrono::DateTime::from_timestamp_millis(ts_ms).map(|dt| dt.to_rfc3339())
}

/// FNV-1a 64-bit — the `run_id` hash (no new deps for a stable id).
fn fnv1a(parts: &[&str]) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for part in parts {
        for byte in part.bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        hash ^= 0xff;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

/// The `run_id` — deterministic per run: campaign name + the log's
/// first event timestamp and id.
fn run_id(campaign_name: &str, log: &[Event]) -> String {
    let first = log.first();
    fnv1a(&[
        campaign_name,
        &first.map_or(0, |e| e.ts).to_string(),
        first.map_or("", |e| e.id.as_str()),
    ])
}

/// Per-node lifecycle + conclusion from the events it sourced.
fn node_outcome(
    rules: &HashMap<&str, Conclusion>,
    events: &[&Event],
) -> (NodeStatus, Option<Conclusion>) {
    if events.is_empty() {
        return (NodeStatus::Pending, None);
    }
    // Last decisive event wins — same rule as the run conclusion.
    let mut decisive: Option<Conclusion> = None;
    for event in events {
        if let EventOutcome::Decisive(conclusion) = event_outcome(rules, event) {
            decisive = Some(conclusion);
        }
    }
    if let Some(conclusion) = decisive {
        return (NodeStatus::Completed, Some(conclusion));
    }
    // No decisive event: all-meta → skipped, otherwise success.
    let conclusion = if events.iter().all(|e| kind_is_meta(&e.kind)) {
        Conclusion::Skipped
    } else {
        Conclusion::Success
    };
    (NodeStatus::Completed, Some(conclusion))
}

impl RunResult {
    /// Project a finished run's event log into the result document —
    /// no fire stats, so `secs`/`fires` stay absent. This is the
    /// supervisor-mode and log-only path; [`crate::Runner::run_result`]
    /// is the richer single-run projection.
    #[must_use]
    pub fn from_log(campaign: &Campaign, log: &[Event]) -> Self {
        Self::project(campaign, log, None)
    }

    /// A bare `error` document — campaign-plane failures before a
    /// run exists (loader rejection, bad `--history` path) still
    /// leave CI a machine-readable verdict. `campaign` is the best
    /// available label (the yaml's `name:` when known, else the
    /// workflow file stem). GH Checks has no `error` conclusion —
    /// `failure` is the closest honest mapping; `verdict: "error"`
    /// preserves the distinction.
    #[must_use]
    pub fn error(campaign: &str) -> Self {
        Self {
            schema: RUN_RESULT_SCHEMA.to_string(),
            run_id: fnv1a(&[campaign, "0", ""]),
            campaign: campaign.to_string(),
            status: RunStatus::Completed,
            verdict: RunVerdict::Error,
            conclusion: Conclusion::Failure,
            started_at: None,
            completed_at: None,
            output: RunOutput {
                title: campaign.to_string(),
                summary: String::new(),
                text: String::new(),
            },
            nodes: Vec::new(),
            failed_events: Vec::new(),
        }
    }

    /// The shared projection — `stats` (when the caller has the
    /// runner's fire accounting) fills `secs`/`fires`.
    pub(crate) fn project(
        campaign: &Campaign,
        log: &[Event],
        stats: Option<&HashMap<String, NodeFireStats>>,
    ) -> Self {
        let campaign_name = if campaign.name.is_empty() {
            "(unnamed)"
        } else {
            campaign.name.as_str()
        };
        // Declared `result:` rules, keyed by event kind for the
        // per-event lookup (duplicate kinds: last rule wins).
        let mut rules: HashMap<&str, Conclusion> = HashMap::new();
        for rule in &campaign.result {
            rules.insert(rule.event.as_str(), rule.conclusion);
        }

        // Run conclusion — last decisive event in log order wins.
        let mut conclusion = Conclusion::Success;
        let cancelled = log.iter().any(|e| e.kind == "workflow.cancelled");
        for event in log {
            if let EventOutcome::Decisive(c) = event_outcome(&rules, event) {
                conclusion = c;
            }
        }
        // `workflow.cancelled` is decisive via the heuristic arm, but
        // belt-and-braces: an explicit cancel always concludes
        // `cancelled` + lifecycle `cancelled`.
        if cancelled {
            conclusion = Conclusion::Cancelled;
        }
        let status = if cancelled {
            RunStatus::Cancelled
        } else {
            RunStatus::Completed
        };
        let verdict = if conclusion_passes(conclusion) {
            RunVerdict::Pass
        } else {
            RunVerdict::Fail
        };

        // Node ids in report order: campaign declaration order, then
        // any extra ids the log/stats saw (mid-run `inject` nodes)
        // sorted for determinism.
        let declared: BTreeSet<&str> = campaign.nodes.iter().map(|n| n.id.as_str()).collect();
        let mut extra: BTreeSet<String> = BTreeSet::new();
        for event in log {
            if event.source != "__runner__" && !event.source.is_empty() {
                extra.insert(event.source.clone());
            }
        }
        if let Some(stats) = stats {
            for id in stats.keys() {
                extra.insert(id.clone());
            }
        }
        extra.retain(|id| !declared.contains(id.as_str()));

        let mut nodes = Vec::with_capacity(campaign.nodes.len() + extra.len());
        for id in campaign
            .nodes
            .iter()
            .map(|n| n.id.as_str())
            .chain(extra.iter().map(String::as_str))
        {
            let events: Vec<&Event> = log.iter().filter(|e| e.source == id).collect();
            let (status, conclusion) = node_outcome(&rules, &events);
            let rc = events.iter().rev().find_map(|e| event_rc(e));
            let stat = stats.and_then(|s| s.get(id));
            nodes.push(NodeResult {
                id: id.to_string(),
                status,
                conclusion,
                fires: stat.map(|s| s.fires),
                secs: stat.map(|s| (s.secs * 1000.0).round() / 1000.0),
                rc,
            });
        }

        let mut failed_events: Vec<String> = Vec::new();
        for event in log {
            // A declared rule's non-success conclusion counts the same
            // as a heuristic failure for `failed_events`.
            let decisive_non_success = match event_outcome(&rules, event) {
                EventOutcome::Decisive(c) => !conclusion_passes(c),
                EventOutcome::Neutral => false,
            };
            if decisive_non_success && !failed_events.contains(&event.kind) {
                failed_events.push(event.kind.clone());
            }
        }

        Self {
            schema: RUN_RESULT_SCHEMA.to_string(),
            run_id: run_id(campaign_name, log),
            campaign: campaign_name.to_string(),
            status,
            verdict,
            conclusion,
            started_at: log.first().and_then(|e| rfc3339(e.ts)),
            completed_at: log.last().and_then(|e| rfc3339(e.ts)),
            output: RunOutput {
                title: campaign_name.to_string(),
                summary: String::new(),
                text: String::new(),
            },
            nodes,
            failed_events,
        }
    }
}
