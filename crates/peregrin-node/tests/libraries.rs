#![allow(clippy::unwrap_used, clippy::pedantic)]

//! Resilience-fragment e2e — the YAML artifacts in `libraries/{retry,
//! circuit-breaker,dead-letter}` loaded through `campaign::load`
//! (include expansion, id-deduped compose) and executed on the engine
//! `Runner`. The stock registry covers shell/noop/runner; two small
//! fakes (`emit-list`, `worker`) simulate producers and a fallible
//! worker with an `attempt` counter so every fragment branch —
//! retry-then-done, retry-exhaustion, backoff delay, circuit
//! open→half-open→gate→closed, dlq capture/inventory/replay — is
//! exercised end to end.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use peregrin_core::{Event, Node};
use peregrin_driver::{Driver, DriverError, ExecutionContext};
use peregrin_engine::{Registry, RunOptions, Runner};
use peregrin_node::campaign;
use serde_json::{Value, json};

// ---------------------------------------------------------------------------
// Test fakes
// ---------------------------------------------------------------------------

/// `driver: emit-list` — emits every `{type, payload}` in
/// `params.events`, in order. The producer for trigger events a YAML
/// campaign can't otherwise synthesize.
struct EmitList;
impl Driver for EmitList {
    fn name(&self) -> &'static str {
        "emit-list"
    }

    fn invoke(
        &self,
        node: &Node,
        _t: &[Event],
        ctx: &ExecutionContext,
    ) -> Result<Vec<Event>, DriverError> {
        let events = ctx
            .params
            .get("events")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        Ok(events
            .iter()
            .filter_map(|spec| {
                let kind = spec.get("type").and_then(Value::as_str)?.to_string();
                let payload = spec.get("payload").cloned().unwrap_or(Value::Null);
                Some(Event::new(kind, payload, &node.id))
            })
            .collect())
    }
}

/// `driver: worker` — the fallible task executor. Reads
/// `triggers[0].payload.attempt` (default 0) and emits
/// `params.failed` (default `task.failed`) carrying `{attempt + 1,
/// error}` — until the node's fire count reaches `params.succeed_at`,
/// when it emits `params.done` (default `task.done`). `attempt` in the
/// failure payload is what the retry forwarder carries back into the
/// next request.
struct Worker {
    calls: Arc<AtomicU64>,
}
impl Driver for Worker {
    fn name(&self) -> &'static str {
        "worker"
    }

    fn invoke(
        &self,
        node: &Node,
        triggers: &[Event],
        ctx: &ExecutionContext,
    ) -> Result<Vec<Event>, DriverError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let attempt = triggers
            .first()
            .and_then(|t| t.payload.get("attempt"))
            .and_then(Value::as_i64)
            .unwrap_or(0)
            + 1;
        let succeed_at = ctx
            .params
            .get("succeed_at")
            .and_then(Value::as_i64)
            .unwrap_or(i64::MAX);
        let (kind, payload) = if ctx.fires as i64 >= succeed_at {
            (
                ctx.params
                    .get("done")
                    .and_then(Value::as_str)
                    .unwrap_or("task.done"),
                json!({ "attempt": attempt }),
            )
        } else {
            (
                ctx.params
                    .get("failed")
                    .and_then(Value::as_str)
                    .unwrap_or("task.failed"),
                json!({ "attempt": attempt, "error": "boom" }),
            )
        };
        Ok(vec![Event::new(kind.to_string(), payload, &node.id)])
    }
}

/// `driver: fail` — always `Err`, producing the engine's own
/// `node.failed` event (the same one retry exhaustion and the DLQ port
/// see).
struct FailAlways;
impl Driver for FailAlways {
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

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn libraries_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../libraries")
        .canonicalize()
        .unwrap()
}

fn registry(calls: &Arc<AtomicU64>) -> Registry {
    campaign::registry()
        .with("emit-list", Arc::new(EmitList))
        .with("fail", Arc::new(FailAlways))
        .with(
            "worker",
            Arc::new(Worker {
                calls: calls.clone(),
            }),
        )
}

/// Write `yaml` to a tempdir and run it through `Runner` on the fake
/// registry. `{lib}` in the yaml is replaced by the libraries dir.
fn run_yaml(dir: &Path, yaml: &str, calls: &Arc<AtomicU64>) -> Vec<Event> {
    let path = dir.join("campaign.yaml");
    std::fs::write(
        &path,
        yaml.replace("{lib}", &libraries_dir().display().to_string()),
    )
    .unwrap();
    let loaded = campaign::load(&path).unwrap();
    let mut runner = Runner::new(registry(calls));
    runner.run(&loaded, &RunOptions::default()).unwrap()
}

fn kinds(log: &[Event]) -> Vec<&str> {
    log.iter().map(|e| e.kind.as_str()).collect()
}

fn count(log: &[Event], kind: &str) -> usize {
    log.iter().filter(|e| e.kind == kind).count()
}

// ---------------------------------------------------------------------------
// retry
// ---------------------------------------------------------------------------

#[test]
fn retry_exhaustion_reemits_request_with_attempt_then_dead() {
    let tmp = tempfile::tempdir().unwrap();
    let calls = Arc::new(AtomicU64::new(0));
    let log = run_yaml(
        tmp.path(),
        r#"
        name: retry-e2e
        include: ["{lib}/retry/main.yaml"]
        nodes:
          - id: producer
            driver: emit-list
            on: [start]
            emits: [task.request]
            params:
              events:
                - { type: task.request, payload: { attempt: 0 } }
          - id: worker
            driver: worker
            on: [task.request]
            emits: [task.done, task.failed]
            max_fires: unlimited
        "#,
        &calls,
    );
    let t = kinds(&log);
    // 1 original + 2 retries = max_attempts requests, each failure
    // re-emitted with the incremented attempt riding the payload.
    let requests: Vec<&Event> = log.iter().filter(|e| e.kind == "task.request").collect();
    assert_eq!(requests.len(), 3, "trace: {t:?}");
    assert_eq!(requests[1].payload["attempt"], 1);
    assert_eq!(requests[2].payload["attempt"], 2);
    assert_eq!(count(&log, "task.failed"), 3);
    assert_eq!(count(&log, "retry.scheduled"), 2);
    // Terminal event once the budget is spent — fan_in payload counts
    // the logged failures.
    let dead = log.iter().find(|e| e.kind == "task.dead").unwrap();
    assert_eq!(dead.payload["count"], 3);
    // The exhausted forwarder reports node.exhausted; no 4th attempt.
    let exhausted = log.iter().find(|e| e.kind == "node.exhausted").unwrap();
    assert_eq!(exhausted.payload["node"], "retry.loop");
    assert_eq!(calls.load(Ordering::SeqCst), 3);
    assert_eq!(t.last(), Some(&"workflow.done"), "trace: {t:?}");
}

#[test]
fn retry_success_stops_before_dead() {
    let tmp = tempfile::tempdir().unwrap();
    let calls = Arc::new(AtomicU64::new(0));
    let log = run_yaml(
        tmp.path(),
        r#"
        name: retry-ok
        include: ["{lib}/retry/main.yaml"]
        nodes:
          - id: producer
            driver: emit-list
            on: [start]
            emits: [task.request]
            params:
              events:
                - { type: task.request, payload: { attempt: 0 } }
          - id: worker
            driver: worker
            on: [task.request]
            emits: [task.done, task.failed]
            max_fires: unlimited
            params:
              succeed_at: 2
        "#,
        &calls,
    );
    let t = kinds(&log);
    assert_eq!(count(&log, "task.request"), 2);
    assert_eq!(count(&log, "task.failed"), 1);
    assert!(t.contains(&"task.done"), "trace: {t:?}");
    assert!(!t.contains(&"task.dead"), "trace: {t:?}");
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[test]
fn retry_backoff_delay_routes_through_wait() {
    let tmp = tempfile::tempdir().unwrap();
    let calls = Arc::new(AtomicU64::new(0));
    let log = run_yaml(
        tmp.path(),
        r#"
        name: retry-delay
        include: ["{lib}/retry/main.yaml"]
        config:
          retry: { delay: 0 }
        nodes:
          - id: producer
            driver: emit-list
            on: [start]
            emits: [task.request]
            params:
              events:
                - { type: task.request, payload: { attempt: 0 } }
          - id: worker
            driver: worker
            on: [task.request]
            emits: [task.done, task.failed]
            max_fires: unlimited
            params:
              succeed_at: 2
          # Local retry.loop wins the id-dedup — the fragment's is
          # dropped, so the forward target becomes task.retry-wait.
          - include: "{lib}/retry/retry.node.yml"
            id: retry.loop
            on: [task.failed]
            forward: [task.retry-wait]
            until: [task.done, task.dead]
            emits: [retry.scheduled, task.retry-wait]
            max_fires: 2
          - include: "{lib}/retry/delay.node.yml"
            id: retry.delay
            on: [task.retry-wait]
            forward: [task.request]
            until: [task.done, task.dead]
            emits: [retry.delayed, task.request]
            max_fires: unlimited
        "#,
        &calls,
    );
    let t = kinds(&log);
    assert_eq!(count(&log, "task.retry-wait"), 1, "trace: {t:?}");
    assert_eq!(count(&log, "retry.delayed"), 1, "trace: {t:?}");
    assert_eq!(count(&log, "task.request"), 2);
    assert!(t.contains(&"task.done"), "trace: {t:?}");
    assert!(!t.contains(&"task.dead"), "trace: {t:?}");
}

#[test]
fn retry_fragment_runs_on_the_stock_registry() {
    // campaign::run on a real shell worker — `exit 1` mapped to
    // task.failed exercises the fragment without test fakes.
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("campaign.yaml");
    std::fs::write(
        &path,
        format!(
            r#"
            name: retry-shell
            include: ["{}/retry/main.yaml"]
            nodes:
              - id: worker
                driver: shell
                on: [task.request]
                emits: [task.done, task.failed]
                max_fires: unlimited
                params:
                  cmd: "exit 1"
                  emit:
                    "0": task.done
                    default: task.failed
              - id: kick
                driver: shell
                on: [start]
                emits: [task.request]
                params:
                  cmd: "true"
                  emit:
                    "0": task.request
                    default: task.request
            "#,
            libraries_dir().display()
        ),
    )
    .unwrap();
    let log = campaign::run(&path, &RunOptions::default()).unwrap();
    let t = kinds(&log);
    assert_eq!(count(&log, "task.request"), 3, "trace: {t:?}");
    assert_eq!(count(&log, "task.failed"), 3, "trace: {t:?}");
    assert!(t.contains(&"task.dead"), "trace: {t:?}");
    assert_eq!(t.last(), Some(&"workflow.done"), "trace: {t:?}");
}

#[test]
fn include_compose_dedupes_redeclared_node_ids() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("campaign.yaml");
    std::fs::write(
        &path,
        format!(
            r#"
            name: dedupe
            include: ["{}/retry/main.yaml"]
            nodes:
              - id: retry.loop
                driver: noop
                on: [mytask.failed]
            "#,
            libraries_dir().display()
        ),
    )
    .unwrap();
    let loaded = campaign::load(&path).unwrap();
    let loops: Vec<_> = loaded
        .nodes
        .iter()
        .filter(|n| n.id == "retry.loop")
        .collect();
    assert_eq!(
        loops.len(),
        1,
        "nodes: {:?}",
        loaded.nodes.iter().map(|n| &n.id).collect::<Vec<_>>()
    );
    assert_eq!(loops[0].on, vec!["mytask.failed".to_string()]);
    // The fragment's other nodes survive the merge.
    assert!(loaded.nodes.iter().any(|n| n.id == "retry.dead"));
}

// ---------------------------------------------------------------------------
// circuit-breaker
// ---------------------------------------------------------------------------

#[test]
fn circuit_opens_gates_then_probes_on_half_open() {
    let tmp = tempfile::tempdir().unwrap();
    let calls = Arc::new(AtomicU64::new(0));
    let log = run_yaml(
        tmp.path(),
        r#"
        name: circuit-e2e
        include: ["{lib}/circuit-breaker/main.yaml"]
        config:
          circuit: { threshold: 3, reset_after: 0.05 }
        nodes:
          - id: producer
            driver: emit-list
            on: [start]
            emits: [task.request]
            params:
              events:
                - { type: task.request, payload: { attempt: 0 } }
                - { type: task.request, payload: { attempt: 0 } }
                - { type: task.request, payload: { attempt: 0 } }
          - id: worker
            driver: worker
            on: [task.dispatch]
            emits: [task.done, task.failed]
            max_fires: unlimited
          # A request published after the trip — the dispatcher's
          # `until: [circuit.open]` must swallow it.
          - id: late
            driver: emit-list
            on: [circuit.open]
            emits: [task.request]
            params:
              events:
                - { type: task.request, payload: { attempt: 0, late: true } }
        "#,
        &calls,
    );
    let t = kinds(&log);
    assert!(t.contains(&"circuit.open"), "trace: {t:?}");
    assert!(t.contains(&"circuit.half-open"), "trace: {t:?}");
    assert!(t.contains(&"circuit.armed"), "trace: {t:?}");
    assert!(t.contains(&"circuit.disarmed"), "trace: {t:?}");
    // 3 burst dispatches + the half-open probe trial; the late request
    // never reaches the dispatch channel.
    assert_eq!(count(&log, "task.request"), 4, "trace: {t:?}");
    assert_eq!(count(&log, "task.dispatch"), 4, "trace: {t:?}");
    assert_eq!(calls.load(Ordering::SeqCst), 4);
    assert_eq!(count(&log, "task.failed"), 4);
    // The trial failed — one-shot fragment emits no closed.
    assert!(!t.contains(&"circuit.closed"), "trace: {t:?}");
    // `circuit.open` is an engine failure type — a tripped breaker
    // marks the run failed (same semantics as the engine field).
    assert_eq!(t.last(), Some(&"workflow.failed"), "trace: {t:?}");
}

#[test]
fn circuit_closed_after_successful_probe() {
    let tmp = tempfile::tempdir().unwrap();
    let calls = Arc::new(AtomicU64::new(0));
    let log = run_yaml(
        tmp.path(),
        r#"
        name: circuit-recover
        include: ["{lib}/circuit-breaker/main.yaml"]
        config:
          circuit: { threshold: 3, reset_after: 0.05 }
        nodes:
          - id: producer
            driver: emit-list
            on: [start]
            emits: [task.request]
            params:
              events:
                - { type: task.request, payload: { attempt: 0 } }
                - { type: task.request, payload: { attempt: 0 } }
                - { type: task.request, payload: { attempt: 0 } }
          - id: worker
            driver: worker
            on: [task.dispatch]
            emits: [task.done, task.failed]
            max_fires: unlimited
            params:
              succeed_at: 4   # the probe trial succeeds
        "#,
        &calls,
    );
    let t = kinds(&log);
    assert!(t.contains(&"circuit.open"), "trace: {t:?}");
    assert!(t.contains(&"circuit.half-open"), "trace: {t:?}");
    assert!(t.contains(&"task.done"), "trace: {t:?}");
    assert!(t.contains(&"circuit.closed"), "trace: {t:?}");
    // closed lands after half-open in the log.
    let half = t.iter().position(|k| *k == "circuit.half-open").unwrap();
    let closed = t.iter().position(|k| *k == "circuit.closed").unwrap();
    assert!(closed > half, "trace: {t:?}");
    assert_eq!(calls.load(Ordering::SeqCst), 4);
}

#[test]
fn circuit_skip_if_gate_skips_while_the_flag_exists() {
    // The file-flag gate (gate.node.yml): a request published between
    // arm and disarm sees the flag and skips with node.skipped.
    let tmp = tempfile::tempdir().unwrap();
    let flag_dir = tmp.path().join("cb");
    std::fs::create_dir_all(&flag_dir).unwrap();
    let calls = Arc::new(AtomicU64::new(0));
    let log = run_yaml(
        tmp.path(),
        &format!(
            r#"
        name: circuit-gate
        include: ["{{lib}}/circuit-breaker/main.yaml"]
        config:
          circuit: {{ threshold: 1, reset_after: 0.05 }}
        nodes:
          - id: producer
            driver: emit-list
            on: [start]
            emits: [task.request]
            params:
              events:
                - {{ type: task.request, payload: {{ attempt: 0 }} }}
          # Re-declare arm/disarm so the flag lives in the tempdir —
          # the id-dedup drops the fragment's defaults.
          - include: "{{lib}}/circuit-breaker/arm.node.yml"
            id: circuit-breaker.arm
            on: [circuit.open]
            params:
              dir: "{flag_dir}"
          - include: "{{lib}}/circuit-breaker/disarm.node.yml"
            id: circuit-breaker.disarm
            on: [circuit.half-open]
            params:
              dir: "{flag_dir}"
          # Re-declare the probe to forward onto the request channel —
          # this variant's gated worker subscribes task.request only.
          - include: "{{lib}}/circuit-breaker/probe.node.yml"
            id: circuit-breaker.probe
            on: [circuit.half-open]
            forward: [task.request]
            emits: [circuit.probing, task.request]
          # Gated worker on the raw request channel + the gate mixin.
          - include: "{{lib}}/circuit-breaker/gate.node.yml"
            id: worker
            driver: worker
            on: [task.request]
            emits: [task.done, task.failed]
            max_fires: unlimited
            state:
              key: worker-cb
              skip_if: "${{shell('test -f {flag_dir}/.circuit.open && echo 1 || echo 0')}}"
          # A request that lands while the flag exists — post-open,
          # pre-disarm — must be skipped.
          - id: late
            driver: emit-list
            on: [circuit.open]
            emits: [task.request]
            params:
              events:
                - {{ type: task.request, payload: {{ attempt: 0, late: true }} }}
        "#,
            flag_dir = flag_dir.display()
        ),
        &calls,
    );
    let t = kinds(&log);
    // threshold 1 → first failure trips; the late request is skipped
    // while the flag file exists; the probe trial runs after disarm.
    assert!(t.contains(&"circuit.open"), "trace: {t:?}");
    assert!(t.contains(&"circuit.half-open"), "trace: {t:?}");
    let skipped = log.iter().find(|e| e.kind == "node.skipped").unwrap();
    assert_eq!(skipped.payload["node"], "worker");
    // 1 burst fire (skipped 2nd, probe 3rd? no — burst=1, late skipped,
    // probe trial=2 total driver invocations).
    assert_eq!(calls.load(Ordering::SeqCst), 2, "trace: {t:?}");
    assert_eq!(count(&log, "task.failed"), 2);
    assert!(
        !flag_dir.join(".circuit.open").exists(),
        "flag must be disarmed"
    );
}

// ---------------------------------------------------------------------------
// dead-letter
// ---------------------------------------------------------------------------

#[test]
fn dlq_captures_dead_and_failed_and_replays_request() {
    let tmp = tempfile::tempdir().unwrap();
    let calls = Arc::new(AtomicU64::new(0));
    let log = run_yaml(
        tmp.path(),
        r#"
        name: dlq-e2e
        include: ["{lib}/dead-letter/main.yaml"]
        nodes:
          - id: producer
            driver: emit-list
            on: [start]
            emits: [task.dead, task.failed, dlq.replay-requested.task]
            params:
              events:
                - { type: task.dead, payload: { job: a } }
                - { type: task.failed, payload: { job: b } }
                - { type: dlq.replay-requested.task, payload: { job: a } }
          - id: worker
            driver: worker
            on: [task.request]
            emits: [task.done]
            params:
              succeed_at: 1
        "#,
        &calls,
    );
    let t = kinds(&log);
    // Both terminal events are captured under dlq.captured.* with
    // their payloads preserved — the log is the queue.
    let captured: Vec<&Event> = log
        .iter()
        .filter(|e| e.kind == "dlq.captured.task")
        .collect();
    assert_eq!(captured.len(), 2, "trace: {t:?}");
    assert_eq!(captured[0].payload["job"], "a");
    assert_eq!(captured[1].payload["job"], "b");
    assert_eq!(count(&log, "dlq.event"), 2);
    // The inventory snapshot carries every letter.
    let stored = log.iter().find(|e| e.kind == "dlq.stored").unwrap();
    assert_eq!(stored.payload["count"], 1);
    let latest_stored = log.iter().rfind(|e| e.kind == "dlq.stored").unwrap();
    assert!(latest_stored.payload["triggers"].as_array().unwrap().len() >= 2);
    // The replay request re-emits <name>.request; the stored-letter
    // replay also emits replay.<name> carrying the oldest letter.
    let replayed = log.iter().find(|e| e.kind == "task.request").unwrap();
    assert_eq!(replayed.payload["job"], "a");
    let stored_replays: Vec<&Event> = log.iter().filter(|e| e.kind == "replay.task").collect();
    assert!(!stored_replays.is_empty(), "trace: {t:?}");
    assert!(
        stored_replays.iter().any(|e| e.payload["job"] == "a"),
        "stored letter payload must re-emit: {t:?}"
    );
    assert!(t.contains(&"dlq.replayed"), "trace: {t:?}");
    // The replayed request actually reached the worker.
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(t.contains(&"task.done"), "trace: {t:?}");
}

#[test]
fn dlq_captures_engine_node_failed() {
    // A shell node that exits nonzero without an emit map produces the
    // engine's `node.failed` — the fragment captures it on the bus.
    let tmp = tempfile::tempdir().unwrap();
    let calls = Arc::new(AtomicU64::new(0));
    let log = run_yaml(
        tmp.path(),
        r#"
        name: dlq-engine
        include: ["{lib}/dead-letter/main.yaml"]
        nodes:
          - id: bad
            driver: fail
            on: [start]
        "#,
        &calls,
    );
    let t = kinds(&log);
    // DriverError → the engine's own node.failed (retry-exhausted path).
    let failed = log.iter().find(|e| e.kind == "node.failed").unwrap();
    assert_eq!(failed.payload["node"], "bad");
    let captured = log.iter().find(|e| e.kind == "dlq.captured.node").unwrap();
    assert_eq!(captured.payload["error"], "boom");
    assert!(t.contains(&"workflow.failed"), "trace: {t:?}");
}
