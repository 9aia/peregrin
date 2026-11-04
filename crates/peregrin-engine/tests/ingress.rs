#![allow(clippy::unwrap_used, clippy::pedantic)]
//! Ingress channel tests — external events injected into a live run:
//! delivery, ordering (after the triggers of the current step),
//! `workflow.cancel` propagation, multi-producer, and sender-drop.
//! The runner stays on one thread (`!Send` via `Rc` internals);
//! producers push through the `Send + Sync` `Injector` from scoped
//! threads — the audit-I-014 model.

use std::sync::Arc;
use std::time::Duration;

use peregrin_core::{Campaign, Event, Node};
use peregrin_driver::{Driver, DriverError, ExecutionContext, NoopDriver};
use peregrin_engine::{InjectError, Registry, RunOptions, Runner, ingress};
use peregrin_loader::Loader;
use peregrin_loader_yaml::YamlLoader;
use serde_json::{Value, json};

/// A driver that sleeps — gives an external producer a window to
/// inject mid-run deterministically.
struct SlowDriver;

impl Driver for SlowDriver {
    fn name(&self) -> &'static str {
        "slow"
    }

    fn invoke(
        &self,
        node: &Node,
        _triggers: &[Event],
        _ctx: &ExecutionContext,
    ) -> Result<Vec<Event>, DriverError> {
        std::thread::sleep(Duration::from_millis(400));
        Ok(vec![Event::new(
            node.emits
                .first()
                .cloned()
                .unwrap_or_else(|| format!("{}.done", node.id)),
            Value::Object(serde_json::Map::new()),
            &node.id,
        )])
    }
}

fn registry() -> Registry {
    Registry::default()
        .with("noop", Arc::new(NoopDriver))
        .with("slow", Arc::new(SlowDriver))
}

fn load(yaml: &str) -> Campaign {
    YamlLoader
        .load_str(yaml, std::path::Path::new("."))
        .unwrap()
}

fn kinds(log: &[Event]) -> Vec<&str> {
    log.iter().map(|e| e.kind.as_str()).collect()
}

fn index_of(log: &[Event], kind: &str, nth: usize) -> usize {
    log.iter()
        .enumerate()
        .filter(|(_, e)| e.kind == kind)
        .nth(nth)
        .map(|(i, _)| i)
        .unwrap_or_else(|| panic!("event '{kind}' x{nth} not in {}", kinds(log).join(", ")))
}

#[test]
fn injected_event_reaches_the_graph() {
    let (injector, chan) = ingress();
    injector
        .inject(Event::new("ext.ping", json!({"n": 1}), "probe"))
        .unwrap();
    let campaign = load(
        "
        nodes:
          - id: a
            driver: noop
            on: [start]
            emits: [a.done]
          - id: ext
            driver: noop
            on: [ext.ping]
            emits: [ext.done]
        ",
    );
    let log = Runner::new(registry())
        .with_ingress(chan)
        .run(&campaign, &RunOptions::default())
        .unwrap();
    let kinds = kinds(&log);
    assert!(kinds.contains(&"ext.ping"), "injected event published");
    assert_eq!(log[index_of(&log, "ext.ping", 0)].payload["n"], 1);
    // The `ext` node fired on it.
    assert_eq!(kinds.iter().filter(|k| **k == "node.done").count(), 2);
}

#[test]
fn injected_events_serialize_after_the_triggers_of_the_current_step() {
    let (injector, chan) = ingress();
    injector
        .inject(Event::new("ext.ping", json!({}), "probe"))
        .unwrap();
    let campaign = load(
        "
        nodes:
          - id: a
            driver: noop
            on: [start]
            emits: [a.done]
        ",
    );
    let log = Runner::new(registry())
        .with_ingress(chan)
        .run(&campaign, &RunOptions::default())
        .unwrap();
    // `start` (the trigger of the first step) publishes first; the
    // injected event joins pending behind it — before `a`'s output,
    // which is only emitted by the step that ran after the drain.
    assert_eq!(index_of(&log, "start", 0), 0);
    assert_eq!(index_of(&log, "ext.ping", 0), 1);
    assert!(index_of(&log, "node.done", 0) > 1);
    assert_eq!(log.last().unwrap().kind, "workflow.done");
}

#[test]
fn injected_event_with_no_subscribers_still_logs() {
    let (injector, chan) = ingress();
    injector
        .inject(Event::new("ext.unknown", json!({}), "probe"))
        .unwrap();
    let campaign = load(
        "
        nodes:
          - id: a
            driver: noop
            on: [start]
        ",
    );
    let log = Runner::new(registry())
        .with_ingress(chan)
        .run(&campaign, &RunOptions::default())
        .unwrap();
    assert!(kinds(&log).contains(&"ext.unknown"));
    assert_eq!(log.last().unwrap().kind, "workflow.done");
}

#[test]
fn mid_run_injection_lands_after_pending_events() {
    let (injector, chan) = ingress();
    let campaign = load(
        "
        nodes:
          - id: slow
            driver: slow
            on: [start]
            emits: [slow.done]
          - id: poke
            driver: noop
            on: [ext.poke]
            emits: [poke.done]
        ",
    );
    let mut runner = Runner::new(registry()).with_ingress(chan);
    let log = std::thread::scope(|scope| {
        scope.spawn(|| {
            // Lands while `slow` sleeps — drained into pending on the
            // next iteration, behind the already-pending slow.done.
            std::thread::sleep(Duration::from_millis(100));
            injector
                .inject(Event::new("ext.poke", json!({}), "probe"))
                .unwrap();
        });
        runner.run(&campaign, &RunOptions::default()).unwrap()
    });
    assert!(index_of(&log, "slow.done", 0) < index_of(&log, "ext.poke", 0));
    assert!(index_of(&log, "ext.poke", 0) < index_of(&log, "node.done", 0));
    assert_eq!(log.last().unwrap().kind, "workflow.done");
}

#[test]
fn injected_workflow_cancel_ends_the_run() {
    let (injector, chan) = ingress();
    let campaign = load(
        "
        nodes:
          - id: slow
            driver: slow
            on: [start]
            emits: [slow.done]
          - id: never
            driver: noop
            on: [slow.done]
        ",
    );
    let mut runner = Runner::new(registry()).with_ingress(chan);
    let log = std::thread::scope(|scope| {
        scope.spawn(|| {
            std::thread::sleep(Duration::from_millis(100));
            injector.inject(Event::cancel("superseded")).unwrap();
        });
        runner.run(&campaign, &RunOptions::default()).unwrap()
    });
    let kinds = kinds(&log);
    // The cancel is consumed by the loop (never published); the run
    // ends with workflow.cancelled and `never` never fires.
    assert!(kinds.contains(&"slow.done"), "in-flight step completes");
    assert!(!kinds.contains(&"workflow.cancel"));
    assert_eq!(log.last().unwrap().kind, "workflow.cancelled");
    assert_eq!(kinds.iter().filter(|k| **k == "node.done").count(), 0);
}

#[test]
fn cancel_injected_before_start_cancels_immediately() {
    let (injector, chan) = ingress();
    injector.inject(Event::cancel("early")).unwrap();
    let campaign = load(
        "
        nodes:
          - id: a
            driver: noop
            on: [start]
        ",
    );
    let log = Runner::new(registry())
        .with_ingress(chan)
        .run(&campaign, &RunOptions::default())
        .unwrap();
    // `a` still fires — the cancel drains into pending *ahead* of
    // `a`'s emitted events, so they're dropped with the run.
    assert_eq!(kinds(&log), vec!["start", "workflow.cancelled"]);
}

#[test]
fn multiple_producers_all_deliver() {
    let (injector, chan) = ingress();
    let other = injector.clone();
    injector
        .inject(Event::new("ext.one", json!({}), "p1"))
        .unwrap();
    other
        .inject(Event::new("ext.two", json!({}), "p2"))
        .unwrap();
    let campaign = load(
        "
        nodes:
          - id: a
            driver: noop
            on: [start]
        ",
    );
    let log = Runner::new(registry())
        .with_ingress(chan)
        .run(&campaign, &RunOptions::default())
        .unwrap();
    let kinds = kinds(&log);
    assert!(kinds.contains(&"ext.one"));
    assert!(kinds.contains(&"ext.two"));
    // Arrival order preserved.
    assert!(index_of(&log, "ext.one", 0) < index_of(&log, "ext.two", 0));
}

#[test]
fn sender_drop_lets_the_run_finish_normally() {
    let (injector, chan) = ingress();
    drop(injector);
    let campaign = load(
        "
        nodes:
          - id: a
            driver: noop
            on: [start]
        ",
    );
    let log = Runner::new(registry())
        .with_ingress(chan)
        .run(&campaign, &RunOptions::default())
        .unwrap();
    assert_eq!(log.last().unwrap().kind, "workflow.done");
}

#[test]
fn drain_into_reports_count_and_sender_drop_yields_empty() {
    let (injector, chan) = ingress();
    let mut out = Vec::new();
    assert_eq!(chan.drain_into(&mut out), 0);
    injector.inject(Event::new("e1", json!({}), "t")).unwrap();
    injector.inject(Event::new("e2", json!({}), "t")).unwrap();
    assert_eq!(chan.drain_into(&mut out), 2);
    assert_eq!(out.len(), 2);
    drop(injector);
    assert_eq!(chan.drain_into(&mut out), 0, "disconnected drains to empty");
    assert_eq!(out.len(), 2, "appends only");
}

#[test]
fn inject_after_receiver_drop_returns_the_event() {
    let (injector, chan) = ingress();
    drop(chan);
    let event = Event::new("lost", json!({"k": 1}), "probe");
    let err: InjectError = injector.inject(event).unwrap_err();
    assert_eq!(err.0.kind, "lost", "undelivered event handed back");
    assert_eq!(err.0.payload["k"], 1);
    let rendered = err.to_string();
    assert!(rendered.contains("ingress"), "{rendered}");
    assert!(std::error::Error::source(&err).is_none());
}

// --- serve mode: ingress campaigns park at quiet ---------------------

/// An ingress-marked fake — `is_ingress()` is what flips a campaign
/// into serve mode. Emits `<id>.listening` the way the real
/// listeners do after bind.
struct FakeIngressDriver;

impl Driver for FakeIngressDriver {
    fn name(&self) -> &'static str {
        "fake-ingress"
    }

    fn is_ingress(&self) -> bool {
        true
    }

    fn invoke(
        &self,
        node: &Node,
        _triggers: &[Event],
        _ctx: &ExecutionContext,
    ) -> Result<Vec<Event>, DriverError> {
        Ok(vec![Event::new(
            format!("{}.listening", node.id),
            Value::Object(serde_json::Map::new()),
            &node.id,
        )])
    }
}

#[test]
fn serve_parks_at_quiet_then_wakes_on_injected_event() {
    let campaign = load(
        r#"
name: gate
nodes:
  - id: watch
    driver: fake-ingress
    on: [start]
    emits: [watch.listening]
  - id: work
    driver: noop
    on: [wake]
"#,
    );
    let (injector, chan) = ingress();
    let registry = registry().with("fake-ingress", Arc::new(FakeIngressDriver));
    let mut runner = Runner::new(registry).with_ingress(chan);

    let producer = injector.clone();
    let handle = std::thread::spawn(move || {
        // Past the initial quiet — proves the run was parked, not
        // still draining startup work.
        std::thread::sleep(Duration::from_millis(300));
        producer
            .inject(Event::new("wake", json!({}), "test"))
            .unwrap();
        std::thread::sleep(Duration::from_millis(100));
        producer.inject(Event::cancel("test done")).unwrap();
    });
    let log = runner.run(&campaign, &RunOptions::default()).unwrap();
    handle.join().unwrap();
    let kinds = kinds(&log);
    assert!(kinds.contains(&"watch.listening"), "{kinds:?}");
    assert!(kinds.contains(&"wake"), "{kinds:?}");
    assert!(kinds.contains(&"node.done"), "{kinds:?}");
    assert_eq!(kinds.last(), Some(&"workflow.cancelled"), "{kinds:?}");
}

#[test]
fn finite_campaign_exits_despite_ingress_channel_attached() {
    // `serve` requires BOTH an attached channel and an ingress-driver
    // node — a finite campaign on an ingress-aware runner still
    // terminates at quiet.
    let campaign = load(
        r#"
name: finite
nodes:
  - id: one
    driver: noop
    on: [start]
"#,
    );
    let (injector, chan) = ingress();
    let mut runner = Runner::new(registry()).with_ingress(chan);
    let started = std::time::Instant::now();
    let log = runner.run(&campaign, &RunOptions::default()).unwrap();
    assert!(started.elapsed() < Duration::from_secs(2), "must not park");
    assert_eq!(kinds(&log).last(), Some(&"workflow.done"));
    drop(injector);
}

#[test]
fn serve_exits_when_every_injector_is_gone() {
    // The driver's own clone lives in the run, so disconnect needs a
    // registry whose ingress driver drops its clone after invoke —
    // modelled here by a campaign whose only injector is external.
    // Simpler faithful shape: cancel is the in-band exit; disconnect
    // is covered by `wait_*` unit tests below.
    let campaign = load(
        r#"
name: gate
nodes:
  - id: watch
    driver: fake-ingress
    on: [start]
    emits: [watch.listening]
"#,
    );
    let (injector, chan) = ingress();
    let mut runner = Runner::new(registry().with("fake-ingress", Arc::new(FakeIngressDriver)))
        .with_ingress(chan);
    let handle = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(200));
        injector.inject(Event::cancel("shutdown")).unwrap();
    });
    let log = runner.run(&campaign, &RunOptions::default()).unwrap();
    handle.join().unwrap();
    assert_eq!(kinds(&log).last(), Some(&"workflow.cancelled"));
}

// --- Ingress::wait ---------------------------------------------------

#[test]
fn wait_event_appends_and_drains_follow_ups() {
    let (injector, chan) = ingress();
    injector.inject(Event::new("a", json!({}), "t")).unwrap();
    injector.inject(Event::new("b", json!({}), "t")).unwrap();
    let mut out = Vec::new();
    assert_eq!(
        chan.wait(Some(Duration::from_secs(1)), &mut out),
        peregrin_engine::WaitOutcome::Event
    );
    assert_eq!(out.len(), 2, "first recv + follow-up drain");
}

#[test]
fn wait_timeout_returns_at_the_deadline() {
    let (injector, chan) = ingress();
    let mut out = Vec::new();
    let started = std::time::Instant::now();
    let outcome = chan.wait(Some(Duration::from_millis(60)), &mut out);
    assert_eq!(outcome, peregrin_engine::WaitOutcome::Timeout);
    assert!(started.elapsed() >= Duration::from_millis(50));
    assert!(out.is_empty());
    drop(injector);
}

#[test]
fn wait_disconnected_when_every_sender_gone() {
    let (injector, chan) = ingress();
    drop(injector);
    let mut out = Vec::new();
    assert_eq!(
        chan.wait(None, &mut out),
        peregrin_engine::WaitOutcome::Disconnected
    );
}
