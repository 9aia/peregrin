#![allow(clippy::unwrap_used, clippy::pedantic)]
//! `subworkflow` driver tests (ADR-0022) — `params.path` resolution
//! anchored at the campaign dir, `inputs` → sub-run `config`, the
//! `outputs` first-match mapping (incl. `workflow.failed` terminal
//! mapping), namespace isolation (no leaks, no ingress drivers), and
//! secrets inheritance.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::{Value, json};

use peregrin_core::{Event, Node};
use peregrin_driver::{Driver, DriverError, ExecutionContext, NoopDriver};
use peregrin_driver_subworkflow::SubworkflowDriver;
use peregrin_engine::{Registry, RunOptions, Runner};
use peregrin_loader::Loader;
use peregrin_loader_yaml::YamlLoader;

/// `driver: emit` — emits `params.kind` with every other *resolved*
/// param as payload; test scaffolding so sub-campaigns can template
/// `config`/`secrets` into emitted events.
struct EmitDriver;

impl Driver for EmitDriver {
    fn name(&self) -> &'static str {
        "emit"
    }

    fn invoke(
        &self,
        node: &Node,
        _triggers: &[Event],
        ctx: &ExecutionContext,
    ) -> Result<Vec<Event>, DriverError> {
        let mut payload = (*ctx.params).clone();
        let kind = payload
            .remove("kind")
            .and_then(|v| v.as_str().map(str::to_string))
            .unwrap_or_else(|| "emit.done".to_string());
        Ok(vec![Event::new(kind, Value::Object(payload), &node.id)])
    }
}

/// `driver: fail` — always errors with `params.reason`.
struct FailDriver;

impl Driver for FailDriver {
    fn name(&self) -> &'static str {
        "fail"
    }

    fn invoke(
        &self,
        _node: &Node,
        _triggers: &[Event],
        ctx: &ExecutionContext,
    ) -> Result<Vec<Event>, DriverError> {
        Err(DriverError::new(
            ctx.params
                .get("reason")
                .and_then(Value::as_str)
                .unwrap_or("failed"),
        ))
    }
}

/// `driver: probe` — an ingress driver; excluded from sub-run
/// registries.
struct ProbeDriver;

impl Driver for ProbeDriver {
    fn name(&self) -> &'static str {
        "probe"
    }

    fn is_ingress(&self) -> bool {
        true
    }

    fn invoke(
        &self,
        _node: &Node,
        _triggers: &[Event],
        _ctx: &ExecutionContext,
    ) -> Result<Vec<Event>, DriverError> {
        Ok(Vec::new())
    }
}

fn registry() -> Registry {
    Registry::default()
        .with("subworkflow", Arc::new(SubworkflowDriver::new()))
        .with("emit", Arc::new(EmitDriver))
        .with("fail", Arc::new(FailDriver))
        .with("probe", Arc::new(ProbeDriver))
        .with("noop", Arc::new(NoopDriver))
}

/// `tests/fixtures` — the sub-campaign YAMLs; `load_str`'s base_dir
/// stamps `campaign.source_dir`, anchoring `params.path` there.
fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn run_opts(yaml: &str, options: &RunOptions) -> Vec<Event> {
    let campaign = YamlLoader.load_str(yaml, &fixtures()).unwrap();
    Runner::new(registry()).run(&campaign, options).unwrap()
}

fn run(yaml: &str) -> Vec<Event> {
    run_opts(yaml, &RunOptions::default())
}

fn kinds(log: &[Event]) -> Vec<&str> {
    log.iter().map(|e| e.kind.as_str()).collect()
}

fn node(id: &str, params: Value) -> Node {
    Node {
        id: id.to_string(),
        driver: "subworkflow".to_string(),
        on: vec!["start".to_string()],
        emits: vec![format!("{id}.done")],
        params,
        ..Node::default()
    }
}

const PARENT: &str = r#"
name: parent-e2e
config:
  greeting: hello-parent
nodes:
  - id: setup
    driver: subworkflow
    on: [start]
    emits: [worktree.ready, worktree.failed]
    params:
      path: echo.yaml
      inputs:
        greeting: "${config.greeting}"
      outputs:
        echo.ready: worktree.ready
  - id: after
    driver: emit
    on: [worktree.ready]
    emits: [after.done]
    params:
      kind: after.done
      got: "${event.payload.message}"
"#;

#[test]
fn mapped_output_reaches_downstream() {
    let log = run(PARENT);
    assert_eq!(
        kinds(&log),
        ["start", "worktree.ready", "after.done", "workflow.done"]
    );
    // `inputs.greeting` templated off the parent config became the
    // sub-run's `${config.greeting}`; its payload rode the mapping.
    let after = log.iter().find(|e| e.kind == "after.done").unwrap();
    assert_eq!(after.payload["got"], "hello-parent");
    // The mapped event carries provenance, source = the parent node.
    let ready = log.iter().find(|e| e.kind == "worktree.ready").unwrap();
    assert_eq!(ready.source, "setup");
    assert_eq!(ready.payload["sub_run"]["type"], "echo.ready");
    assert_eq!(ready.payload["sub_run"]["source"], "say");
}

#[test]
fn declared_config_is_the_default() {
    // No `inputs` — the sub-campaign's own `config:` applies.
    let log = run(r#"
nodes:
  - id: setup
    driver: subworkflow
    on: [start]
    emits: [worktree.ready]
    params:
      path: echo.yaml
      outputs: {echo.ready: worktree.ready}
"#);
    let ready = log.iter().find(|e| e.kind == "worktree.ready").unwrap();
    assert_eq!(ready.payload["message"], "default-greeting");
}

#[test]
fn sub_events_do_not_leak() {
    let log = run(PARENT);
    // Neither the mapped sub type nor the unmapped `echo.chatter`
    // (nor the sub-run's own node ids / terminal events) appear in
    // the parent log — isolation is by construction: a fresh bus.
    assert!(kinds(&log).iter().all(|k| !k.starts_with("echo.")));
    assert!(
        log.iter()
            .all(|e| e.source != "say" && e.source != "chatter")
    );
    assert_eq!(log.iter().filter(|e| e.kind == "start").count(), 1);
}

#[test]
fn secrets_are_inherited() {
    let options = RunOptions {
        secrets: [("API_KEY".to_string(), "s3cr3t".to_string())]
            .into_iter()
            .collect(),
        ..RunOptions::default()
    };
    let log = run_opts(PARENT, &options);
    let ready = log.iter().find(|e| e.kind == "worktree.ready").unwrap();
    assert_eq!(ready.payload["token"], "s3cr3t");
}

#[test]
fn first_matching_output_wins() {
    let log = run(r#"
nodes:
  - id: setup
    driver: subworkflow
    on: [start]
    emits: [early, late]
    params:
      path: echo.yaml
      outputs:
        echo.chatter: late
        echo.ready: early
"#);
    // `say` fires before `chatter` in the sub-run, so `echo.ready`
    // is the first log entry matching an `outputs` key.
    let kinds = kinds(&log);
    assert!(kinds.contains(&"early"));
    assert!(!kinds.contains(&"late"));
}

#[test]
fn terminal_failure_maps_through_outputs() {
    let log = run(r#"
nodes:
  - id: setup
    driver: subworkflow
    on: [start]
    emits: [worktree.ready, worktree.failed]
    params:
      path: fail.yaml
      outputs:
        echo.ready: worktree.ready
        workflow.failed: worktree.failed
  - id: on-fail
    driver: emit
    on: [worktree.failed]
    emits: [handled.done]
    params:
      kind: handled.done
"#);
    let kinds = kinds(&log);
    assert!(kinds.contains(&"worktree.failed"));
    assert!(kinds.contains(&"handled.done"));
    // The mapped failure event is a normal event — the parent run
    // completes, it does not fail.
    assert_eq!(kinds.last(), Some(&"workflow.done"));
    let failed = log.iter().find(|e| e.kind == "worktree.failed").unwrap();
    assert_eq!(failed.payload["sub_run"]["type"], "workflow.failed");
}

#[test]
fn unmapped_sub_failure_is_node_failed() {
    let log = run(r#"
nodes:
  - id: setup
    driver: subworkflow
    on: [start]
    emits: [worktree.ready]
    params:
      path: fail.yaml
      outputs: {echo.ready: worktree.ready}
"#);
    let failed = log.iter().find(|e| e.kind == "node.failed").unwrap();
    assert_eq!(failed.source, "setup");
    assert!(
        failed.payload["error"]
            .as_str()
            .unwrap()
            .contains("ended with 'workflow.failed'")
    );
    assert_eq!(kinds(&log).last(), Some(&"workflow.failed"));
}

#[test]
fn no_outputs_success_marker() {
    let log = run(r#"
nodes:
  - id: setup
    driver: subworkflow
    on: [start]
    params:
      path: echo.yaml
"#);
    let done = log.iter().find(|e| e.kind == "setup.done").unwrap();
    assert_eq!(done.payload["sub_run"]["path"], "echo.yaml");
    assert_eq!(kinds(&log).last(), Some(&"workflow.done"));
}

#[test]
fn no_matching_output_is_node_failed() {
    let log = run(r#"
nodes:
  - id: setup
    driver: subworkflow
    on: [start]
    emits: [worktree.ready]
    params:
      path: quiet.yaml
      outputs: {echo.ready: worktree.ready}
"#);
    let failed = log.iter().find(|e| e.kind == "node.failed").unwrap();
    assert!(
        failed.payload["error"]
            .as_str()
            .unwrap()
            .contains("none of the declared outputs")
    );
}

#[test]
fn nested_subworkflow_recurses() {
    let log = run(r#"
nodes:
  - id: outer
    driver: subworkflow
    on: [start]
    emits: [outer.ready]
    params:
      path: nested.yaml
      outputs: {inner.ready: outer.ready}
"#);
    // `nested.yaml` itself runs `echo.yaml` — `params.path` anchored
    // at the sub file's directory, one level down.
    let ready = log.iter().find(|e| e.kind == "outer.ready").unwrap();
    assert_eq!(ready.payload["message"], "from-nested");
    assert_eq!(ready.payload["sub_run"]["type"], "inner.ready");
    assert_eq!(kinds(&log).last(), Some(&"workflow.done"));
}

#[test]
fn missing_path_param_is_event_invalid() {
    // `path` is required by the driver's `input` contract — the
    // engine's input validation emits `event.invalid` before invoke
    // (non-strict campaigns don't escalate to `node.failed`).
    let log = run(r#"
nodes:
  - id: setup
    driver: subworkflow
    on: [start]
"#);
    let invalid = log.iter().find(|e| e.kind == "event.invalid").unwrap();
    assert_eq!(invalid.source, "setup");
    assert!(
        invalid.payload["errors"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e.as_str().unwrap().contains("path"))
    );
}

#[test]
fn wrong_typed_path_is_node_failed() {
    // A `path` that passes schema (`string`) but is empty fails
    // inside `invoke` — the DriverError → `node.failed` path.
    let log = run(r#"
nodes:
  - id: setup
    driver: subworkflow
    on: [start]
    params:
      path: ""
"#);
    let failed = log.iter().find(|e| e.kind == "node.failed").unwrap();
    assert!(
        failed.payload["error"]
            .as_str()
            .unwrap()
            .contains("missing 'params.path'")
    );
}

#[test]
fn unloadable_path_is_node_failed() {
    let log = run(r#"
nodes:
  - id: setup
    driver: subworkflow
    on: [start]
    params:
      path: does-not-exist.yaml
"#);
    let failed = log.iter().find(|e| e.kind == "node.failed").unwrap();
    assert!(
        failed.payload["error"]
            .as_str()
            .unwrap()
            .contains("cannot load 'does-not-exist.yaml'")
    );
}

#[test]
fn ingress_drivers_do_not_resolve_in_sub_runs() {
    let log = run(r#"
nodes:
  - id: setup
    driver: subworkflow
    on: [start]
    params:
      path: ingress.yaml
"#);
    let failed = log.iter().find(|e| e.kind == "node.failed").unwrap();
    assert!(
        failed.payload["error"]
            .as_str()
            .unwrap()
            .contains("no driver registered for 'probe'")
    );
}

#[test]
fn dry_run_short_circuits() {
    let options = RunOptions {
        dry_run: true,
        ..RunOptions::default()
    };
    let log = run_opts(PARENT, &options);
    let ready = log.iter().find(|e| e.kind == "worktree.ready").unwrap();
    assert_eq!(ready.payload["dry_run"], true);
}

#[test]
fn detached_context_has_no_sub_run_channel() {
    let driver = SubworkflowDriver::new();
    let ctx = ExecutionContext {
        campaign_dir: Some(fixtures()),
        ..ExecutionContext::default()
    };
    let err = driver
        .invoke(&node("setup", json!({"path": "echo.yaml"})), &[], &ctx)
        .unwrap_err();
    assert!(err.reason.contains("no sub-run channel"));
}
