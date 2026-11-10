#![allow(clippy::unwrap_used, clippy::expect_used, clippy::pedantic)]
//! RunSupervisor tests — the queue + ingress composition: submit
//! hands out PendingRuns, supersede signals become `workflow.cancel`
//! injections on the active run's channel, and queue-behind /
//! latest-wins dedupe run through the real engine.

use std::time::Duration;

use peregrin_core::{Campaign, Event};
use peregrin_engine::RunOptions;
use peregrin_loader::Loader;
use peregrin_loader_yaml::YamlLoader;
use peregrin_node::{campaign, supervisor::RunSupervisor};
use peregrin_queue::{Disposition, Mode, Supersede};
use serde_json::{Value, json};

fn policy(mode: Mode, cancel_running: bool) -> Supersede {
    Supersede {
        key: "{repo}".to_string(),
        mode,
        cancel_running,
    }
}

fn trigger(repo: &str, extra: Value) -> Event {
    let mut payload = extra;
    payload["repo"] = json!(repo);
    Event::new("push", payload, "github")
}

fn load(yaml: &str) -> Campaign {
    YamlLoader
        .load_str(yaml, std::path::Path::new("."))
        .unwrap()
}

/// A `push`-triggered noop campaign — the trigger event itself
/// readies the node (ingress-delivered, not `start`).
const PUSH_NOOP: &str = "
nodes:
  - id: build
    driver: noop
    on: [push]
    emits: [build.done]
";

#[test]
fn idle_submit_hands_out_a_pending_run() {
    let campaign = load(PUSH_NOOP);
    let mut supervisor = RunSupervisor::new(policy(Mode::LatestWins, true));
    assert!(supervisor.is_idle());

    let outcome = supervisor.submit(trigger("a/b", json!({"sha": "1"})));
    assert!(!outcome.cancelled_active);
    assert_eq!(outcome.decision.key, "a/b");
    let pending = outcome.start.expect("idle supervisor starts now");
    assert_eq!(pending.job.key, "a/b");
    assert!(!supervisor.is_idle());

    // The trigger was pre-loaded on the run's ingress channel — it
    // publishes into the live run and readies `build`.
    let log = campaign::runner()
        .with_ingress(pending.ingress)
        .run(&campaign, &RunOptions::default())
        .unwrap();
    let kinds: Vec<&str> = log.iter().map(|e| e.kind.as_str()).collect();
    assert!(kinds.contains(&"push"), "trigger event delivered");
    assert!(kinds.contains(&"node.done"), "push readied `build`");
    assert_eq!(log.last().unwrap().kind, "workflow.done");

    assert!(supervisor.finish().is_none());
    assert!(supervisor.is_idle());
}

#[test]
fn queue_behind_waits_for_the_active_run() {
    let mut supervisor = RunSupervisor::new(policy(Mode::Fifo, false));
    let first = supervisor
        .submit(trigger("a", json!({})))
        .start
        .expect("first run starts");
    // Submit while active, cancel_running off — queues, no cancel.
    let outcome = supervisor.submit(trigger("b", json!({})));
    assert!(!outcome.cancelled_active);
    assert_eq!(outcome.decision.disposition, Disposition::Queued);
    assert!(outcome.start.is_none(), "sequential supervisor");
    assert_eq!(
        supervisor.queue().pending_len(),
        1,
        "b waits behind the active run"
    );
    assert!(supervisor.queue().active().is_some());
    drop(first);

    let next = supervisor.finish().expect("b runs next");
    assert_eq!(next.job.key, "b");
    assert!(supervisor.finish().is_none());
    assert!(supervisor.is_idle());
}

#[test]
fn latest_wins_dedupes_pending_behind_the_active_run() {
    let mut supervisor = RunSupervisor::new(policy(Mode::LatestWins, false));
    let _active = supervisor.submit(trigger("a", json!({}))).start.unwrap();
    supervisor.submit(trigger("a", json!({"sha": "old"})));
    let outcome = supervisor.submit(trigger("a", json!({"sha": "new"})));
    let Disposition::ReplacedPending(dropped) = outcome.decision.disposition else {
        panic!("latest-wins should replace, got {:?}", outcome.decision);
    };
    assert_eq!(dropped.event.payload["sha"], "old");

    let next = supervisor.finish().unwrap();
    assert_eq!(next.job.event.payload["sha"], "new");
    assert!(supervisor.finish().is_none());
}

#[test]
fn cancel_running_injects_workflow_cancel_into_the_active_run() {
    // A slow run in flight; the second trigger supersedes it.
    let campaign = load(
        "
        nodes:
          - id: build
            driver: shell
            on: [push]
            emits: [build.done]
            params:
              cmd: \"sleep 0.5\"
              emit: {\"0\": build.done}
        ",
    );
    let mut supervisor = RunSupervisor::new(policy(Mode::LatestWins, true));
    let pending = supervisor.submit(trigger("a", json!({}))).start.unwrap();
    let mut runner = campaign::runner().with_ingress(pending.ingress);

    let log = std::thread::scope(|scope| {
        scope.spawn(|| {
            // Mid-run supersede: the supervisor pushes
            // `workflow.cancel` into the run's own ingress channel.
            std::thread::sleep(Duration::from_millis(100));
            let outcome = supervisor.submit(trigger("a", json!({"sha": "2"})));
            assert!(outcome.decision.cancel_active);
            assert!(outcome.cancelled_active, "cancel was delivered");
            assert!(outcome.start.is_none(), "still busy");
        });
        runner.run(&campaign, &RunOptions::default()).unwrap()
    });
    let kinds: Vec<&str> = log.iter().map(|e| e.kind.as_str()).collect();
    assert_eq!(log.last().unwrap().kind, "workflow.cancelled");
    assert!(kinds.contains(&"push"), "the first trigger ran");
    assert!(kinds.contains(&"build.done"), "in-flight step finished");

    // The superseding job is now queued behind nothing — finish
    // hands it out and it runs to done.
    let next = supervisor.finish().expect("superseded-by run queued");
    assert_eq!(next.job.event.payload["sha"], "2");
    let log = campaign::runner()
        .with_ingress(next.ingress)
        .run(&campaign, &RunOptions::default())
        .unwrap();
    assert_eq!(log.last().unwrap().kind, "workflow.done");
    assert!(supervisor.finish().is_none());
    assert!(supervisor.is_idle());
}

#[test]
fn caller_held_injector_still_emits_mid_run() {
    let campaign = load(
        "
        nodes:
          - id: build
            driver: shell
            on: [push]
            emits: [build.done]
            params:
              cmd: \"sleep 0.5\"
              emit: {\"0\": build.done}
          - id: extra
            driver: noop
            on: [ext.extra]
        ",
    );
    let mut supervisor = RunSupervisor::new(policy(Mode::Fifo, false));
    let pending = supervisor.submit(trigger("a", json!({}))).start.unwrap();
    let injector = pending.injector.clone();
    let mut runner = campaign::runner().with_ingress(pending.ingress);

    let log = std::thread::scope(|scope| {
        scope.spawn(|| {
            std::thread::sleep(Duration::from_millis(100));
            injector
                .inject(Event::new("ext.extra", json!({}), "probe"))
                .unwrap();
        });
        runner.run(&campaign, &RunOptions::default()).unwrap()
    });
    let kinds: Vec<&str> = log.iter().map(|e| e.kind.as_str()).collect();
    assert!(kinds.contains(&"ext.extra"), "external producer delivered");
    assert_eq!(log.last().unwrap().kind, "workflow.done");
}

// ---------------------------------------------------------------------------
// Durable queue store — restart replay (C1a)
// ---------------------------------------------------------------------------

#[test]
fn store_replays_pending_and_running_across_a_restart() {
    use std::sync::Arc;

    use peregrin_queue::{MemoryQueueStore, QueueStore};

    let store = Arc::new(MemoryQueueStore::new());
    let policy = policy(Mode::Fifo, false);

    // "Process 1": A starts (running marker), B + C stay pending, then
    // the process "dies" — the supervisor is dropped mid-flight.
    {
        let mut s1 = RunSupervisor::with_store(policy.clone(), store.clone()).unwrap();
        let _active = s1.submit(trigger("a", json!({"sha": "A"}))).start.unwrap();
        s1.submit(trigger("b", json!({"sha": "B"})));
        s1.submit(trigger("c", json!({"sha": "C"})));
        drop(s1); // kill — no finish, no drain
    }

    // What the dead process left behind: A in-flight, B+C pending.
    let persisted = store.load().unwrap();
    assert_eq!(persisted.running.unwrap().event.payload["sha"], "A");
    assert_eq!(persisted.pending.len(), 2);

    // "Process 2": same store — the snapshot replays before any new
    // trigger. A re-queues at the FRONT (at-least-once), then B, C.
    let mut s2 = RunSupervisor::with_store(policy, store).unwrap();
    assert!(s2.queue().active().is_none());
    assert_eq!(s2.queue().pending_len(), 3);

    // A new trigger arrives — the restored front job starts first,
    // the new one queues behind everything.
    let outcome = s2.submit(trigger("d", json!({"sha": "D"})));
    let first = outcome.start.expect("restored queue drains");
    assert_eq!(first.job.event.payload["sha"], "A");

    let shas: Vec<String> = [first]
        .into_iter()
        .chain(std::iter::from_fn(|| s2.finish()))
        .map(|p| p.job.event.payload["sha"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(
        shas,
        ["A", "B", "C", "D"],
        "replay order: interrupted, pending, new"
    );
    assert!(s2.is_idle());
}

#[test]
fn store_is_optional_and_absent_means_memory_only() {
    // Regression: no store configured — identical in-memory behavior.
    let mut supervisor = RunSupervisor::new(policy(Mode::Fifo, false));
    let _active = supervisor.submit(trigger("a", json!({}))).start.unwrap();
    supervisor.submit(trigger("b", json!({})));
    drop(supervisor);
    // Nothing to assert beyond construction — the point is `new`
    // still works with no store and no restore path runs.
    let s2 = RunSupervisor::new(policy(Mode::Fifo, false));
    assert!(s2.is_idle(), "fresh supervisor starts empty");
}
