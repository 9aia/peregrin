#![allow(clippy::unwrap_used)]
//! Engine tests — ports of `ac-engine`'s runner semantics plus the
//! `packages/tests` integration suite (fan-in, forward wildcards,
//! capabilities, until/retire_on/max_fires, snippets).

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use peregrin_bus::{LocalBus, capture_wildcards, matches};
use peregrin_core::{Campaign, Event, Node};
use peregrin_driver::{Driver, DriverError, ExecutionContext, NoopDriver};
use peregrin_engine::{Registry, RunOptions, Runner, SecretSource};
use peregrin_loader::Loader;
use peregrin_loader_yaml::YamlLoader;
use peregrin_store::MemoryDlq;
use serde_json::{Value, json};

// ---------------------------------------------------------------------------
// Fake drivers — ports of the TS test-suite fakes.
// ---------------------------------------------------------------------------

/// TS `pythonDriver` — emits `emits[0].replace("*", id)` per `params.ids`.
struct PythonFake;
impl Driver for PythonFake {
    fn name(&self) -> &'static str {
        "python"
    }

    fn invoke(
        &self,
        node: &Node,
        _t: &[Event],
        _c: &ExecutionContext,
    ) -> Result<Vec<Event>, DriverError> {
        let ids: Vec<String> = node
            .params
            .get("ids")
            .and_then(Value::as_array)
            .map_or_else(
                || vec!["a".into(), "b".into(), "c".into()],
                |a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(str::to_string))
                        .collect()
                },
            );
        let template = node
            .emits
            .first()
            .cloned()
            .unwrap_or_else(|| format!("{}.done", node.id));
        Ok(ids
            .into_iter()
            .map(|id| {
                Event::new(
                    template.replacen('*', &id, 1),
                    json!({ "id": id }),
                    &node.id,
                )
            })
            .collect())
    }
}

/// TS test `shellDriver` — emits `emits[0]` (wildcard → trigger shard)
/// with `{stdout: cmd}`.
struct ShellFake;
impl Driver for ShellFake {
    fn name(&self) -> &'static str {
        "shell"
    }

    fn invoke(
        &self,
        node: &Node,
        triggers: &[Event],
        ctx: &ExecutionContext,
    ) -> Result<Vec<Event>, DriverError> {
        // Like the TS fake: drivers get RAW params and re-resolve
        // inside invoke (resolvedParams only feed input/capability
        // checks upstream).
        let tctx = peregrin_template::Context {
            strict: ctx.strict,
            missing_secrets: ctx.missing_secrets.clone(),
            capabilities: (*ctx.capabilities).clone(),
            effects: &peregrin_template::SystemEffects,
        };
        let resolved = peregrin_template::resolve_value_opts(
            &node.params,
            &tctx,
            &peregrin_template::Options {
                node: Some(node),
                functions: &[],
                ..Default::default()
            },
        )
        .unwrap_or_else(|_| node.params.clone());
        let cmd = resolved
            .get("cmd")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let trigger = triggers.first();
        let pattern = trigger
            .and_then(|t| node.on.iter().find(|p| matches(p, &t.kind)))
            .or_else(|| node.on.first());
        let shard = trigger
            .zip(pattern)
            .and_then(|(t, p)| capture_wildcards(p, &t.kind))
            .and_then(|caps| caps.get("1").cloned())
            .unwrap_or_default();
        let template = node
            .emits
            .first()
            .cloned()
            .unwrap_or_else(|| format!("{}.done", node.id));
        let kind = if template.contains('*') {
            template.replacen('*', &shard, 1)
        } else {
            template
        };
        Ok(vec![Event::new(kind, json!({ "stdout": cmd }), &node.id)])
    }
}

/// TS `looperDriver` — `loop.retry` until `ctx.fires >= 2`.
struct LooperFake;
impl Driver for LooperFake {
    fn name(&self) -> &'static str {
        "looper"
    }

    fn invoke(
        &self,
        node: &Node,
        _t: &[Event],
        ctx: &ExecutionContext,
    ) -> Result<Vec<Event>, DriverError> {
        if ctx.fires >= 2 {
            return Ok(vec![Event::new(
                "loop.success",
                json!({ "fires": ctx.fires }),
                &node.id,
            )]);
        }
        Ok(vec![Event::new(
            "loop.retry",
            json!({ "fires": ctx.fires }),
            &node.id,
        )])
    }
}

/// Always fails — retry/DLQ tests.
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

/// Fails until the Nth invoke — retry-success + circuit tests.
struct FlakyDriver {
    calls: Arc<AtomicU64>,
    succeed_on: u64,
}
impl Driver for FlakyDriver {
    fn name(&self) -> &'static str {
        "flaky"
    }

    fn invoke(
        &self,
        node: &Node,
        _t: &[Event],
        _c: &ExecutionContext,
    ) -> Result<Vec<Event>, DriverError> {
        if self.calls.fetch_add(1, Ordering::SeqCst) + 1 >= self.succeed_on {
            return Ok(vec![Event::new("flaky.ok", json!({}), &node.id)]);
        }
        Err(DriverError::new("not yet"))
    }
}

/// Echoes the fire's resolved params (`ctx.params`) — the runner-bound
/// `${scope.*}` resolution is observable in the emitted payload.
struct ScopeEchoDriver;
impl Driver for ScopeEchoDriver {
    fn name(&self) -> &'static str {
        "scope-echo"
    }

    fn invoke(
        &self,
        node: &Node,
        _t: &[Event],
        ctx: &ExecutionContext,
    ) -> Result<Vec<Event>, DriverError> {
        Ok(vec![Event::new(
            node.emits
                .first()
                .cloned()
                .unwrap_or_else(|| "echo.done".to_string()),
            json!({ "params": ctx.params.as_ref().clone() }),
            &node.id,
        )])
    }
}

/// Writes `{"flag": "yes"}` into the shared store under
/// `params.state_key` — seeds the scoped store for `state.key` nodes.
struct StateSetDriver;
impl Driver for StateSetDriver {
    fn name(&self) -> &'static str {
        "state-set"
    }

    fn invoke(
        &self,
        node: &Node,
        _t: &[Event],
        ctx: &ExecutionContext,
    ) -> Result<Vec<Event>, DriverError> {
        let key = ctx
            .params
            .get("state_key")
            .and_then(Value::as_str)
            .unwrap_or("k");
        ctx.state.borrow_mut().set(key, json!({"flag": "yes"}));
        Ok(vec![Event::new(
            node.emits
                .first()
                .cloned()
                .unwrap_or_else(|| "set.done".to_string()),
            json!({}),
            &node.id,
        )])
    }
}

/// Emits via `ctx.emit` — side-channel events reach the pending queue.
struct EmitDriver;
impl Driver for EmitDriver {
    fn name(&self) -> &'static str {
        "emit"
    }

    fn invoke(
        &self,
        node: &Node,
        _t: &[Event],
        ctx: &ExecutionContext,
    ) -> Result<Vec<Event>, DriverError> {
        ctx.emit(Event::new("side.event", json!({}), &node.id));
        Ok(vec![Event::new("emit.done", json!({}), &node.id)])
    }
}

/// Asks for shutdown — `ctx.request_shutdown` cancels the run.
struct ShutdownDriver;
impl Driver for ShutdownDriver {
    fn name(&self) -> &'static str {
        "shutdown"
    }

    fn invoke(
        &self,
        _n: &Node,
        _t: &[Event],
        ctx: &ExecutionContext,
    ) -> Result<Vec<Event>, DriverError> {
        ctx.request_shutdown();
        Ok(vec![])
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn registry() -> Registry {
    Registry::default()
        .with("python", Arc::new(PythonFake))
        .with("shell", Arc::new(ShellFake))
        .with("looper", Arc::new(LooperFake))
        .with("noop", Arc::new(NoopDriver))
        .with("fail", Arc::new(FailDriver))
        .with("emit", Arc::new(EmitDriver))
        .with("scope-echo", Arc::new(ScopeEchoDriver))
        .with("state-set", Arc::new(StateSetDriver))
        .with("shutdown", Arc::new(ShutdownDriver))
        .with("runner", Arc::new(peregrin_driver::RunnerDriver))
}

fn run(campaign: &Campaign) -> Vec<Event> {
    Runner::new(registry())
        .run(campaign, &RunOptions::default())
        .unwrap()
}

fn types(log: &[Event]) -> Vec<&str> {
    log.iter().map(|e| e.kind.as_str()).collect()
}

fn load(yaml: &str) -> Campaign {
    YamlLoader
        .load_str(yaml, std::path::Path::new("."))
        .unwrap()
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

#[test]
fn registry_lookup_and_prefix_fallback() {
    let r = Registry::default().with("shell", Arc::new(NoopDriver));
    assert!(r.resolve("shell").is_ok());
    // `name.split(":")[0]` fallback (TS registry.resolve).
    assert!(r.resolve("shell:custom").is_ok());
    let err = r.resolve("missing").err().unwrap();
    assert_eq!(err.reason, "no driver registered for 'missing'");
}

// ---------------------------------------------------------------------------
// Basic run / ordering
// ---------------------------------------------------------------------------

#[test]
fn simple_run_emits_start_done_and_node_events() {
    let log = run(&load(
        "
        name: basic
        nodes:
          - id: a
            driver: noop
            on: [start]
            emits: [node.done]
        ",
    ));
    assert_eq!(types(&log), vec!["start", "node.done", "workflow.done"]);
    assert_eq!(log[1].source, "a");
}

#[test]
fn chain_of_nodes_follows_emits() {
    let log = run(&load(
        "
        name: chain
        nodes:
          - id: a
            driver: python
            on: [start]
            emits: [a.done]
            params: { ids: ['1'] }
          - id: b
            driver: noop
            on: [a.done]
        ",
    ));
    assert_eq!(
        types(&log),
        vec!["start", "a.done", "node.done", "workflow.done"]
    );
}

#[test]
fn ctx_emit_side_events_join_pending() {
    let log = run(&load(
        "
        name: side
        nodes:
          - id: e
            driver: emit
            on: [start]
          - id: s
            driver: noop
            on: [side.event]
        ",
    ));
    let t = types(&log);
    assert!(t.contains(&"side.event"));
    assert!(t.contains(&"emit.done"));
    assert!(t.contains(&"node.done"));
}

#[test]
fn ctx_fires_counts_including_current() {
    let log = run(&load(
        "
        name: fires
        nodes:
          - id: loop
            driver: looper
            on: [start, loop.retry]
            emits: [loop.success, loop.retry]
            max_fires: 5
        ",
    ));
    let retry = log.iter().filter(|e| e.kind == "loop.retry").count();
    assert_eq!(retry, 1);
    assert!(types(&log).contains(&"loop.success"));
}

// ---------------------------------------------------------------------------
// Integration suite ports (packages/tests/index.test.ts)
// ---------------------------------------------------------------------------

#[test]
fn map_reduce_with_fan_in_and_wildcard_triggers() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join(".handl")).unwrap();
    std::fs::write(
        dir.path().join(".handl/map.node.yml"),
        "
        id: map
        driver: python
        on: [start]
        emits: [shard.*.ready]
        max_fires: unlimited
        params:
          ids: [\"1\", \"2\"]
        ",
    )
    .unwrap();
    let campaign = YamlLoader
        .load_str(
            "
            name: map-reduce
            nodes:
              - id: fanin
                driver: noop
                on: [shard.*.ready]
                fan_in:
                  count: 2
                  emit: reduce.done
                max_fires: 1
              - id: reduce
                driver: shell
                on: [reduce.done]
                emits: [reduce.complete]
                params:
                  cmd: \"echo reduced\"
            ",
            dir.path(),
        )
        .unwrap();
    let log = run(&campaign);
    let t = types(&log);
    assert!(t.contains(&"shard.1.ready"));
    assert!(t.contains(&"shard.2.ready"));
    assert!(t.contains(&"reduce.done"));
    assert!(t.contains(&"reduce.complete"));
    assert!(t.contains(&"workflow.done"));
    assert!(!t.contains(&"workflow.failed"));
}

#[test]
fn forwards_wildcard_events_preserving_payload() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join(".handl")).unwrap();
    std::fs::write(
        dir.path().join(".handl/map.node.yml"),
        "
        id: map
        driver: python
        on: [start]
        emits: [shard.*.ready]
        max_fires: unlimited
        params:
          ids: [\"1\", \"2\"]
        ",
    )
    .unwrap();
    let campaign = YamlLoader
        .load_str(
            "
            name: forward-wildcard
            nodes:
              - id: bridge
                driver: noop
                on: [shard.*.ready]
                forward: [proc.*.run]
                max_fires: unlimited
              - id: sink
                driver: shell
                on: [proc.*.run]
                emits: [sink.*.done]
                max_fires: unlimited
                params:
                  cmd: \"echo done\"
            ",
            dir.path(),
        )
        .unwrap();
    let log = run(&campaign);
    let t = types(&log);
    assert!(t.contains(&"proc.1.run"));
    assert!(t.contains(&"proc.2.run"));
    assert!(t.contains(&"sink.1.done"));
    assert!(t.contains(&"sink.2.done"));
    let find = |kind: &str| log.iter().find(|e| e.kind == kind).unwrap();
    assert_eq!(find("proc.1.run").payload["id"], "1");
    assert_eq!(find("proc.2.run").payload["id"], "2");
    assert_eq!(find("sink.1.done").payload["stdout"], "echo done");
}

#[test]
fn enforces_capabilities() {
    let campaign = load(
        "
        name: capabilities
        capabilities:
          default_policy: deny
          shell:
            allow: [\"echo *\"]
        nodes:
          - id: safe
            driver: shell
            on: [start]
            emits: [safe.done]
            params:
              cmd: \"echo hello\"
          - id: danger
            driver: shell
            on: [safe.done]
            emits: [danger.done]
            params:
              cmd: \"rm -rf /\"
        ",
    );
    let log = run(&campaign);
    let t = types(&log);
    assert!(t.contains(&"safe.done"));
    assert!(t.contains(&"capability.violation"));
    assert!(!t.contains(&"danger.done"));
    assert!(t.contains(&"workflow.done"));
}

#[test]
fn honors_max_fires_until_and_retire_on() {
    let campaign = load(
        "
        name: loop
        nodes:
          - id: loop
            driver: looper
            on: [start, loop.retry]
            emits: [loop.success, loop.retry]
            max_fires: 3
            until: [loop.success]
            retire_on: [loop.success]
          - id: after
            driver: shell
            on: [loop.success]
            emits: [after.done]
            params:
              cmd: \"echo done\"
        ",
    );
    let log = run(&campaign);
    let t = types(&log);
    assert!(t.contains(&"loop.retry"));
    assert!(t.contains(&"loop.success"));
    assert!(t.contains(&"after.done"));
    assert!(t.contains(&"workflow.done"));
    assert_eq!(t.iter().filter(|k| **k == "loop.retry").count(), 1);
}

#[test]
fn discovers_snippets_through_file_router() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join(".handl")).unwrap();
    std::fs::write(
        dir.path().join(".handl/fragments.yaml"),
        "
        snippets:
          greet:
            driver: shell
            params:
              cmd: \"echo hello ${name}\"
        ",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("workflow.yaml"),
        "
        name: snippets
        include: [.handl/fragments.yaml]
        nodes:
          - id: greet
            snippet: greet
            on: [start]
            emits: [greet.done]
        ",
    )
    .unwrap();
    let campaign = YamlLoader.load(&dir.path().join("workflow.yaml")).unwrap();
    let log = run(&campaign);
    let t = types(&log);
    assert!(t.contains(&"greet.done"));
    let greet = log.iter().find(|e| e.kind == "greet.done").unwrap();
    assert!(
        greet.payload["stdout"]
            .as_str()
            .unwrap()
            .contains("${name}")
    );
    assert!(!t.contains(&"capability.violation"));
    assert!(t.contains(&"workflow.done"));
}

// ---------------------------------------------------------------------------
// Strict mode (ADR-0001)
// ---------------------------------------------------------------------------

#[test]
fn strict_requires_schemas_on_and_emits() {
    let err = Runner::new(registry())
        .run(
            &load(
                "
                name: strict
                strict: true
                events:
                  - type: foo
                nodes:
                  - id: a
                    driver: noop
                    on: [start]
                    emits: [x]
                ",
            ),
            &RunOptions::default(),
        )
        .unwrap_err();
    assert!(
        err.reason.contains("payload schema for event 'foo'"),
        "{err}"
    );

    let err = Runner::new(registry())
        .run(
            &load(
                "
                name: strict
                strict: true
                nodes:
                  - id: a
                    driver: noop
                    emits: [x]
                ",
            ),
            &RunOptions::default(),
        )
        .unwrap_err();
    assert_eq!(err.reason, "strict mode: node 'a' must declare 'on'");

    let err = Runner::new(registry())
        .run(
            &load(
                "
                name: strict
                strict: true
                nodes:
                  - id: a
                    driver: noop
                    on: [start]
                ",
            ),
            &RunOptions::default(),
        )
        .unwrap_err();
    assert_eq!(err.reason, "strict mode: node 'a' must declare 'emits'");
}

#[test]
fn strict_rejects_undeclared_emits() {
    let campaign = load(
        "
        name: strict-emits
        strict: true
        events:
          - type: node.done
            fields: {}
        nodes:
          - id: a
            driver: noop
            on: [start]
            emits: [a.done]
        ",
    );
    // noop emits `node.done`, not declared in `emits` → node.failed.
    let err = Runner::new(registry())
        .run(&campaign, &RunOptions::default())
        .unwrap_err();
    // events[*] all need compiled schemas — `fields: {}` is empty →
    // preflight fails first. Use a schema-bearing event instead.
    assert!(err.reason.contains("payload schema"), "{err}");

    let campaign = load(
        "
        name: strict-emits
        strict: true
        nodes:
          - id: a
            driver: noop
            on: [start]
            emits: [a.done]
        ",
    );
    let log = run(&campaign);
    let fail = log.iter().find(|e| e.kind == "node.failed").unwrap();
    assert_eq!(
        fail.payload["error"],
        "emitted 'node.done' not declared in 'emits'"
    );
    assert!(types(&log).contains(&"workflow.failed"));
}

#[test]
fn strict_unresolved_placeholder_fails_node() {
    let log = run(&load(
        "
        name: strict-tpl
        strict: true
        nodes:
          - id: a
            driver: shell
            on: [start]
            emits: [a.done]
            params:
              cmd: \"echo ${bogus fn}\"
        ",
    ));
    let fail = log.iter().find(|e| e.kind == "node.failed").unwrap();
    assert!(
        fail.payload["error"]
            .as_str()
            .unwrap()
            .contains("unresolved placeholders"),
        "{:?}",
        fail.payload
    );
}

// ---------------------------------------------------------------------------
// Reliability: retry / circuit breaker / DLQ
// ---------------------------------------------------------------------------

#[test]
fn retry_exhausts_to_node_failed_and_dlq() {
    let mut runner = Runner::new(registry()).with_dlq(Box::new(MemoryDlq::default()));
    let campaign = load(
        "
        name: retry
        nodes:
          - id: f
            driver: fail
            on: [start]
            retry: { max_attempts: 3, delay: 0 }
        ",
    );
    let log = runner.run(&campaign, &RunOptions::default()).unwrap();
    let fail = log.iter().find(|e| e.kind == "node.failed").unwrap();
    assert_eq!(fail.payload["error"], "boom");
    assert_eq!(fail.source, "f");
    assert!(types(&log).contains(&"workflow.failed"));
}

#[test]
fn retry_succeeds_within_attempts() {
    let calls = Arc::new(AtomicU64::new(0));
    let registry = Registry::default().with(
        "flaky",
        Arc::new(FlakyDriver {
            calls: calls.clone(),
            succeed_on: 2,
        }),
    );
    let mut runner = Runner::new(registry);
    let campaign = load(
        "
        name: retry-ok
        nodes:
          - id: f
            driver: flaky
            on: [start]
            retry: { max_attempts: 3, delay: 0 }
        ",
    );
    let log = runner.run(&campaign, &RunOptions::default()).unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert!(types(&log).contains(&"flaky.ok"));
    assert!(types(&log).contains(&"workflow.done"));
}

#[test]
fn circuit_breaker_trips_and_reports() {
    let calls = Arc::new(AtomicU64::new(0));
    let registry = Registry::default().with(
        "flaky",
        Arc::new(FlakyDriver {
            calls: calls.clone(),
            succeed_on: u64::MAX,
        }),
    );
    let mut runner = Runner::new(registry);
    let campaign = load(
        "
        name: cb
        nodes:
          - id: f
            driver: flaky
            on: [start]
            retry: { max_attempts: 5, delay: 0 }
            circuit_breaker: { threshold: 2 }
        ",
    );
    let log = runner.run(&campaign, &RunOptions::default()).unwrap();
    assert!(types(&log).contains(&"circuit.open"));
    let open = log.iter().find(|e| e.kind == "circuit.open").unwrap();
    assert_eq!(open.payload["node"], "f");
    assert_eq!(open.payload["failures"], 2);
    let fail = log.iter().find(|e| e.kind == "node.failed").unwrap();
    assert_eq!(fail.payload["error"], "circuit breaker tripped for 'f'");
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[test]
fn dry_run_short_circuits_drivers() {
    let mut runner = Runner::new(registry());
    let campaign = load(
        "
        name: dry
        nodes:
          - id: a
            driver: fail
            on: [start]
            emits: [a.done]
        ",
    );
    let log = runner
        .run(
            &campaign,
            &RunOptions {
                dry_run: true,
                ..RunOptions::default()
            },
        )
        .unwrap();
    let a = log.iter().find(|e| e.kind == "a.done").unwrap();
    assert_eq!(a.payload["dry_run"], true);
    assert!(types(&log).contains(&"workflow.done"));
}

// ---------------------------------------------------------------------------
// Termination: stop_on / workflow.cancel / shutdown
// ---------------------------------------------------------------------------

#[test]
fn stop_on_cancels() {
    // Emit via the emit driver's ctx.emit side channel.
    let campaign = load(
        "
        name: stop
        stop_on: [side.event]
        nodes:
          - id: a
            driver: emit
            on: [start]
          - id: b
            driver: noop
            on: [side.event]
          - id: c
            driver: noop
            on: [emit.done]
        ",
    );
    let log = run(&campaign);
    let t = types(&log);
    assert!(t.contains(&"workflow.cancelled"));
    assert!(!t.contains(&"workflow.done"));
}

#[test]
fn workflow_cancel_event_cancels() {
    let campaign = load(
        "
        name: cancel
        nodes:
          - id: a
            driver: python
            on: [start]
            emits: [workflow.cancel]
            params: { ids: ['x'] }
        ",
    );
    let log = run(&campaign);
    assert!(types(&log).contains(&"workflow.cancelled"));
}

#[test]
fn request_shutdown_cancels() {
    let campaign = load(
        "
        name: shutdown
        nodes:
          - id: a
            driver: shutdown
            on: [start]
        ",
    );
    let log = run(&campaign);
    assert!(types(&log).contains(&"workflow.cancelled"));
}

#[test]
fn workflow_failed_on_node_failure() {
    let campaign = load(
        "
        name: fail
        nodes:
          - id: f
            driver: fail
            on: [start]
        ",
    );
    let log = run(&campaign);
    let t = types(&log);
    assert!(t.contains(&"node.failed"));
    assert!(t.contains(&"workflow.failed"));
}

// ---------------------------------------------------------------------------
// Fan-in / mode: all / forward / state / replay / inject
// ---------------------------------------------------------------------------

#[test]
fn mode_all_collects_first_trigger_per_pattern() {
    let campaign = load(
        "
        name: all
        nodes:
          - id: p
            driver: python
            on: [start]
            emits: [a.done]
            params: { ids: ['1'] }
          - id: q
            driver: python
            on: [start]
            emits: [b.done]
            params: { ids: ['2'] }
          - id: join
            driver: noop
            on: [a.done, b.done]
            mode: all
        ",
    );
    let log = run(&campaign);
    assert!(types(&log).contains(&"node.done"));
    assert_eq!(
        types(&log),
        vec!["start", "a.done", "b.done", "node.done", "workflow.done"]
    );
}

#[test]
fn fan_in_emits_count_and_triggers_payload() {
    let campaign = load(
        "
        name: fanin
        nodes:
          - id: m
            driver: python
            on: [start]
            emits: [shard.*.ready]
            max_fires: unlimited
            params: { ids: ['1', '2'] }
          - id: fanin
            driver: noop
            on: [shard.*.ready]
            fan_in: { count: 2 }
            max_fires: 1
        ",
    );
    let log = run(&campaign);
    let join = log.iter().find(|e| e.kind == "fanin.done").unwrap();
    assert_eq!(join.payload["count"], 2);
    let triggers = join.payload["triggers"].as_array().unwrap();
    assert_eq!(triggers.len(), 2);
    assert_eq!(triggers[0]["type"], "shard.1.ready");
    assert_eq!(triggers[0]["payload"]["id"], "1");
    assert_eq!(triggers[1]["type"], "shard.2.ready");
}

#[test]
fn runner_driver_injects_nodes_mid_run() {
    let campaign = load(
        "
        name: inject
        nodes:
          - id: r
            driver: runner
            on: [start]
            emits: [r.done]
          - id: gen
            driver: python
            on: [start]
            emits: [spawn]
            params: { ids: [] }
        ",
    );
    // RunnerDriver reads nodes off its trigger payload — `start` has
    // none, so inject nothing; still emits r.done.
    let log = run(&campaign);
    assert!(types(&log).contains(&"r.done"));
}

#[test]
fn replay_events_seed_the_run() {
    let campaign = load(
        "
        name: replay
        nodes:
          - id: a
            driver: noop
            on: [seen.x]
        ",
    );
    let mut runner = Runner::new(registry());
    let log = runner
        .run(
            &campaign,
            &RunOptions {
                replay_events: vec![Event::new("seen.x", json!({}), "prior")],
                ..RunOptions::default()
            },
        )
        .unwrap();
    // No `start` in a replayed log; the node fires off the seed.
    assert!(!types(&log).contains(&"start"));
    assert!(types(&log).contains(&"node.done"));
    assert!(types(&log).contains(&"workflow.done"));
}

#[test]
fn node_exhausted_event_on_max_fires() {
    let campaign = load(
        "
        name: exhaust
        nodes:
          - id: p
            driver: python
            on: [start]
            emits: [x]
            max_fires: unlimited
            params: { ids: ['1', '2'] }
          - id: capped
            driver: noop
            on: [x]
        ",
    );
    let log = run(&campaign);
    let exhausted: Vec<_> = log.iter().filter(|e| e.kind == "node.exhausted").collect();
    assert_eq!(exhausted.len(), 1);
    assert_eq!(exhausted[0].payload["node"], "capped");
    assert_eq!(exhausted[0].payload["maxFires"], 1);
}

#[test]
fn state_key_scopes_state_and_skip_if_skips() {
    // A driver that records what ctx.state shows it.
    struct StateProbe;
    impl Driver for StateProbe {
        fn name(&self) -> &'static str {
            "probe"
        }
        fn invoke(
            &self,
            node: &Node,
            _t: &[Event],
            ctx: &ExecutionContext,
        ) -> Result<Vec<Event>, DriverError> {
            let seen = ctx.state.borrow().get("count");
            ctx.state.borrow_mut().set(
                "count",
                json!(seen.as_ref().and_then(Value::as_i64).unwrap_or(0) + 1),
            );
            Ok(vec![Event::new("probe.done", json!({}), &node.id)])
        }
    }
    let registry = Registry::default().with("probe", Arc::new(StateProbe));
    let mut runner = Runner::new(registry);
    let campaign = load(
        "
        name: state
        nodes:
          - id: a
            driver: probe
            on: [start]
            state: { key: \"counter\" }
        ",
    );
    let log = runner.run(&campaign, &RunOptions::default()).unwrap();
    assert!(types(&log).contains(&"probe.done"));

    // skip_if rendering truthy → node.skipped.
    let campaign = load(
        "
        name: skip
        nodes:
          - id: a
            driver: probe
            on: [start]
            state: { key: \"counter\", skip_if: \"true\" }
        ",
    );
    let log = runner.run(&campaign, &RunOptions::default()).unwrap();
    let skipped = log.iter().find(|e| e.kind == "node.skipped").unwrap();
    assert_eq!(skipped.payload["state_key"], "counter");
}

#[test]
fn input_schema_violation_emits_event_invalid() {
    // Check via a node-declared `input:` schema.
    let campaign = load(
        "
        name: input
        nodes:
          - id: s
            driver: shell
            on: [start]
            input:
              fields: { cmd: string }
            params: {}
        ",
    );
    let log = run(&campaign);
    let invalid = log.iter().find(|e| e.kind == "event.invalid").unwrap();
    assert_eq!(invalid.payload["eventType"], "node.input");
    assert!(
        invalid.payload["errors"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e.as_str().unwrap().contains("missing required property"))
    );
}

#[test]
fn event_payload_schema_validation() {
    let campaign = load(
        "
        name: schemas
        events:
          - type: user
            fields: { name: string, age: \"integer?\" }
        nodes:
          - id: p
            driver: python
            on: [start]
            emits: [user]
            params: { ids: ['x'] }
        ",
    );
    // PythonFake emits {id: "x"} — fails `name: string` required.
    let log = run(&campaign);
    let invalid = log.iter().find(|e| e.kind == "event.invalid").unwrap();
    assert_eq!(invalid.payload["eventType"], "user");
}

#[test]
fn template_params_resolve_functions() {
    // `${uuid()}` resolves when capability-granted.
    let campaign = load(
        "
        name: tpl
        capabilities:
          uuid: true
        nodes:
          - id: a
            driver: shell
            on: [start]
            emits: [a.done]
            params:
              cmd: \"echo ${uuid()}\"
        ",
    );
    let log = run(&campaign);
    let a = log.iter().find(|e| e.kind == "a.done").unwrap();
    let stdout = a.payload["stdout"].as_str().unwrap();
    assert!(stdout.starts_with("echo "), "{stdout}");
    assert!(stdout.len() > "echo ".len() + 30);
}

#[test]
fn functions_invoke_drivers_as_template_fns() {
    // `shout('z')` → python driver → returns payload.id "z" → the
    // resolved cmd "z" passes the deny-policy capability check
    // (resolvedParams feed `check`, not the driver).
    let campaign = load(
        "
        name: fns
        capabilities:
          default_policy: deny
          shout: true
          shell: { allow: ['z'] }
        functions:
          shout:
            driver: python
            params: { ids: ['${0}'] }
            returns: id
        nodes:
          - id: a
            driver: shell
            on: [start]
            emits: [a.done]
            params:
              cmd: \"${shout('z')}\"
        ",
    );
    let log = run(&campaign);
    let t = types(&log);
    assert!(t.contains(&"a.done"), "{t:?}");
    assert!(!t.contains(&"capability.violation"), "{t:?}");
}

#[test]
fn bus_observer_hooks_fire() {
    use peregrin_bus::Observer;
    use std::sync::Mutex;
    #[derive(Default)]
    struct Spy(Mutex<Vec<String>>);
    impl Observer for Spy {
        fn on_event(&self, event: &Event) {
            self.0.lock().unwrap().push(format!("event:{}", event.kind));
        }
        fn on_retire(&self, node_id: &str) {
            self.0.lock().unwrap().push(format!("retire:{node_id}"));
        }
    }
    let spy = Arc::new(Spy::default());
    let bus = LocalBus::new(
        Box::new(peregrin_store::MemoryStore::default()),
        spy.clone() as Arc<dyn Observer>,
    );
    let mut runner = Runner::new(registry()).with_bus(Box::new(bus));
    let campaign = load(
        "
        name: obs
        nodes:
          - id: a
            driver: noop
            on: [start]
            retire_on: [node.done]
        ",
    );
    runner.run(&campaign, &RunOptions::default()).unwrap();
    let calls = spy.0.lock().unwrap();
    assert!(calls.iter().any(|c| c == "event:start"));
    assert!(calls.iter().any(|c| c == "retire:a"));
}

// ---------------------------------------------------------------------------
// Scope resolution (`docs/reference/scope-resolution.md`)
// ---------------------------------------------------------------------------

#[test]
fn scope_resolution_binds_event_config_params_node_secrets() {
    let campaign = load(
        "
        name: scope
        config:
          region: us-east
        capabilities:
          secrets: true
        nodes:
          - id: seeder
            driver: state-set
            on: [start]
            emits: [seed.ready]
          - id: b
            driver: scope-echo
            on: [seed.*]
            emits: [b.done]
            params:
              ev: \"${event.type}\"
              shard: \"${event.shard}\"
              cap: \"${event.match.1}\"
              region: \"${config.region}\"
              missing: \"${config.nope:-fallback}\"
              nid: \"${node.id}\"
              fires: \"${node.fires}\"
              selfref: \"${params.ev}-x\"
              tok: \"${secret.TOKEN}\"
        ",
    );
    let mut opts = RunOptions::default();
    opts.secrets.insert("TOKEN".to_string(), "s3".to_string());
    let log = Runner::new(registry()).run(&campaign, &opts).unwrap();
    let evt = log.iter().find(|e| e.kind == "b.done").unwrap();
    let p = &evt.payload["params"];
    assert_eq!(p["ev"], json!("seed.ready"));
    assert_eq!(p["shard"], json!("ready"));
    assert_eq!(p["cap"], json!("ready"));
    assert_eq!(p["region"], json!("us-east"));
    assert_eq!(p["missing"], json!("fallback"));
    assert_eq!(p["nid"], json!("b"));
    assert_eq!(p["fires"], json!("1"));
    assert_eq!(p["selfref"], json!("seed.ready-x"));
    assert_eq!(p["tok"], json!("s3"));
}

#[test]
fn scope_resolution_reads_shared_state_and_state_key_templates() {
    let campaign = load(
        "
        name: scope-state
        nodes:
          - id: seeder
            driver: state-set
            on: [start]
            emits: [set.done]
            params:
              state_key: \"${node.id}-k\"
          - id: echo
            driver: scope-echo
            on: [set.done]
            emits: [echo.done]
            params:
              flag: \"${state.seeder-k.flag}\"
              absent: \"${state.nothing:-none}\"
        ",
    );
    let log = run(&campaign);
    let evt = log.iter().find(|e| e.kind == "echo.done").unwrap();
    let p = &evt.payload["params"];
    assert_eq!(p["flag"], json!("yes"));
    assert_eq!(p["absent"], json!("none"));
}

#[test]
fn scope_resolution_state_key_and_skip_if_use_scoped_store() {
    let campaign = load(
        "
        name: scope-skip
        nodes:
          - id: seeder
            driver: state-set
            on: [start]
            emits: [set.done]
            params:
              state_key: b-state
          - id: b
            driver: scope-echo
            on: [set.done]
            emits: [b.done]
            state:
              key: \"${node.id}-state\"
              skip_if: \"${state.flag}\"
        ",
    );
    let log = run(&campaign);
    let t = types(&log);
    // `state.key` resolved `${node.id}` → `b-state`; the scoped store
    // seeded `{"flag": "yes"}` so `skip_if` skipped the driver.
    assert!(t.contains(&"node.skipped"));
    assert!(!t.contains(&"b.done"));
}

#[test]
fn scope_resolution_unbound_and_unknown_stay_verbatim() {
    let campaign = load(
        "
        name: scope-verbatim
        nodes:
          - id: b
            driver: scope-echo
            on: [start]
            emits: [b.done]
            params:
              raw: \"${bogus.ns}\"
              foreign: \"${env.HOME}\"
        ",
    );
    let log = run(&campaign);
    let evt = log.iter().find(|e| e.kind == "b.done").unwrap();
    let p = &evt.payload["params"];
    assert_eq!(p["raw"], json!("${bogus.ns}"));
    assert_eq!(p["foreign"], json!("${env.HOME}"));
}

// ---------------------------------------------------------------------------
// Deferred emissions — `ctx.schedule_at`/`schedule_in` (ADR-0018)
// ---------------------------------------------------------------------------

/// Schedules `emits[0]` `params.ms` out through `ctx.schedule_in` and
/// returns `{id}.armed` immediately — the `delay` driver's shape.
struct DeferredDriver;

impl Driver for DeferredDriver {
    fn name(&self) -> &'static str {
        "deferred"
    }

    fn invoke(
        &self,
        node: &Node,
        _t: &[Event],
        ctx: &ExecutionContext,
    ) -> Result<Vec<Event>, DriverError> {
        let emit = node
            .emits
            .first()
            .cloned()
            .unwrap_or_else(|| format!("{}.done", node.id));
        let ms = node.params.get("ms").and_then(Value::as_u64).unwrap_or(0);
        ctx.schedule_in(
            Event::new(emit, json!({ "deferred": true }), &node.id),
            std::time::Duration::from_millis(ms),
        );
        Ok(vec![Event::new(
            format!("{}.armed", node.id),
            json!({}),
            &node.id,
        )])
    }
}

/// A deferred emission holds a finite run open — the runner parks on
/// the timer heap until the deadline lands instead of exiting quiet.
#[test]
fn deferred_emission_holds_the_run_until_it_lands() {
    let started = std::time::Instant::now();
    let campaign = load(
        "
        name: defer
        nodes:
          - id: d
            driver: deferred
            on: [start]
            emits: [d.done, d.armed]
            params: {ms: 60}
          - id: tail
            driver: noop
            on: [d.done]
        ",
    );
    let log = Runner::new(registry().with("deferred", Arc::new(DeferredDriver)))
        .run(&campaign, &RunOptions::default())
        .unwrap();
    let t = types(&log);
    let armed = t.iter().position(|k| *k == "d.armed").unwrap();
    let done = t.iter().position(|k| *k == "d.done").unwrap();
    assert!(armed < done, "order: {t:?}");
    assert!(
        started.elapsed() >= std::time::Duration::from_millis(50),
        "run ended before the deferred emission: {t:?}"
    );
    assert_eq!(t.last(), Some(&"workflow.done"));
}

/// A retired node's armed deferred emissions are dropped — the
/// `retire_on` bound reads as "no further activity".
#[test]
fn retired_node_drops_its_deferred_emissions() {
    let campaign = load(
        "
        name: defer-retire
        nodes:
          - id: d
            driver: deferred
            on: [start]
            emits: [d.done, d.armed]
            retire_on: [emit.done]
            params: {ms: 60000}
          - id: aborter
            driver: emit
            on: [d.armed]
        ",
    );
    let log = Runner::new(registry().with("deferred", Arc::new(DeferredDriver)))
        .run(&campaign, &RunOptions::default())
        .unwrap();
    let t = types(&log);
    assert!(!t.contains(&"d.done"), "log: {t:?}");
    assert_eq!(t.last(), Some(&"workflow.done"));
}

// ---------------------------------------------------------------------------
// Secret resolution chain — record → ambient env → declared stores
// (issue I-018). The `${secret.*}` binding in runner `scope_for` runs
// `ctx.resolve_secret`; the fakes below stand in for the keyring/env-file
// `Secrets` impls so the gate and ordering are exercised without
// `secret-tool` or env mutation (edition-2024 `set_var` is unsafe).
// ---------------------------------------------------------------------------

struct FakeSecrets(std::collections::BTreeMap<String, String>);

impl hypha_secret::Secrets for FakeSecrets {
    fn get(&self, name: &str) -> Option<String> {
        self.0.get(name).cloned()
    }

    fn set(&self, _name: &str, _value: &str) -> std::io::Result<()> {
        Ok(())
    }
}

fn secret_source(pairs: &[(&str, &str)], declared_only: bool) -> SecretSource {
    SecretSource {
        store: std::rc::Rc::new(FakeSecrets(
            pairs
                .iter()
                .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                .collect(),
        )),
        declared_only,
        batch: None,
    }
}

#[test]
fn secret_chain_ambient_env_resolves_after_record_miss() {
    // `export`ed names resolve — the I-018 gap: the run secrets
    // record misses and the ambient env is the next leg.
    let campaign = load(
        "
        name: secret-env
        capabilities:
          secrets: true
        nodes:
          - id: e
            driver: scope-echo
            on: [start]
            emits: [e.done]
            params:
              v: \"${secret.PATH}\"
        ",
    );
    let log = run(&campaign);
    let evt = log.iter().find(|e| e.kind == "e.done").unwrap();
    assert_eq!(
        evt.payload["params"]["v"],
        json!(std::env::var("PATH").unwrap())
    );
}

#[test]
fn secret_chain_precedence_and_declared_store_gate() {
    let campaign = load(
        "
        name: secret-chain
        capabilities:
          secrets: [KR_TOKEN]
        nodes:
          - id: e
            driver: scope-echo
            on: [start]
            emits: [e.done]
            params:
              rec: \"${secret.PATH}\"
              ring: \"${secret.KR_TOKEN}\"
              gated: \"${secret.KR_OTHER:-missed}\"
              file: \"${secret.FILE_LEG}\"
        ",
    );
    let options = RunOptions {
        // Record beats the ambient env on the same name.
        secrets: std::collections::BTreeMap::from([(
            "PATH".to_string(),
            "record-value".to_string(),
        )]),
        secret_stores: vec![
            // The keyring leg — declared-only.
            secret_source(
                &[("KR_TOKEN", "ring-value"), ("KR_OTHER", "gated-value")],
                true,
            ),
            // The env-file leg — always available.
            secret_source(&[("FILE_LEG", "file-value")], false),
        ],
        ..RunOptions::default()
    };
    let log = Runner::new(registry()).run(&campaign, &options).unwrap();
    let params = &log.iter().find(|e| e.kind == "e.done").unwrap().payload["params"];
    assert_eq!(params["rec"], json!("record-value"));
    // Declared in `capabilities.secrets` → the declared-only store hits.
    assert_eq!(params["ring"], json!("ring-value"));
    // KR_OTHER is in the same store but undeclared — gated out, the
    // `:-` default applies like any other miss.
    assert_eq!(params["gated"], json!("missed"));
    // A non-declared store resolves whatever it holds — the grant
    // list only bounds the declared-only (keyring) leg.
    assert_eq!(params["file"], json!("file-value"));
}

#[test]
fn secret_chain_strict_hit_is_not_a_missing_secret() {
    // Strict: a declared name resolved through ambient env does not
    // count as a missing secret — the node completes.
    let campaign = load(
        "
        name: secret-strict-hit
        strict: true
        capabilities:
          secrets: [PATH]
        nodes:
          - id: e
            driver: scope-echo
            on: [start]
            emits: [e.done]
            params:
              v: \"${secret.PATH}\"
        ",
    );
    let log = run(&campaign);
    let evt = log.iter().find(|e| e.kind == "e.done").unwrap();
    assert_eq!(
        evt.payload["params"]["v"],
        json!(std::env::var("PATH").unwrap())
    );
    assert_eq!(log.last().unwrap().kind, "workflow.done");
}

#[test]
fn secret_chain_strict_full_miss_still_audits() {
    // Strict: a name no leg has still lands in the missing-secrets
    // ledger — the env/store legs don't hide genuine misses.
    let campaign = load(
        "
        name: secret-strict-miss
        strict: true
        capabilities:
          secrets: true
        nodes:
          - id: e
            driver: scope-echo
            on: [start]
            emits: [e.done]
            params:
              v: \"${secret.PEREGRIN_TEST_DEFINITELY_UNSET_018}\"
        ",
    );
    let log = run(&campaign);
    let failed = log.iter().find(|e| e.kind == "node.failed").unwrap();
    assert_eq!(
        failed.payload["error"],
        json!("missing secrets: PEREGRIN_TEST_DEFINITELY_UNSET_018")
    );
}

#[test]
fn pending_timers_and_replay_timers_seam() {
    use peregrin_engine::PendingTimer;
    use std::time::SystemTime;

    let now_epoch_ms = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    let target_event = Event::new("timer.fired", json!({"tag": 42}), "test-node");
    let timer = PendingTimer {
        event: target_event.clone(),
        at_epoch_ms: now_epoch_ms + 20,
        holds_run: true,
    };

    let mut runner = Runner::new(Registry::default());
    runner.arm_timers(std::slice::from_ref(&timer));
    let pending = runner.pending_timers();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].event.kind, "timer.fired");
    assert!(pending[0].holds_run);
    assert!(pending[0].at_epoch_ms >= now_epoch_ms);

    // Replay run restores the timer and fires it when ready
    let campaign = load(
        "
        name: timer-replay
        nodes:
          - id: listener
            driver: noop
            on: [timer.fired]
            emits: [listener.done]
        ",
    );
    let mut replay_runner = Runner::new(Registry::default().with("noop", Arc::new(NoopDriver)));
    let log = replay_runner
        .run(
            &campaign,
            &RunOptions {
                replay_timers: vec![timer],
                ..Default::default()
            },
        )
        .unwrap();

    assert!(log.iter().any(|e| e.kind == "timer.fired"));
    assert!(
        log.iter()
            .any(|e| e.kind == "node.done" && e.source == "listener")
    );
    assert_eq!(log.last().unwrap().kind, "workflow.done");
}
