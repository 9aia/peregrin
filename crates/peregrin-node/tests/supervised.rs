#![allow(clippy::unwrap_used, clippy::expect_used, clippy::pedantic)]
//! `supervise` e2e — the production driving loop behind
//! `supersede:` + ingress nodes: fake-ingress triggers pushed through
//! the returned `Injector` exercise latest-wins pending dedupe,
//! `cancel_running` mid-flight cancels, fifo queueing, the shared
//! `--history` JSONL, and the docs-deploy campaign's live `supersede:`
//! block end to end.

use std::path::{Path, PathBuf};
use std::time::Duration;

use peregrin_core::{Campaign, Event, SupersedeMode};
use peregrin_loader::Loader;
use peregrin_loader_yaml::YamlLoader;
use peregrin_node::campaign;
use peregrin_node::supervisor::{SupervisedOptions, SupervisedRun, supervise};
use peregrin_queue::{Mode, Supersede};
use serde_json::{Value, json};

fn load(yaml: &str) -> Campaign {
    YamlLoader.load_str(yaml, Path::new(".")).unwrap()
}

fn trigger(repo: &str, extra: Value) -> Event {
    let mut payload = extra;
    payload["repo"] = json!(repo);
    Event::new("push", payload, "github")
}

fn kinds(log: &[Event]) -> Vec<&str> {
    log.iter().map(|e| e.kind.as_str()).collect()
}

/// `supervise` + a joined shutdown — push `events` as triggers, drop
/// the injector (end of triggers), join for the aggregated logs.
fn drive(campaign: &Campaign, options: SupervisedOptions, events: Vec<Event>) -> SupervisedRun {
    let run = supervise(campaign, options);
    for event in events {
        run.injector.inject(event).unwrap();
    }
    run
}

/// `mode`/`cancel_running` as a `supersede:` yaml block over a
/// `sleep`-command worker — the run's only work is to be slow enough
/// to supersede mid-flight.
fn sleep_campaign(mode: &str, cancel_running: bool, secs: f64) -> String {
    format!(
        "
        supersede:
          key: \"{{repo}}\"
          mode: {mode}
          cancel_running: {cancel_running}
        nodes:
          - id: build
            driver: shell
            on: [push]
            emits: [build.done]
            params:
              cmd: \"sleep {secs}\"
              emit: {{\"0\": build.done}}
        "
    )
}

#[test]
fn cancel_running_supersedes_the_active_run() {
    let campaign = load(&sleep_campaign("latest-wins", true, 1.0));
    let run = supervise(&campaign, SupervisedOptions::default());
    run.injector
        .inject(trigger("a/b", json!({"sha": "1"})))
        .unwrap();
    // Land the second trigger mid-sleep — the supervisor pushes
    // `workflow.cancel` into the active run's own ingress channel.
    std::thread::sleep(Duration::from_millis(150));
    run.injector
        .inject(trigger("a/b", json!({"sha": "2"})))
        .unwrap();
    drop(run.injector);

    let logs = run.join.join().unwrap().unwrap();
    let kinds = kinds(&logs);
    // Run 1 ran its in-flight step out then cancelled at the next
    // iteration boundary (cooperative cancel — no forced kill).
    let cancelled = kinds
        .iter()
        .position(|k| *k == "workflow.cancelled")
        .unwrap();
    let done = kinds.iter().rposition(|k| *k == "workflow.done").unwrap();
    assert!(
        cancelled < done,
        "the superseding run executes after: {kinds:?}"
    );
    // The in-flight `sleep` fire completed before the cancel landed —
    // `build.done` precedes `workflow.cancelled` in run 1's log.
    assert_eq!(kinds[cancelled - 1], "build.done", "trace: {kinds:?}");
    // Both triggers published — sha 1 in the superseded run, sha 2 in
    // its replacement.
    let pushes: Vec<&Event> = logs.iter().filter(|e| e.kind == "push").collect();
    assert_eq!(pushes.len(), 2);
    assert_eq!(pushes[0].payload["sha"], "1");
    assert_eq!(pushes[1].payload["sha"], "2");
    // Two runs exactly — each spawned run gets its own `start`.
    assert_eq!(kinds.iter().filter(|k| **k == "start").count(), 2);
}

#[test]
fn fifo_queues_behind_the_active_run() {
    let campaign = load(&sleep_campaign("fifo", false, 0.3));
    let run = drive(
        &campaign,
        SupervisedOptions::default(),
        vec![
            trigger("a", json!({"sha": "1"})),
            trigger("a", json!({"sha": "2"})),
            trigger("a", json!({"sha": "3"})),
        ],
    );
    drop(run.injector);
    let logs = run.join.join().unwrap().unwrap();
    let kinds = kinds(&logs);
    assert_eq!(kinds.iter().filter(|k| **k == "workflow.done").count(), 3);
    assert!(!kinds.contains(&"workflow.cancelled"));
    // Sequential drain — pushes appear in submit order, each inside
    // its own run's start..done window.
    let shas: Vec<&str> = logs
        .iter()
        .filter(|e| e.kind == "push")
        .filter_map(|e| e.payload["sha"].as_str())
        .collect();
    assert_eq!(shas, ["1", "2", "3"], "trace: {kinds:?}");
}

#[test]
fn latest_wins_replaces_the_pending_run() {
    // cancel_running off: sha 2 and 3 queue behind the active run —
    // latest-wins drops sha 2 in place; only sha 3's run executes.
    let campaign = load(&sleep_campaign("latest-wins", false, 0.4));
    let run = drive(
        &campaign,
        SupervisedOptions::default(),
        vec![
            trigger("a", json!({"sha": "1"})),
            trigger("a", json!({"sha": "2"})),
            trigger("a", json!({"sha": "3"})),
        ],
    );
    drop(run.injector);
    let logs = run.join.join().unwrap().unwrap();
    let shas: Vec<&str> = logs
        .iter()
        .filter(|e| e.kind == "push")
        .filter_map(|e| e.payload["sha"].as_str())
        .collect();
    assert_eq!(shas, ["1", "3"], "sha 2 was displaced pending");
    assert_eq!(logs.iter().filter(|e| e.kind == "workflow.done").count(), 2);
}

#[test]
fn distinct_keys_dont_supersede_each_other() {
    // key "{repo}" — different repos are different logical runs;
    // fifo-mode queueing semantics per key.
    let campaign = load(&sleep_campaign("latest-wins", false, 0.3));
    let run = drive(
        &campaign,
        SupervisedOptions::default(),
        vec![
            trigger("repo-a", json!({"sha": "1"})),
            trigger("repo-b", json!({"sha": "2"})),
            trigger("repo-a", json!({"sha": "3"})),
        ],
    );
    drop(run.injector);
    let logs = run.join.join().unwrap().unwrap();
    let keys: Vec<(&str, &str)> = logs
        .iter()
        .filter(|e| e.kind == "push")
        .map(|e| {
            (
                e.payload["repo"].as_str().unwrap(),
                e.payload["sha"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        keys,
        [("repo-a", "1"), ("repo-b", "2"), ("repo-a", "3")],
        "different keys never dedupe — all three run in order"
    );
}

#[test]
fn every_spawned_run_appends_to_the_shared_history() {
    let tmp = tempfile::tempdir().unwrap();
    let history = tmp.path().join("history.jsonl");
    let campaign = load(&sleep_campaign("latest-wins", true, 0.5));
    let run = supervise(
        &campaign,
        SupervisedOptions {
            history: Some(history.clone()),
            ..SupervisedOptions::default()
        },
    );
    run.injector
        .inject(trigger("a/b", json!({"sha": "aa"})))
        .unwrap();
    std::thread::sleep(Duration::from_millis(150));
    run.injector
        .inject(trigger("a/b", json!({"sha": "bb"})))
        .unwrap();
    drop(run.injector);
    let logs = run.join.join().unwrap().unwrap();

    // The aggregated join result holds each run's own log only —
    // run 2 must not have inherited run 1's events (append, not
    // read-back).
    assert_eq!(kinds(&logs).iter().filter(|k| **k == "start").count(), 2);

    // The history file reads back both runs — the surface
    // `peregrin runs`/`log`/`explain` consumes.
    let text = std::fs::read_to_string(&history).unwrap();
    let file_kinds: Vec<String> = text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            serde_json::from_str::<Value>(l).unwrap()["type"]
                .as_str()
                .unwrap()
                .to_string()
        })
        .collect();
    assert!(
        file_kinds.contains(&"workflow.cancelled".to_string()),
        "run 1's cancel persisted: {file_kinds:?}"
    );
    assert!(
        file_kinds.contains(&"workflow.done".to_string()),
        "run 2 persisted: {file_kinds:?}"
    );
    assert_eq!(
        file_kinds.iter().filter(|k| *k == "push").count(),
        2,
        "both triggers persisted"
    );
}

#[test]
fn displaced_pending_run_marks_the_shared_history() {
    let tmp = tempfile::tempdir().unwrap();
    let history = tmp.path().join("history.jsonl");
    let campaign = load(&sleep_campaign("latest-wins", false, 0.3));
    let run = supervise(
        &campaign,
        SupervisedOptions {
            history: Some(history.clone()),
            ..SupervisedOptions::default()
        },
    );
    // sha 1 starts the (slow) run; sha 2 + sha 3 queue behind it —
    // latest-wins displaces sha 2 before sha 3's run starts.
    for sha in ["1", "2", "3"] {
        run.injector
            .inject(trigger("a/b", json!({"sha": sha})))
            .unwrap();
    }
    drop(run.injector);
    let logs = run.join.join().unwrap().unwrap();

    // The aggregated log carries the marker inside sha 3's run.
    let marker = logs
        .iter()
        .find(|e| e.kind == "push.superseded")
        .expect("displaced pending run leaves a marker");
    assert_eq!(marker.payload["sha"], "2");
    assert_eq!(marker.payload["superseded"], true);

    // …and it persisted to the shared `--history` JSONL — the
    // surface `peregrin runs`/`explain` reads (status `superseded`).
    let text = std::fs::read_to_string(&history).unwrap();
    let events: Vec<Value> = text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    let persisted = events
        .iter()
        .find(|e| e["type"] == "push.superseded")
        .expect("marker persisted to history");
    assert_eq!(persisted["payload"]["sha"], "2");
    assert_eq!(persisted["payload"]["superseded"], true);
}

#[test]
fn listener_run_appends_its_boot_failure_to_the_shared_history() {
    // The pr-heal/T-076 shape: a `github-webhook` listener node whose
    // repo secret can't resolve — the listener run dies at boot and,
    // before the fix, left no durable trace (its event log died with
    // the thread; nothing reached the shared `--history` JSONL).
    let tmp = tempfile::tempdir().unwrap();
    let history = tmp.path().join("history.jsonl");
    let campaign = load(
        r#"
        supersede:
          key: "{repo}"
          mode: latest-wins
        nodes:
          - id: gate.ingress
            driver: github-webhook
            on: [start]
            params:
              listen: "127.0.0.1:0"
              repos:
                - name: "o/r"
                  secret_env: "PEREGRIN_LISTENER_TEST_MISSING"
        "#,
    );
    // The listener's own drivers hold trigger-channel injectors, so
    // dropping `run.injector` alone never ends supervision — the
    // `campaign serve` shutdown flag is the production stop (same as
    // SIGTERM). Give the listener a beat to hit its boot failure,
    // then flag shutdown and join.
    let shutdown = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let run = supervise(
        &campaign,
        SupervisedOptions {
            history: Some(history.clone()),
            shutdown: Some(std::sync::Arc::clone(&shutdown)),
            ..SupervisedOptions::default()
        },
    );
    std::thread::sleep(Duration::from_millis(500));
    shutdown.store(true, std::sync::atomic::Ordering::Relaxed);
    run.join.join().unwrap().unwrap();

    let text = std::fs::read_to_string(&history).unwrap();
    let file_kinds: Vec<String> = text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            serde_json::from_str::<Value>(l).unwrap()["type"]
                .as_str()
                .unwrap()
                .to_string()
        })
        .collect();
    // `node.failed` is the signal that was silent for ~15h in the
    // pr-heal incident; `workflow.cancelled` is the shutdown drain.
    assert!(
        file_kinds.contains(&"node.failed".to_string()),
        "listener boot failure persisted: {file_kinds:?}"
    );
}

#[test]
fn listener_run_hard_error_surfaces_mid_supervision_with_a_history_marker() {
    // The other half of the T-076 gap: a listener run that dies on a
    // hard `Err` — here strict preflight rejects the campaign before
    // `start` ever publishes, so the history file would otherwise
    // stay EMPTY — must (a) be observed while the supervisor is
    // still running, not only at shutdown join, and (b) leave a
    // durable terminal marker in the shared `--history` JSONL.
    let tmp = tempfile::tempdir().unwrap();
    let history = tmp.path().join("history.jsonl");
    let campaign = load(
        r#"
        strict: true
        supersede:
          key: "{repo}"
          mode: latest-wins
        nodes:
          - id: gate.ingress
            driver: github-webhook
            on: [start]
            # no `emits` — strict preflight fails the run before any
            # event publishes
            params:
              listen: "127.0.0.1:0"
              repos:
                - name: "o/r"
                  secret_env: "PEREGRIN_LISTENER_TEST_MISSING"
        "#,
    );
    let shutdown = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let run = supervise(
        &campaign,
        SupervisedOptions {
            history: Some(history.clone()),
            shutdown: Some(std::sync::Arc::clone(&shutdown)),
            ..SupervisedOptions::default()
        },
    );
    // The listener thread exits on its own; the shutdown flag (the
    // `campaign serve` SIGTERM path) ends supervision — `run.injector`
    // stays alive so the trigger channel alone can't end it.
    std::thread::sleep(Duration::from_millis(500));
    shutdown.store(true, std::sync::atomic::Ordering::Relaxed);
    let result = run.join.join().unwrap();
    assert!(result.is_err(), "listener failure must fail the join");

    let text = std::fs::read_to_string(&history).unwrap();
    let markers: Vec<Value> = text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str::<Value>(l).unwrap())
        .filter(|e: &Value| e["type"] == "workflow.failed")
        .collect();
    assert_eq!(markers.len(), 1, "exactly one terminal marker: {text}");
    assert_eq!(markers[0]["payload"]["listener"], true);
    assert!(
        markers[0]["payload"]["error"]
            .as_str()
            .unwrap()
            .contains("ingress listener run failed"),
        "marker names the listener failure: {}",
        markers[0]
    );
}

/// The silent-listener shape shared by the watchdog tests: a
/// `github-webhook` ingress node whose trigger kind never arrives —
/// the run publishes only its synthetic `start`, then parks in
/// serve-quiet forever. Same observable signature as the incident's
/// stuck run (blocked in invoke/secret resolution): alive thread,
/// zero post-`start` events.
const SILENT_LISTENER_YAML: &str = r#"
    supersede:
      key: "{repo}"
      mode: latest-wins
    nodes:
      - id: gate.ingress
        driver: github-webhook
        on: [bogus.never]
        params:
          listen: "127.0.0.1:0"
          repos: []
"#;

fn history_kinds(history: &Path) -> Vec<String> {
    std::fs::read_to_string(history)
        .unwrap()
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            serde_json::from_str::<Value>(l).unwrap()["type"]
                .as_str()
                .unwrap()
                .to_string()
        })
        .collect()
}

#[test]
fn listener_watchdog_marks_a_listener_that_never_emits() {
    // The stuck-not-dead half of the T-076 gap: the exit-watch only
    // sees a dead JoinHandle; a run parked before its first real
    // event used to leave zero trace. The watchdog must append a
    // durable `listener.stalled` marker once the timeout lapses.
    let tmp = tempfile::tempdir().unwrap();
    let history = tmp.path().join("history.jsonl");
    let campaign = load(SILENT_LISTENER_YAML);
    let shutdown = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let run = supervise(
        &campaign,
        SupervisedOptions {
            history: Some(history.clone()),
            shutdown: Some(std::sync::Arc::clone(&shutdown)),
            listener_watchdog: Duration::from_millis(300),
            ..SupervisedOptions::default()
        },
    );
    // Well past the 300ms watchdog — the marker must already be in
    // the file before shutdown drains the loop.
    std::thread::sleep(Duration::from_millis(900));
    shutdown.store(true, std::sync::atomic::Ordering::Relaxed);
    run.join.join().unwrap().unwrap();

    let kinds = history_kinds(&history);
    assert!(
        kinds.contains(&"listener.stalled".to_string()),
        "silent listener left a stall marker: {kinds:?}"
    );
    let text = std::fs::read_to_string(&history).unwrap();
    let marker = text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str::<Value>(l).unwrap())
        .find(|e| e["type"] == "listener.stalled")
        .unwrap();
    assert_eq!(marker["payload"]["listener"], true);
    assert_eq!(marker["payload"]["timeout_ms"], 300);
}

#[test]
fn listener_watchdog_disarms_once_the_listener_emits() {
    // A healthy `github-webhook` node (`on: [start]`, ephemeral
    // port, no repos) binds and returns `{id}.listening` — the
    // first post-`start` event disarms the watchdog permanently;
    // listeners idle between deliveries, so going quiet later is
    // normal and must never re-arm.
    let tmp = tempfile::tempdir().unwrap();
    let history = tmp.path().join("history.jsonl");
    let campaign = load(
        r#"
        supersede:
          key: "{repo}"
          mode: latest-wins
        capabilities:
          secrets: true
        nodes:
          - id: gate.ingress
            driver: github-webhook
            on: [start]
            params:
              listen: "127.0.0.1:0"
              repos:
                - name: "o/r"
                  secret_env: "PEREGRIN_WATCHDOG_WH_TEST"
        "#,
    );
    let shutdown = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let run = supervise(
        &campaign,
        SupervisedOptions {
            history: Some(history.clone()),
            secrets: std::collections::BTreeMap::from([(
                "PEREGRIN_WATCHDOG_WH_TEST".to_string(),
                "fake-secret".to_string(),
            )]),
            shutdown: Some(std::sync::Arc::clone(&shutdown)),
            listener_watchdog: Duration::from_millis(300),
            ..SupervisedOptions::default()
        },
    );
    // Several watchdog-timeouts of quiet after the `.listening`
    // emission — disarmed means no marker, ever.
    std::thread::sleep(Duration::from_millis(1200));
    shutdown.store(true, std::sync::atomic::Ordering::Relaxed);
    run.join.join().unwrap().unwrap();

    let kinds = history_kinds(&history);
    assert!(
        kinds.iter().any(|k| k == "gate.ingress.listening"),
        "healthy listener published its readiness event: {kinds:?}"
    );
    assert!(
        !kinds.contains(&"listener.stalled".to_string()),
        "disarmed watchdog stayed silent: {kinds:?}"
    );
}

#[test]
fn listener_watchdog_zero_disables_the_check() {
    // `Duration::ZERO` opts out — the silent listener produces no
    // marker (and, with no armed watchdog, no extra ticking either).
    let tmp = tempfile::tempdir().unwrap();
    let history = tmp.path().join("history.jsonl");
    let campaign = load(SILENT_LISTENER_YAML);
    let shutdown = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let run = supervise(
        &campaign,
        SupervisedOptions {
            history: Some(history.clone()),
            shutdown: Some(std::sync::Arc::clone(&shutdown)),
            listener_watchdog: Duration::ZERO,
            ..SupervisedOptions::default()
        },
    );
    std::thread::sleep(Duration::from_millis(600));
    shutdown.store(true, std::sync::atomic::Ordering::Relaxed);
    run.join.join().unwrap().unwrap();

    let kinds = history_kinds(&history);
    assert!(
        !kinds.contains(&"listener.stalled".to_string()),
        "disabled watchdog stayed silent: {kinds:?}"
    );
}

// ---------------------------------------------------------------------------
// docs-deploy — the shipped `supersede:` block is live config now
// ---------------------------------------------------------------------------

fn libraries_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../libraries")
        .canonicalize()
        .unwrap()
}

#[test]
fn docs_deploy_supersede_block_parses() {
    let campaign = campaign::load(&libraries_dir().join("docs-deploy/main.yaml")).unwrap();
    let supersede = campaign.supersede.expect("docs-deploy declares supersede:");
    assert_eq!(supersede.key, "{repo}:{ref}");
    assert_eq!(supersede.mode, SupersedeMode::LatestWins);
    assert!(supersede.cancel_running);
    // …and converts into the queue policy the supervisor drives.
    let policy = Supersede::from(supersede);
    assert_eq!(policy.mode, Mode::LatestWins);
    assert!(policy.cancel_running);
}

#[test]
fn docs_deploy_operator_overlay_splits_into_listener_and_workers() {
    // The operator node config — `include:` the bundle policy plus a
    // node-level `github-webhook` ingress — is what `campaign run`
    // sees under the systemd unit. The merge carries the bundle's
    // `supersede:` through.
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("gate.yaml");
    std::fs::write(
        &path,
        format!(
            r#"
            name: my-gate
            include: ["{}/docs-deploy/main.yaml"]
            nodes:
              - id: gate.ingress
                driver: github-webhook
                on: [start]
                params:
                  listen: "127.0.0.1:0"
                  repos: []
            "#,
            libraries_dir().display()
        ),
    )
    .unwrap();
    let campaign = campaign::load(&path).unwrap();
    assert!(
        campaign.supersede.is_some(),
        "include merge carries the bundle's supersede:"
    );
    assert!(
        campaign::has_ingress_nodes(&campaign),
        "webhook node is ingress"
    );

    // The split `campaign run` supervises: `gate.ingress` persists in
    // the listener half; the bundle's worker graph is the per-trigger
    // half — a `driver: github-webhook` node never lands inside a
    // spawned run (it would re-bind the port).
    let (injector, _ingress) = peregrin_engine::ingress();
    let registry = campaign::registry_with_ingress(&injector);
    let (listener, worker) = campaign::split_ingress(&campaign, &registry);
    // The bundle ships its own webhook ingress node — the
    // listener half holds both webhook nodes; the worker half none.
    assert_eq!(listener.nodes.len(), 2);
    assert!(listener.nodes.iter().any(|n| n.id == "gate.ingress"));
    assert_eq!(worker.nodes.len(), campaign.nodes.len() - 2);
    assert!(worker.nodes.iter().all(|n| n.driver != "github-webhook"));
    assert!(listener.nodes.iter().all(|n| n.driver == "github-webhook"));
}

#[test]
fn supervised_runs_write_through_the_queue_store() {
    use std::sync::Arc;

    use peregrin_queue::{MemoryQueueStore, QueueStore};

    let store = Arc::new(MemoryQueueStore::new());
    let campaign = load(&sleep_campaign("fifo", false, 0.2));
    let run = supervise(
        &campaign,
        SupervisedOptions {
            queue_store: Some(store.clone()),
            ..SupervisedOptions::default()
        },
    );
    run.injector
        .inject(trigger("a", json!({"sha": "1"})))
        .unwrap();
    run.injector
        .inject(trigger("a", json!({"sha": "2"})))
        .unwrap();
    drop(run.injector);
    let logs = run.join.join().unwrap().unwrap();
    assert_eq!(logs.iter().filter(|e| e.kind == "workflow.done").count(), 2);

    // Every run finished and the queue drained — the durable store
    // holds nothing: no pending submissions, no in-flight marker.
    let snapshot = store.load().unwrap();
    assert!(snapshot.pending.is_empty(), "queue drained: {snapshot:?}");
    assert!(snapshot.running.is_none(), "no run in flight");
}
