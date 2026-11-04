#![allow(clippy::unwrap_used, clippy::expect_used, clippy::pedantic)]
//! `sturn-poll` driver tests — param parsing, ref-head polling over a
//! scripted `Git` fake and real `git` (bare-remote `ls-remote` and
//! local-clone `for-each-ref`), the baseline/stalled/recovered loop
//! semantics, channel-close exit, and an end-to-end
//! `Runner::with_ingress` run where a `sturn snap`-style WAL bump
//! fires a downstream node.

use std::collections::VecDeque;
use std::ffi::OsStr;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use serde_json::{Map, Value, json};

use peregrin_core::{Event, Node};
use peregrin_driver::{Driver, DriverError, ExecutionContext, NoopDriver};
use peregrin_driver_sturn::{SturnDriver, Watch};
use peregrin_engine::{Ingress, Registry, RunOptions, Runner, ingress};
use peregrin_git::{Git, SystemGit};
use peregrin_loader::Loader;
use peregrin_loader_yaml::YamlLoader;

// ---------------------------------------------------------------------------
// Scripted Git — pops queued outcomes, then blocks on `rx` for more.
// ---------------------------------------------------------------------------

/// One canned `git.run` result.
enum Outcome {
    /// Stdout text.
    Out(&'static str),
    /// A failing git invocation.
    Fail,
}

/// A `Git` fake: `script` outcomes are served first, in order; once it
/// drains, every call blocks on `rx.recv()` so the test paces the
/// poller deterministically (a closed `rx` answers `Fail` forever).
struct SeqGit {
    calls: Mutex<Vec<Vec<String>>>,
    state: Mutex<SeqState>,
}

struct SeqState {
    script: VecDeque<Outcome>,
    rx: Receiver<Outcome>,
}

impl SeqGit {
    fn new(script: Vec<Outcome>, rx: Receiver<Outcome>) -> Self {
        Self {
            calls: Mutex::new(Vec::new()),
            state: Mutex::new(SeqState {
                script: VecDeque::from(script),
                rx,
            }),
        }
    }

    fn argv(&self) -> Vec<Vec<String>> {
        self.calls.lock().unwrap().clone()
    }
}

impl Git for SeqGit {
    fn run(&self, _dir: &Path, args: &[&OsStr]) -> Result<String, peregrin_git::Error> {
        self.calls.lock().unwrap().push(
            args.iter()
                .map(|a| a.to_string_lossy().into_owned())
                .collect(),
        );
        let outcome = {
            let mut state = self.state.lock().unwrap();
            state
                .script
                .pop_front()
                .unwrap_or_else(|| state.rx.recv().unwrap_or(Outcome::Fail))
        };
        match outcome {
            Outcome::Out(stdout) => Ok(stdout.to_string()),
            Outcome::Fail => Err(peregrin_git::Error::Git {
                dir: PathBuf::from("/repo"),
                args: vec!["ls-remote".to_string()],
                status: "exit status: 128".to_string(),
                stderr: "stub fail".to_string(),
            }),
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn node(id: &str, params: Value) -> Node {
    Node {
        id: id.into(),
        driver: "sturn-poll".into(),
        params,
        ..Default::default()
    }
}

fn watch(remote: &str) -> Watch {
    Watch {
        repo: PathBuf::from("/repo"),
        remote: remote.to_string(),
        patterns: vec!["refs/*".to_string()],
        poll: Duration::from_millis(1),
        clone_url: None,
    }
}

/// Poll `cond` until it holds or the deadline trips.
fn wait_until(mut cond: impl FnMut() -> bool, what: &str) {
    let deadline = Instant::now() + Duration::from_secs(15);
    while !cond() {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        thread::sleep(Duration::from_millis(5));
    }
}

/// Drain the test's `Ingress` into `events` until it grows to `n`.
fn drain_to(ing: &Ingress, events: &mut Vec<Event>, n: usize) {
    wait_until(
        || {
            ing.drain_into(events);
            events.len() >= n
        },
        "events",
    );
}

fn kinds(events: &[Event]) -> Vec<&str> {
    events.iter().map(|e| e.kind.as_str()).collect()
}

/// `git <args>` under `dir` with an inline identity — asserts success.
fn sh(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(["-c", "user.email=t@t", "-c", "user.name=t"])
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?} in {}: {}",
        dir.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// `git init --bare remote` + a `work` clone source with one commit
/// pushed to `refs/heads/main` and `refs/sturn/wal/dev`.
/// Returns `(bare, work, head_sha)`.
fn scaffold(tmp: &Path) -> (PathBuf, PathBuf, String) {
    let bare = tmp.join("remote.git");
    let work = tmp.join("work");
    std::fs::create_dir_all(&bare).unwrap();
    std::fs::create_dir_all(&work).unwrap();
    sh(&bare, &["init", "--bare"]);
    sh(&work, &["init"]);
    std::fs::write(work.join("f"), "x").unwrap();
    sh(&work, &["add", "f"]);
    sh(&work, &["commit", "-m", "init"]);
    let bare_str = bare.to_str().unwrap().to_string();
    sh(&work, &["remote", "add", "origin", &bare_str]);
    sh(
        &work,
        &[
            "push",
            "origin",
            "HEAD:refs/heads/main",
            "HEAD:refs/sturn/wal/dev",
        ],
    );
    let sha = sh(&work, &["rev-parse", "HEAD"]);
    (bare, work, sha)
}

/// `sturn snap` on the remote: an empty commit pushed to `ref`.
fn bump(work: &Path, refname: &str) -> String {
    sh(work, &["commit", "--allow-empty", "-m", "snap"]);
    let sha = sh(work, &["rev-parse", "HEAD"]);
    sh(work, &["push", "origin", &format!("HEAD:{refname}")]);
    sha
}

/// A spawn seam that keeps the real `JoinHandle` in `slot` (the
/// driver drops its copy — tests need it to prove thread exit).
fn capturing_spawn(slot: Arc<Mutex<Option<JoinHandle<()>>>>) -> Arc<peregrin_driver_sturn::Spawn> {
    Arc::new(move |task| {
        *slot.lock().unwrap() = Some(thread::spawn(task));
        Ok(thread::spawn(|| {}))
    })
}

// ---------------------------------------------------------------------------
// Params → Watch
// ---------------------------------------------------------------------------

#[test]
fn from_params_defaults_and_overrides() {
    let empty = Map::new();
    let w = Watch::from_params(&empty);
    assert_eq!(w.remote, "origin");
    assert_eq!(w.repo, PathBuf::from("."));
    assert_eq!(w.poll, Duration::from_millis(1000));
    assert_eq!(w.patterns, vec!["refs/heads/*", "refs/sturn/wal/*"]);

    let custom = json!({
        "remote": "upstream",
        "repo": "/tmp/somewhere",
        "poll_ms": 50,
        "refs": "refs/a/*:refs/b/*",
    })
    .as_object()
    .unwrap()
    .clone();
    let w = Watch::from_params(&custom);
    assert_eq!(w.remote, "upstream");
    assert_eq!(w.repo, PathBuf::from("/tmp/somewhere"));
    assert_eq!(w.poll, Duration::from_millis(50));
    assert_eq!(w.patterns, vec!["refs/a/*", "refs/b/*"]);

    // `remote: ""` is local mode — verbatim, not defaulted.
    let local = json!({"remote": "", "refs": ["x/*", "", "y/*", 4]})
        .as_object()
        .unwrap()
        .clone();
    let w = Watch::from_params(&local);
    assert_eq!(w.remote, "");
    assert_eq!(w.patterns, vec!["x/*", "y/*"]);

    // Empty/garbage values all fall back to the defaults.
    for refs in [json!(":::"), json!([]), json!(7)] {
        let params = json!({"refs": refs}).as_object().unwrap().clone();
        assert_eq!(
            Watch::from_params(&params).patterns,
            vec!["refs/heads/*", "refs/sturn/wal/*"]
        );
    }
    for poll_ms in [json!(0), json!("fast"), json!(-3)] {
        let params = json!({"poll_ms": poll_ms}).as_object().unwrap().clone();
        assert_eq!(
            Watch::from_params(&params).poll,
            Duration::from_millis(1000)
        );
    }
    let params = json!({"repo": ""}).as_object().unwrap().clone();
    assert_eq!(Watch::from_params(&params).repo, PathBuf::from("."));
    // `~` expands per the peregrin-git convention.
    let params = json!({"repo": "~/cache"}).as_object().unwrap().clone();
    let home = std::env::var_os("HOME").unwrap();
    assert_eq!(
        Watch::from_params(&params).repo,
        PathBuf::from(home).join("cache")
    );
}

// ---------------------------------------------------------------------------
// heads() — argv + parsing per mode
// ---------------------------------------------------------------------------

#[test]
fn heads_remote_parses_ls_remote() {
    let (_tx, rx) = mpsc::channel();
    let git = SeqGit::new(
        vec![Outcome::Out(
            "aaa\trefs/heads/main\nbbb\trefs/tags/v1\nbbb\trefs/tags/v1^{}\nccc\tHEAD\nmalformed\n",
        )],
        rx,
    );
    let heads = watch("origin").heads(&git).unwrap();
    assert_eq!(
        git.argv()[0],
        vec!["ls-remote", "origin", "refs/*"],
        "ls-remote <remote> <patterns>"
    );
    // Peeled `^{}` and `HEAD` filtered; malformed lines skipped.
    assert_eq!(heads.len(), 2);
    assert_eq!(heads["refs/heads/main"], "aaa");
    assert_eq!(heads["refs/tags/v1"], "bbb");
}

#[test]
fn heads_local_parses_for_each_ref() {
    let (_tx, rx) = mpsc::channel();
    let git = SeqGit::new(
        vec![
            Outcome::Out("refs/sturn/wal/dev\taaa\nrefs/heads/main\tccc\n"),
            Outcome::Out(""),
        ],
        rx,
    );
    let heads = watch("").heads(&git).unwrap();
    // `*` deepens to `**` — for-each-ref's `*` doesn't cross `/`
    // where ls-remote's does; the modes must watch the same refs.
    assert_eq!(
        git.argv()[0],
        vec![
            "for-each-ref",
            "--format=%(refname)%09%(objectname)",
            "refs/**"
        ]
    );
    assert_eq!(heads["refs/sturn/wal/dev"], "aaa");
    assert_eq!(heads["refs/heads/main"], "ccc");

    // `**` inputs and `*`-runs inside patterns deepen the same way —
    // every `*` run collapses to one `**`.
    let mut w = watch("");
    w.patterns = vec!["refs/**".into(), "a*b*c".into(), "plain".into()];
    w.heads(&git).unwrap();
    assert_eq!(
        git.argv()[1],
        vec![
            "for-each-ref",
            "--format=%(refname)%09%(objectname)",
            "refs/**",
            "a**b**c",
            "plain"
        ]
    );
}

// ---------------------------------------------------------------------------
// The poll loop — baseline, diff events, stalled/recovered, exit
// ---------------------------------------------------------------------------

#[test]
fn run_seeds_baseline_then_emits_typed_changes() {
    let (tx, rx) = mpsc::channel();
    // First poll = baseline, silent.
    let git = SeqGit::new(vec![Outcome::Out("aaa\trefs/sturn/wal/dev\n")], rx);
    let (injector, ing) = ingress();
    let w = watch("origin");
    let stop = AtomicBool::new(false);
    let handle = thread::spawn(move || w.run(&git, &injector, "watch", &stop));
    let mut events = Vec::new();

    tx.send(Outcome::Out("bbb\trefs/sturn/wal/dev\n")).unwrap();
    drain_to(&ing, &mut events, 1);
    assert_eq!(events[0].kind, "wal.update");
    assert_eq!(events[0].source, "watch");
    assert_eq!(events[0].payload["ref"], "refs/sturn/wal/dev");
    assert_eq!(events[0].payload["sha"], "bbb");
    assert_eq!(events[0].payload["old_sha"], "aaa");
    assert_eq!(events[0].payload["ws"], "dev");
    assert_eq!(events[0].payload["remote"], "origin");
    assert_eq!(events[0].payload["repo"], "/repo");

    // An unchanged ref emits nothing.
    tx.send(Outcome::Out("bbb\trefs/sturn/wal/dev\n")).unwrap();
    thread::sleep(Duration::from_millis(50));
    ing.drain_into(&mut events);
    assert_eq!(events.len(), 1, "same sha → no event");

    // New + changed refs: `push` gets `branch`, sturn `ws/` refs get
    // `ws`, unseen refs carry `old_sha: null`, other refs → `ref.update`.
    tx.send(Outcome::Out(
        "bbb\trefs/sturn/wal/dev\nddd\trefs/sturn/ws/x\nccc\trefs/heads/main\neee\trefs/other/z\n",
    ))
    .unwrap();
    drain_to(&ing, &mut events, 4);
    let by_ref = |r: &str| events.iter().find(|e| e.payload["ref"] == r).unwrap();
    assert_eq!(by_ref("refs/heads/main").kind, "push");
    assert_eq!(by_ref("refs/heads/main").payload["branch"], "main");
    assert!(by_ref("refs/heads/main").payload["old_sha"].is_null());
    // The github-webhook push fields: `before` mirrors `old_sha`,
    // `actor` is unattested-empty.
    assert!(by_ref("refs/heads/main").payload["before"].is_null());
    assert_eq!(by_ref("refs/heads/main").payload["actor"], "");
    assert_eq!(by_ref("refs/sturn/ws/x").kind, "wal.update");
    assert_eq!(by_ref("refs/sturn/ws/x").payload["ws"], "x");
    assert_eq!(by_ref("refs/other/z").kind, "ref.update");
    assert!(by_ref("refs/other/z").payload.get("ws").is_none());

    // Channel close → the next emit fails and the loop exits.
    drop(ing);
    tx.send(Outcome::Out("fff\trefs/sturn/wal/dev\n")).unwrap();
    handle.join().unwrap();
}

#[test]
fn run_stalled_is_power_of_two_then_recovers() {
    let (tx, rx) = mpsc::channel();
    let git = SeqGit::new(
        vec![
            Outcome::Fail,
            Outcome::Fail,
            Outcome::Fail,
            Outcome::Fail,
            Outcome::Fail,
            Outcome::Out("aaa\trefs/sturn/wal/dev\n"),
        ],
        rx,
    );
    let (injector, ing) = ingress();
    let w = watch("origin");
    let stop = AtomicBool::new(false);
    let handle = thread::spawn(move || w.run(&git, &injector, "watch", &stop));
    let mut events = Vec::new();

    // Failures 1..5 → stalled at 1, 2, 4 (power of two), then a
    // success emits recovered (failures=5) and seeds the baseline.
    drain_to(&ing, &mut events, 4);
    assert_eq!(
        kinds(&events),
        vec![
            "driver.stalled",
            "driver.stalled",
            "driver.stalled",
            "driver.recovered"
        ]
    );
    assert_eq!(events[0].payload["failures"], 1);
    assert_eq!(events[1].payload["failures"], 2);
    assert_eq!(events[2].payload["failures"], 4);
    assert_eq!(events[2].payload["remote"], "origin");
    assert!(
        events[2].payload["error"]
            .as_str()
            .unwrap()
            .contains("stub fail")
    );
    assert_eq!(events[3].payload["failures"], 5);
    assert_eq!(events.len(), 4, "recovery seed is silent");

    // The loop is alive — a later change still emits.
    tx.send(Outcome::Out("zzz\trefs/sturn/wal/dev\n")).unwrap();
    drain_to(&ing, &mut events, 5);
    assert_eq!(events[4].kind, "wal.update");

    // A closed channel + a stalled inject is also an exit path.
    drop(ing);
    tx.send(Outcome::Fail).unwrap(); // stalled inject fails → return
    handle.join().unwrap();

    // And again on the `driver.recovered` emit — a success after a
    // failure streak into a closed channel.
    let (tx2, rx2) = mpsc::channel();
    let git2 = SeqGit::new(vec![Outcome::Fail], rx2);
    let (injector2, ing2) = ingress();
    let w2 = watch("origin");
    let stop2 = AtomicBool::new(false);
    let handle2 = thread::spawn(move || w2.run(&git2, &injector2, "watch", &stop2));
    let mut events2 = Vec::new();
    drain_to(&ing2, &mut events2, 1); // stalled(1)
    drop(ing2);
    tx2.send(Outcome::Out("aaa\trefs/sturn/wal/dev\n")).unwrap(); // recovered inject fails → return
    handle2.join().unwrap();
}

// ---------------------------------------------------------------------------
// invoke() — watching readiness, dry run, replace semantics, spawn
// failure, driver-drop join
// ---------------------------------------------------------------------------

#[test]
fn invoke_dry_run_returns_watching_and_spawns_nothing() {
    let (injector, _ing) = ingress();
    let spawn_calls = Arc::new(AtomicUsize::new(0));
    let calls = spawn_calls.clone();
    let d = SturnDriver::with_parts(
        Arc::new(SystemGit::new()),
        Arc::new(peregrin_template::SystemEffects),
        injector,
        Arc::new(move |task| {
            calls.fetch_add(1, Ordering::SeqCst);
            Ok(thread::spawn(task))
        }),
    );
    let ctx = ExecutionContext {
        dry_run: true,
        ..ExecutionContext::default()
    };
    let out = d
        .invoke(
            &node(
                "w",
                json!({"remote": "up", "repo": "/nonexistent-sturn-test", "poll_ms": 25}),
            ),
            &[],
            &ctx,
        )
        .unwrap();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].kind, "w.watching");
    assert_eq!(out[0].source, "w");
    assert_eq!(out[0].payload["dry_run"], true);
    assert_eq!(out[0].payload["remote"], "up");
    assert_eq!(out[0].payload["repo"], "/nonexistent-sturn-test");
    assert_eq!(out[0].payload["poll_ms"], 25);
    assert_eq!(
        out[0].payload["refs"],
        json!(["refs/heads/*", "refs/sturn/wal/*"])
    );
    // "up" is a remote name with no clone source in a dead dir —
    // clone_url just stays absent.
    assert!(out[0].payload.get("clone_url").is_none());
    assert_eq!(spawn_calls.load(Ordering::SeqCst), 0);
}

#[test]
fn invoke_refire_replaces_poller_and_drop_joins_all() {
    let (injector, _ing) = ingress();
    let handles: Arc<Mutex<Vec<JoinHandle<()>>>> = Arc::new(Mutex::new(Vec::new()));
    let spawn_calls = Arc::new(AtomicUsize::new(0));
    let calls = spawn_calls.clone();
    let hs = handles.clone();
    let d = SturnDriver::with_parts(
        Arc::new(SystemGit::new()),
        Arc::new(peregrin_template::SystemEffects),
        injector,
        Arc::new(move |task| {
            calls.fetch_add(1, Ordering::SeqCst);
            hs.lock().unwrap().push(thread::spawn(task));
            Ok(thread::spawn(|| {}))
        }),
    );
    let ctx = ExecutionContext::default();
    // A dead repo dir keeps the spawned pollers inert (instant git
    // failure → stalled event on the live channel → sliced sleep).
    let params = json!({"repo": "/nonexistent-sturn-test", "poll_ms": 60000});
    let out = d.invoke(&node("w", params.clone()), &[], &ctx).unwrap();
    assert_eq!(out[0].kind, "w.watching");

    // A refire *replaces* the watch — new spawn, old poller's stop
    // flips and its thread exits within a sleep slice.
    d.invoke(&node("w", params.clone()), &[], &ctx).unwrap();
    // …while a different node gets its own poller.
    d.invoke(&node("w2", params), &[], &ctx).unwrap();
    assert_eq!(spawn_calls.load(Ordering::SeqCst), 3);

    let all: Vec<JoinHandle<()>> = handles.lock().unwrap().drain(..).collect();
    wait_until(|| all[0].is_finished(), "replaced poller exits");
    assert!(!all[1].is_finished());
    assert!(!all[2].is_finished());
    // Dropping the driver stops and joins every remaining poller.
    drop(d);
    wait_until(
        || all.iter().all(JoinHandle::is_finished),
        "drop joins pollers",
    );
    for handle in all {
        handle.join().unwrap();
    }
}

#[test]
fn invoke_spawn_failure_returns_node_failed() {
    let (injector, _ing) = ingress();
    let fail = Arc::new(AtomicBool::new(true));
    let failing = fail.clone();
    let d = SturnDriver::with_parts(
        Arc::new(SystemGit::new()),
        Arc::new(peregrin_template::SystemEffects),
        injector,
        Arc::new(move |task| {
            if failing.load(Ordering::SeqCst) {
                Err(io::Error::other("no threads"))
            } else {
                Ok(thread::spawn(task))
            }
        }),
    );
    let ctx = ExecutionContext::default();
    let params = json!({"repo": "/nonexistent-sturn-test", "poll_ms": 60000});
    // A spawn failure is an event-path failure (the github-webhook
    // bind-failure convention), not a DriverError.
    let out = d.invoke(&node("w", params.clone()), &[], &ctx).unwrap();
    assert_eq!(out[0].kind, "node.failed");
    assert_eq!(out[0].source, "w");
    assert!(
        out[0].payload["error"]
            .as_str()
            .unwrap()
            .contains("failed to spawn")
    );
    // Once spawning works a retry watches normally.
    fail.store(false, Ordering::SeqCst);
    let out = d.invoke(&node("w", params), &[], &ctx).unwrap();
    assert_eq!(out[0].kind, "w.watching");
}

#[test]
fn invoke_strict_unresolvable_params_errors() {
    let (injector, _ing) = ingress();
    let d = SturnDriver::new(injector);
    let ctx = ExecutionContext {
        strict: true,
        ..ExecutionContext::default()
    };
    // `${missing.*}` stays unresolved under strict — the driver-side
    // re-resolve surfaces as a DriverError before any spawn.
    let err = d
        .invoke(&node("w", json!({"remote": "${missing.value}"})), &[], &ctx)
        .unwrap_err();
    assert!(!err.reason.is_empty());
}

#[test]
fn input_and_output_contracts() {
    let (injector, _ing) = ingress();
    let d = SturnDriver::new(injector);
    assert_eq!(d.name(), "sturn-poll");
    let input = d.input().unwrap();
    let props = input.schema["properties"].as_object().unwrap();
    for key in ["remote", "repo", "poll_ms", "refs"] {
        assert!(props.contains_key(key), "input.{key}");
    }
    let kinds: Vec<&str> = d.output().iter().map(|s| s.kind.as_str()).collect();
    for kind in [
        "wal.update",
        "push",
        "ref.update",
        "driver.stalled",
        "driver.recovered",
        "*.watching",
        "node.failed",
    ] {
        assert!(kinds.contains(&kind), "output {kind}");
    }
}

// ---------------------------------------------------------------------------
// Real git — remote (ls-remote) and local (for-each-ref) end to end
// ---------------------------------------------------------------------------

#[test]
fn remote_watch_emits_wal_and_push_and_exits_on_close() {
    let tmp = tempfile::TempDir::new().unwrap();
    let (bare, work, sha1) = scaffold(tmp.path());
    let (injector, ing) = ingress();
    let slot: Arc<Mutex<Option<JoinHandle<()>>>> = Arc::new(Mutex::new(None));
    let d = SturnDriver::with_parts(
        Arc::new(SystemGit::new()),
        Arc::new(peregrin_template::SystemEffects),
        injector,
        capturing_spawn(slot.clone()),
    );
    let ctx = ExecutionContext::default();
    // `remote` as the work clone's `origin` name — ls-remote resolves
    // it inside `repo`, and `remote get-url` gives the clone source.
    let bare_str = bare.to_str().unwrap().to_string();
    let params = json!({
        "remote": "origin",
        "repo": work.to_str().unwrap(),
        "poll_ms": 25,
        "refs": "refs/sturn/*:refs/heads/*",
    });
    let out = d.invoke(&node("w", params), &[], &ctx).unwrap();
    // Readiness is `{id}.watching` — the github-webhook
    // `{id}.listening` convention.
    assert_eq!(out[0].kind, "w.watching");
    // `remote get-url origin` in the work clone resolves the bare
    // remote's path.
    assert_eq!(out[0].payload["clone_url"], bare_str);
    assert_eq!(out[0].payload["repo"], bare_str);
    // Let the first poll seed the baseline before mutating — a
    // baseline that catches the change sees no diff.
    thread::sleep(Duration::from_millis(250));
    let mut events = Vec::new();

    // WAL bump → wal.update with old → new.
    let sha2 = bump(&work, "refs/sturn/wal/dev");
    drain_to(&ing, &mut events, 1);
    assert_eq!(events[0].kind, "wal.update");
    assert_eq!(events[0].payload["ws"], "dev");
    assert_eq!(events[0].payload["sha"], sha2);
    assert_eq!(events[0].payload["old_sha"], sha1);
    assert_eq!(events[0].payload["clone_url"], bare_str);

    // Branch bump → push event a ci-gate campaign can `on:`.
    let sha3 = bump(&work, "refs/heads/main");
    // A brand-new WAL ref lands in the same diff.
    sh(&work, &["push", "origin", "HEAD:refs/sturn/wal/other"]);
    drain_to(&ing, &mut events, 3);
    let by_ref = |r: &str| events.iter().find(|e| e.payload["ref"] == r).unwrap();
    assert_eq!(by_ref("refs/heads/main").kind, "push");
    assert_eq!(by_ref("refs/heads/main").payload["branch"], "main");
    assert_eq!(by_ref("refs/heads/main").payload["sha"], sha3);
    assert_eq!(by_ref("refs/sturn/wal/other").kind, "wal.update");
    assert!(by_ref("refs/sturn/wal/other").payload["old_sha"].is_null());

    // Channel close → the poller's next emit fails → thread exits.
    drop(ing);
    bump(&work, "refs/sturn/wal/dev");
    let handle = slot.lock().unwrap().take().unwrap();
    wait_until(|| handle.is_finished(), "poller exit");
    handle.join().unwrap();
}

#[test]
fn local_watch_reads_the_clone_itself() {
    let tmp = tempfile::TempDir::new().unwrap();
    let (_bare, work, _sha) = scaffold(tmp.path());
    let (injector, ing) = ingress();
    let d = SturnDriver::new(injector);
    let ctx = ExecutionContext::default();
    let params = json!({
        "remote": "",
        "repo": work.to_str().unwrap(),
        "poll_ms": 25,
        "refs": ["refs/sturn/*", "refs/heads/*"],
    });
    d.invoke(&node("w", params), &[], &ctx).unwrap();
    thread::sleep(Duration::from_millis(250)); // baseline settle
    let mut events = Vec::new();

    // A commit moves the checked-out branch; update-ref moves the WAL
    // head — for-each-ref sees both in the clone.
    sh(&work, &["commit", "--allow-empty", "-m", "x"]);
    let sha = sh(&work, &["rev-parse", "HEAD"]);
    sh(&work, &["update-ref", "refs/sturn/wal/dev", &sha]);
    drain_to(&ing, &mut events, 2);
    let by_kind = |k: &str| events.iter().find(|e| e.kind == k).unwrap();
    assert_eq!(by_kind("wal.update").payload["ref"], "refs/sturn/wal/dev");
    assert_eq!(by_kind("wal.update").payload["remote"], "");
    // Local mode's clone source is the clone itself.
    assert_eq!(
        by_kind("wal.update").payload["clone_url"],
        work.to_str().unwrap()
    );
    let push = by_kind("push");
    assert!(
        push.payload["ref"]
            .as_str()
            .unwrap()
            .starts_with("refs/heads/")
    );
    assert_eq!(push.payload["sha"], sha);
}

#[test]
fn remote_down_stalls_then_recovers() {
    let tmp = tempfile::TempDir::new().unwrap();
    let gone = tmp.path().join("gone.git");
    let (_bare, work, _sha) = scaffold(tmp.path());
    let (injector, ing) = ingress();
    let d = SturnDriver::new(injector);
    let ctx = ExecutionContext::default();
    // A remote that doesn't exist — every ls-remote fails.
    let params = json!({
        "remote": gone.to_str().unwrap(),
        "repo": tmp.path().to_str().unwrap(),
        "poll_ms": 10,
        "refs": "refs/sturn/*",
    });
    d.invoke(&node("w", params), &[], &ctx).unwrap();
    let mut events = Vec::new();

    // Failures 1, 2, 4 emit stalled — the run is never killed.
    drain_to(&ing, &mut events, 3);
    assert_eq!(kinds(&events), vec!["driver.stalled"; 3]);
    assert_eq!(events[2].payload["failures"], 4);

    // Bring the remote up — the poller recovers on its own.
    std::fs::create_dir_all(&gone).unwrap();
    sh(&gone, &["init", "--bare"]);
    sh(
        &work,
        &["push", gone.to_str().unwrap(), "HEAD:refs/sturn/wal/dev"],
    );
    drain_to(&ing, &mut events, 4);
    assert_eq!(events[3].kind, "driver.recovered");
    assert!(events[3].payload["failures"].as_u64().unwrap() >= 4);
    // Recovery seeds the baseline silently; the next bump emits.
    let sha = bump(&work, "refs/sturn/wal/dev");
    let _ = sh(
        &work,
        &[
            "push",
            gone.to_str().unwrap(),
            &format!("{sha}:refs/sturn/wal/dev"),
        ],
    );
    drain_to(&ing, &mut events, 5);
    assert_eq!(events[4].kind, "wal.update");
}

// ---------------------------------------------------------------------------
// e2e — a real run drains the poller's injected events
// ---------------------------------------------------------------------------

/// Sleeps so the run stays alive while the poller works.
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
        thread::sleep(Duration::from_millis(700));
        Ok(vec![Event::new("slow.done", json!({}), &node.id)])
    }
}

#[test]
fn runner_drains_poller_events_into_the_graph() {
    let tmp = tempfile::TempDir::new().unwrap();
    let (bare, work, _sha) = scaffold(tmp.path());
    let (injector, chan) = ingress();
    let registry = Registry::default()
        .with("noop", Arc::new(NoopDriver))
        .with("slow", Arc::new(SlowDriver))
        .with("sturn-poll", Arc::new(SturnDriver::new(injector)));
    let campaign = YamlLoader
        .load_str(
            &format!(
                "
                nodes:
                  - id: watch
                    driver: sturn-poll
                    on: [start]
                    params:
                      remote: {}
                      repo: {}
                      poll_ms: 25
                      refs: refs/sturn/*
                  - id: slow
                    driver: slow
                    on: [start]
                    emits: [slow.done]
                  - id: gate
                    driver: noop
                    on: [wal.update]
                    emits: [gate.done]
                ",
                bare.display(),
                tmp.path().display()
            ),
            Path::new("."),
        )
        .unwrap();
    let mut runner = Runner::new(registry).with_ingress(chan);
    let log = thread::scope(|scope| {
        scope.spawn(|| {
            // Well past the poller's baseline seed, well inside the
            // slow node's sleep — the injected wal.update must land
            // while the run is still draining.
            thread::sleep(Duration::from_millis(300));
            bump(&work, "refs/sturn/wal/dev");
        });
        runner.run(&campaign, &RunOptions::default()).unwrap()
    });
    let kinds = kinds(&log);
    // The readiness event the invoke returned rode the normal graph.
    assert!(kinds.contains(&"watch.watching"), "{kinds:?}");
    assert!(kinds.contains(&"wal.update"), "{kinds:?}");
    let wal = log.iter().find(|e| e.kind == "wal.update").unwrap();
    assert_eq!(wal.source, "watch");
    assert_eq!(wal.payload["ws"], "dev");
    // The gate fired on the injected event.
    let gate_done = log
        .iter()
        .find(|e| e.kind == "node.done" && e.source == "gate");
    assert!(gate_done.is_some(), "gate fired on wal.update");
    assert_eq!(log.last().unwrap().kind, "workflow.done");
}
