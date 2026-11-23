#![allow(clippy::unwrap_used, clippy::pedantic)]
//! `RunResult` projection tests — verdict semantics over the event
//! log plus the runner's per-node fire stats.

use std::sync::Arc;

use peregrin_core::{Campaign, Event, Node};
use peregrin_driver::{Driver, DriverError, ExecutionContext};
use peregrin_engine::{Registry, RunOptions, RunVerdict, Runner};
use peregrin_loader::Loader;
use peregrin_loader_yaml::YamlLoader;
use serde_json::{Value, json};

/// Always fails — `node.failed` after retries exhaust (default
/// `max_attempts: 1`).
struct FailDriver;
impl Driver for FailDriver {
    fn name(&self) -> &'static str {
        "fail"
    }

    fn invoke(
        &self,
        _n: &Node,
        _t: &[Event],
        _c: &ExecutionContext,
    ) -> Result<Vec<Event>, DriverError> {
        Err(DriverError::new("boom"))
    }
}

/// Emits whatever `params.events` declares — `[{type, payload}]` —
/// so tests can script `*.failed` kinds and `rc` payloads without a
/// real shell.
struct ScriptDriver;
impl Driver for ScriptDriver {
    fn name(&self) -> &'static str {
        "script"
    }

    fn invoke(
        &self,
        node: &Node,
        _t: &[Event],
        _c: &ExecutionContext,
    ) -> Result<Vec<Event>, DriverError> {
        let events = node
            .params
            .get("events")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        Ok(events
            .iter()
            .filter_map(|spec| {
                Some(Event::new(
                    spec.get("type")?.as_str()?,
                    spec.get("payload").cloned().unwrap_or_else(|| json!({})),
                    &node.id,
                ))
            })
            .collect())
    }
}

fn registry() -> Registry {
    Registry::default()
        .with("fail", Arc::new(FailDriver))
        .with("script", Arc::new(ScriptDriver))
}

fn load(yaml: &str) -> Campaign {
    YamlLoader
        .load_str(yaml, std::path::Path::new("."))
        .unwrap()
}

fn run_result(yaml: &str) -> (peregrin_engine::RunResult, Vec<Event>) {
    let campaign = load(yaml);
    let mut runner = Runner::new(registry());
    let log = runner.run(&campaign, &RunOptions::default()).unwrap();
    (runner.run_result(&campaign), log)
}

#[test]
fn passing_run_reports_pass_with_node_stats() {
    let (result, _log) = run_result(
        "
        name: gate
        nodes:
          - id: build
            driver: script
            on: [start]
            emits: [build.done]
            params:
              events: [{type: build.done, payload: {rc: 0}}]
          - id: idle
            driver: script
            on: [never.comes]
            emits: [idle.done]
        ",
    );
    assert_eq!(result.campaign, "gate");
    assert_eq!(result.verdict, RunVerdict::Pass);
    assert!(result.failed_events.is_empty());
    assert_eq!(result.nodes.len(), 2);
    let build = &result.nodes[0];
    assert_eq!(build.id, "build");
    assert_eq!(
        build.status,
        peregrin_engine::NodeStatus::Completed,
        "nodes: {:?}",
        result.nodes
    );
    assert_eq!(build.fires, Some(1));
    assert!(build.secs.is_some());
    assert_eq!(build.rc, Some(0));
    // Declared but never triggered — pending, no stats.
    let idle = &result.nodes[1];
    assert_eq!(idle.id, "idle");
    assert_eq!(idle.status, peregrin_engine::NodeStatus::Pending);
    assert_eq!(idle.fires, None);
    assert_eq!(idle.secs, None);
    assert_eq!(idle.rc, None);
    // Round-trips through serde in the documented shape.
    let json = serde_json::to_value(&result).unwrap();
    assert_eq!(json["verdict"], "pass");
    assert_eq!(json["status"], "completed");
    assert_eq!(json["conclusion"], "success");
    assert!(json["run_id"].as_str().is_some_and(|id| id.len() == 16));
    assert_eq!(json["output"]["title"], "gate");
    assert_eq!(json["output"]["summary"], "");
    assert_eq!(json["nodes"][0]["conclusion"], "success");
    assert_eq!(json["nodes"][0]["status"], "completed");
    assert!(json["nodes"][1].get("secs").is_none());
}

#[test]
fn node_failed_event_fails_the_verdict() {
    let (result, log) = run_result(
        "
        name: gate
        nodes:
          - id: verify
            driver: fail
            on: [start]
        ",
    );
    assert!(types(&log).contains(&"workflow.failed"));
    assert_eq!(result.verdict, RunVerdict::Fail);
    let verify = result.nodes.iter().find(|n| n.id == "verify").unwrap();
    assert_eq!(verify.status, peregrin_engine::NodeStatus::Completed);
    assert_eq!(verify.fires, Some(1));
    // `node.failed` and the runner's `workflow.failed` terminator are
    // both failure-class kinds, deduplicated in log order.
    assert_eq!(result.failed_events, vec!["node.failed", "workflow.failed"]);
    assert_eq!(result.conclusion, peregrin_core::Conclusion::Failure);
}

#[test]
fn emit_mapped_failed_kind_fails_despite_done_terminal() {
    // The CI-gate shape: a shell `emit:` map routes rc!=0 onto
    // `verify.report.failed` — the engine's own `failed` flag doesn't
    // see it (terminal stays `workflow.done`), but the verdict must
    // still be `fail`.
    let (result, log) = run_result(
        "
        name: gate
        nodes:
          - id: verify
            driver: script
            on: [start]
            emits: [verify.report.failed]
            params:
              events: [{type: verify.report.failed, payload: {rc: 101}}]
        ",
    );
    assert_eq!(log.last().map(|e| e.kind.as_str()), Some("workflow.done"));
    assert_eq!(result.verdict, RunVerdict::Fail);
    assert_eq!(result.failed_events, vec!["verify.report.failed"]);
    let verify = &result.nodes[0];
    assert_eq!(verify.conclusion, Some(peregrin_core::Conclusion::Failure));
    assert_eq!(verify.status, peregrin_engine::NodeStatus::Completed);
    assert_eq!(verify.rc, Some(101));
}

#[test]
fn nonzero_rc_on_neutral_kind_fails_the_verdict() {
    // `emit: {"1": "verify.report"}` — the exit code lands on a kind
    // with no `.failed` segment; the payload `rc` still fails it.
    let (result, _log) = run_result(
        "
        name: gate
        nodes:
          - id: verify
            driver: script
            on: [start]
            emits: [verify.report]
            params:
              events: [{type: verify.report, payload: {rc: 1}}]
        ",
    );
    assert_eq!(result.verdict, RunVerdict::Fail);
    assert_eq!(result.failed_events, vec!["verify.report"]);
    assert_eq!(
        result.nodes[0].status,
        peregrin_engine::NodeStatus::Completed
    );
    assert_eq!(
        result.nodes[0].conclusion,
        Some(peregrin_core::Conclusion::Failure)
    );
}

#[test]
fn cancelled_run_is_not_a_pass() {
    let campaign = load(
        "
        name: gate
        stop_on: [halt.now]
        nodes:
          - id: halt
            driver: script
            on: [start]
            emits: [halt.now]
            params:
              events: [{type: halt.now, payload: {}}]
        ",
    );
    let mut runner = Runner::new(registry());
    runner.run(&campaign, &RunOptions::default()).unwrap();
    let result = runner.run_result(&campaign);
    assert_eq!(result.verdict, RunVerdict::Fail);
    assert_eq!(result.failed_events, vec!["workflow.cancelled"]);
    assert_eq!(result.status, peregrin_engine::RunStatus::Cancelled);
    assert_eq!(result.conclusion, peregrin_core::Conclusion::Cancelled);
}

#[test]
fn from_log_projects_without_fire_stats() {
    // The supervisor/log-only path — statuses and rc come off the
    // log; secs/fires stay absent.
    let campaign = load(
        "
        name: gate
        nodes:
          - id: verify
            driver: script
            on: [start]
            emits: [verify.done]
        ",
    );
    let log = vec![
        Event::new("start", json!({}), "__runner__"),
        Event::new("verify.done", json!({"rc": 0}), "verify"),
        Event::done(),
    ];
    let result = peregrin_engine::RunResult::from_log(&campaign, &log);
    assert_eq!(result.verdict, RunVerdict::Pass);
    assert!(result.started_at.is_some() && result.completed_at.is_some());
    assert_eq!(
        result.nodes[0].status,
        peregrin_engine::NodeStatus::Completed
    );
    assert_eq!(result.nodes[0].rc, Some(0));
    assert_eq!(result.nodes[0].secs, None);
    assert_eq!(result.nodes[0].fires, None);
}

#[test]
fn error_document_is_a_bare_verdict() {
    let result = peregrin_engine::RunResult::error("gate");
    assert_eq!(result.campaign, "gate");
    assert_eq!(result.verdict, RunVerdict::Error);
    assert_eq!(result.conclusion, peregrin_core::Conclusion::Failure);
    assert!(result.nodes.is_empty());
    assert!(result.failed_events.is_empty());
    let json = serde_json::to_value(&result).unwrap();
    assert_eq!(json["verdict"], "error");
    assert_eq!(json["conclusion"], "failure");
}

#[test]
fn engine_err_forces_error_verdict() {
    // Strict mode without payload schemas is a campaign-plane
    // failure — `run` returns Err but the partial projection still
    // writes an `error` document.
    let campaign = load(
        "
        name: gate
        strict: true
        events:
          - type: anything.happens
        nodes:
          - id: noop
            driver: script
            on: [start]
        ",
    );
    let mut runner = Runner::new(registry());
    let outcome = runner.run(&campaign, &RunOptions::default());
    assert!(outcome.is_err());
    let result = runner.run_result_error(&campaign);
    assert_eq!(result.verdict, RunVerdict::Error);
    assert_eq!(result.conclusion, peregrin_core::Conclusion::Failure);
}

#[test]
fn declared_result_rule_supplies_conclusion() {
    // `result:` maps gate events to conclusions — last decisive event
    // wins, so `gate.fail` then `gate.pass` concludes success.
    let (result, _log) = run_result(
        "
        name: gate
        result:
          - event: gate.pass
            conclusion: success
          - event: gate.fail
            conclusion: failure
        nodes:
          - id: gate
            driver: script
            on: [start]
            emits: [gate.pass, gate.fail]
            params:
              events:
                - {type: gate.fail, payload: {}}
                - {type: gate.pass, payload: {}}
        ",
    );
    assert_eq!(result.verdict, RunVerdict::Pass);
    assert_eq!(result.conclusion, peregrin_core::Conclusion::Success);
    assert_eq!(
        result.failed_events,
        vec!["gate.fail"],
        "a declared failure still lands in failed_events"
    );
    assert_eq!(
        result.nodes[0].conclusion,
        Some(peregrin_core::Conclusion::Success)
    );
}

#[test]
fn declared_failure_rule_fails_on_neutral_kind() {
    // `gate.fail` has no `.failed` segment and no `rc` — without the
    // declaration the run would pass; the rule makes it decisive.
    let (result, _log) = run_result(
        "
        name: gate
        result:
          - event: gate.fail
            conclusion: failure
        nodes:
          - id: gate
            driver: script
            on: [start]
            emits: [gate.fail]
            params:
              events: [{type: gate.fail, payload: {}}]
        ",
    );
    assert_eq!(result.verdict, RunVerdict::Fail);
    assert_eq!(result.conclusion, peregrin_core::Conclusion::Failure);
    assert_eq!(result.failed_events, vec!["gate.fail"]);
}

#[test]
fn undeclared_events_keep_the_heuristic() {
    // Mixed: a declared rule for one kind, the `*.failed` heuristic
    // still catches undeclared kinds.
    let (result, _log) = run_result(
        "
        name: gate
        result:
          - event: gate.pass
            conclusion: success
        nodes:
          - id: a
            driver: script
            on: [start]
            emits: [gate.pass]
            params:
              events: [{type: gate.pass, payload: {}}]
          - id: b
            driver: script
            on: [gate.pass]
            emits: [verify.failed]
            params:
              events: [{type: verify.failed, payload: {}}]
        ",
    );
    assert_eq!(result.conclusion, peregrin_core::Conclusion::Failure);
    assert_eq!(result.verdict, RunVerdict::Fail);
}

#[test]
fn run_id_is_stable_across_projections() {
    // The same run's log projects to the same `run_id` whether it
    // comes off the runner or `RunResult::from_log` — consumers can
    // correlate result documents with history lines.
    let campaign = load(
        "
        name: gate
        nodes:
          - id: verify
            driver: script
            on: [start]
            emits: [verify.done]
            params:
              events: [{type: verify.done, payload: {rc: 0}}]
        ",
    );
    let mut runner = Runner::new(registry());
    let log = runner.run(&campaign, &RunOptions::default()).unwrap();
    let via_runner = runner.run_result(&campaign);
    let via_log = peregrin_engine::RunResult::from_log(&campaign, &log);
    assert_eq!(via_runner.run_id, via_log.run_id);
    assert!(!via_runner.run_id.is_empty());
    // And the timestamps bracket the log.
    assert!(via_runner.started_at.is_some());
    assert!(via_runner.completed_at.is_some());
    assert!(
        via_runner
            .started_at
            .as_deref()
            .is_some_and(|s| s.ends_with('Z') || s.contains('+'))
    );
}

#[test]
fn result_rule_round_trips_through_to_dict() {
    // `result:` survives the model's dict projection — the loader's
    // include/merge path keeps it.
    let campaign = load(
        "
        name: gate
        result:
          - event: gate.pass
            conclusion: success
        nodes: []
        ",
    );
    let dict = campaign.to_dict();
    let rules = dict["result"].as_array().unwrap();
    assert_eq!(rules[0]["event"], "gate.pass");
    assert_eq!(rules[0]["conclusion"], "success");
    let reparsed = Campaign::from_dict(&dict).unwrap();
    assert_eq!(reparsed.result, campaign.result);
}

fn types(log: &[Event]) -> Vec<&str> {
    log.iter().map(|e| e.kind.as_str()).collect()
}
