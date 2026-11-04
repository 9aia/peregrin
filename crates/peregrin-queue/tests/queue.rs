#![allow(clippy::unwrap_used, clippy::pedantic)]
//! Queue/supersede policy tests — the full dedupe/supersede/cancel/
//! drain matrix plus key-template resolution and config serde.

use peregrin_core::Event;
use peregrin_queue::{Disposition, Mode, Queue, Supersede, resolve_key};
use serde_json::{Value, json};

fn ev(payload: Value) -> Event {
    Event::new("trigger", payload, "ingress")
}

fn policy(key: &str, mode: Mode, cancel_running: bool) -> Supersede {
    Supersede {
        key: key.to_string(),
        mode,
        cancel_running,
    }
}

// ---------------------------------------------------------------------------
// Key template resolution
// ---------------------------------------------------------------------------

#[test]
fn key_resolves_single_and_multi_placeholders() {
    let event = ev(json!({"repo": "9aia/peregrin", "ref": "main"}));
    assert_eq!(resolve_key("{repo}:{ref}", &event), "9aia/peregrin:main");
    assert_eq!(resolve_key("gate-{repo}", &event), "gate-9aia/peregrin");
    assert_eq!(resolve_key("no placeholders", &event), "no placeholders");
    assert_eq!(resolve_key("", &event), "");
}

#[test]
fn key_walks_nested_paths_and_stringifies_values() {
    let event = ev(json!({
        "origin": {"repo": {"slug": "a/b"}},
        "n": 3,
        "b": true,
        "list": ["x", "y", null],
        "obj": {"k": 1},
        "nothing": null,
    }));
    assert_eq!(resolve_key("{origin.repo.slug}", &event), "a/b");
    assert_eq!(resolve_key("{n}", &event), "3");
    assert_eq!(resolve_key("{b}", &event), "true");
    assert_eq!(resolve_key("{list}", &event), "x,y,null");
    assert_eq!(resolve_key("{obj}", &event), "[object Object]");
    assert_eq!(resolve_key("{nothing}", &event), "null");
}

#[test]
fn key_misses_stay_verbatim() {
    let event = ev(json!({"repo": "a/b"}));
    // Absent key.
    assert_eq!(resolve_key("{missing}", &event), "{missing}");
    // Non-object step — `repo` is a string, `slug` can't be walked.
    assert_eq!(resolve_key("{repo.slug}", &event), "{repo.slug}");
    // Empty and unclosed placeholders.
    assert_eq!(resolve_key("{}", &event), "{}");
    assert_eq!(resolve_key("open{repo", &event), "open{repo");
    // Non-object payload — every lookup misses.
    let scalar = Event::new("trigger", json!("not-an-object"), "ingress");
    assert_eq!(resolve_key("{repo}", &scalar), "{repo}");
}

#[test]
fn key_double_brace_emits_literal_and_rescans() {
    let event = ev(json!({"repo": "a/b"}));
    assert_eq!(resolve_key("{{repo}}", &event), "{a/b}");
    assert_eq!(resolve_key("{{", &event), "{{");
}

#[test]
fn queue_key_of_delegates_to_the_policy() {
    let sup = policy("{repo}", Mode::LatestWins, false);
    let queue = Queue::new(sup.clone());
    assert_eq!(queue.policy(), &sup);
    assert_eq!(queue.key_of(&ev(json!({"repo": "x"}))), "x");
}

// ---------------------------------------------------------------------------
// Supersede config serde
// ---------------------------------------------------------------------------

#[test]
fn supersede_deserializes_with_defaults() {
    let parsed: Supersede = serde_json::from_value(json!({})).unwrap();
    assert_eq!(parsed, Supersede::default());
    assert_eq!(parsed.key, "");
    assert_eq!(parsed.mode, Mode::LatestWins);
    assert!(!parsed.cancel_running);
}

#[test]
fn supersede_deserializes_the_yaml_shape() {
    let parsed: Supersede = serde_json::from_value(json!({
        "key": "{repo}:{ref}",
        "mode": "fifo",
        "cancel_running": true,
    }))
    .unwrap();
    assert_eq!(
        parsed,
        policy("{repo}:{ref}", Mode::Fifo, true),
        "kebab-case mode names + snake_case field names"
    );
    // latest-wins parses too.
    let parsed: Supersede = serde_json::from_value(json!({"mode": "latest-wins"})).unwrap();
    assert_eq!(parsed.mode, Mode::LatestWins);
}

#[test]
fn supersede_rejects_unknown_fields_and_round_trips() {
    assert!(serde_json::from_value::<Supersede>(json!({"bogus": 1})).is_err());
    let sup = policy("{repo}", Mode::Fifo, true);
    let round: Supersede = serde_json::from_value(serde_json::to_value(&sup).unwrap()).unwrap();
    assert_eq!(round, sup);
    assert_eq!(serde_json::to_value(Mode::Fifo).unwrap(), json!("fifo"));
    assert_eq!(
        serde_json::to_value(Mode::LatestWins).unwrap(),
        json!("latest-wins")
    );
}

// ---------------------------------------------------------------------------
// Dedupe — latest-wins vs fifo
// ---------------------------------------------------------------------------

#[test]
fn latest_wins_replaces_a_pending_job_with_the_same_key() {
    let mut q = Queue::new(policy("{repo}", Mode::LatestWins, false));
    q.submit(ev(json!({"repo": "a", "sha": "old"})));
    q.submit(ev(json!({"repo": "b", "sha": "other"})));
    let decision = q.submit(ev(json!({"repo": "a", "sha": "new"})));

    assert_eq!(decision.key, "a");
    let Disposition::ReplacedPending(dropped) = decision.disposition else {
        panic!("expected ReplacedPending, got {:?}", decision.disposition);
    };
    assert_eq!(dropped.event.payload["sha"], "old");
    assert!(!decision.cancel_active);
    // Same length, position preserved: a's slot keeps its place, so
    // drain order is the *new* `a` job then `b`.
    assert_eq!(q.pending_len(), 2);
    let first = q.drain().unwrap();
    assert_eq!(first.event.payload["sha"], "new");
    q.finish();
    assert_eq!(q.drain().unwrap().key, "b");
}

#[test]
fn fifo_appends_everything() {
    let mut q = Queue::new(policy("{repo}", Mode::Fifo, false));
    q.submit(ev(json!({"repo": "a", "sha": "1"})));
    let decision = q.submit(ev(json!({"repo": "a", "sha": "2"})));
    assert_eq!(decision.disposition, Disposition::Queued);
    assert_eq!(q.pending_len(), 2);
    let keys: Vec<&str> = q.pending().map(|job| job.key.as_str()).collect();
    assert_eq!(keys, ["a", "a"]);
}

#[test]
fn jobs_carry_seq_key_and_event() {
    let mut q = Queue::new(policy("{repo}", Mode::Fifo, false));
    let event = ev(json!({"repo": "a"}));
    q.submit(event.clone());
    let job = q.drain().unwrap();
    assert_eq!(job.seq, 1);
    assert_eq!(job.key, "a");
    assert_eq!(job.event, event);
    assert_eq!(q.active(), Some(&job));
}

#[test]
fn constant_key_groups_all_events_together() {
    // Empty template → "" → latest-wins keeps only the newest pending.
    let mut q = Queue::new(Supersede::default());
    q.submit(ev(json!({"repo": "a"})));
    q.submit(ev(json!({"repo": "b"})));
    assert_eq!(q.pending_len(), 1);
    assert_eq!(q.drain().unwrap().event.payload["repo"], "b");
}

// ---------------------------------------------------------------------------
// cancel_running
// ---------------------------------------------------------------------------

#[test]
fn cancel_running_signals_on_submit_while_active() {
    let mut q = Queue::new(policy("{repo}", Mode::LatestWins, true));
    q.submit(ev(json!({"repo": "a"})));
    let active = q.drain().unwrap();
    assert_eq!(active.key, "a");

    let decision = q.submit(ev(json!({"repo": "b"})));
    assert!(decision.cancel_active);
    // The signal hands out the job that was active when marked; the
    // new event still queues to run next.
    assert_eq!(q.take_cancel(), Some(active));
    assert_eq!(q.take_cancel(), None, "one-shot signal");
    assert_eq!(q.pending_len(), 1);
}

#[test]
fn cancel_flag_is_not_rearmed_until_consumed() {
    let mut q = Queue::new(policy("{repo}", Mode::Fifo, true));
    q.submit(ev(json!({"repo": "a"})));
    let active = q.drain().unwrap();
    // Two submits while the signal is unread — the flag already held
    // the (single) active job, and both report cancel_active.
    assert!(q.submit(ev(json!({"repo": "b"}))).cancel_active);
    assert!(q.submit(ev(json!({"repo": "c"}))).cancel_active);
    assert_eq!(q.take_cancel(), Some(active));
    assert_eq!(q.take_cancel(), None);
}

#[test]
fn no_cancel_signal_without_running_or_without_the_flag() {
    // cancel_running but no active run — nothing to cancel.
    let mut q = Queue::new(policy("{repo}", Mode::LatestWins, true));
    let decision = q.submit(ev(json!({"repo": "a"})));
    assert!(!decision.cancel_active);
    assert_eq!(q.take_cancel(), None);

    // Active run but cancel_running off — queue behind, no signal.
    let mut q = Queue::new(policy("{repo}", Mode::LatestWins, false));
    q.submit(ev(json!({"repo": "a"})));
    q.drain();
    let decision = q.submit(ev(json!({"repo": "a"})));
    assert!(!decision.cancel_active);
    assert_eq!(q.take_cancel(), None);
    // The same-key event still queued — same target gets another run.
    assert_eq!(q.pending_len(), 1);
}

#[test]
fn finish_clears_an_unread_cancel_signal() {
    let mut q = Queue::new(policy("{repo}", Mode::Fifo, true));
    q.submit(ev(json!({"repo": "a"})));
    let active = q.drain().unwrap();
    assert!(q.submit(ev(json!({"repo": "b"}))).cancel_active);
    // Run ended before the signal was consumed — it's moot now.
    assert_eq!(q.finish(), Some(active));
    assert_eq!(q.take_cancel(), None);
}

// ---------------------------------------------------------------------------
// Drain / lifecycle
// ---------------------------------------------------------------------------

#[test]
fn drain_is_sequential_while_a_run_is_active() {
    let mut q = Queue::new(policy("{repo}", Mode::Fifo, false));
    q.submit(ev(json!({"repo": "a"})));
    q.submit(ev(json!({"repo": "b"})));

    let active = q.drain().unwrap();
    assert_eq!(active.key, "a");
    assert!(q.drain().is_none(), "one run at a time");
    assert!(!q.is_idle());

    assert_eq!(q.finish(), Some(active));
    assert_eq!(q.drain().unwrap().key, "b");
    q.finish();
    assert!(q.drain().is_none(), "empty");
    assert!(q.is_idle());
}

#[test]
fn finish_without_active_returns_none() {
    let mut q = Queue::new(Supersede::default());
    assert_eq!(q.finish(), None);
}

#[test]
fn pending_iterates_in_drain_order() {
    let mut q = Queue::new(policy("{repo}", Mode::Fifo, false));
    for repo in ["a", "b", "c"] {
        q.submit(ev(json!({"repo": repo})));
    }
    let seqs: Vec<u64> = q.pending().map(|job| job.seq).collect();
    assert_eq!(seqs, [1, 2, 3]);
}
