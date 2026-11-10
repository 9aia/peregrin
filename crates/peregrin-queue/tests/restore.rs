#![allow(clippy::unwrap_used, clippy::pedantic)]
//! Durable-queue tests — `Queue::snapshot`/`Queue::restore` round-trips,
//! the running-marker front re-queue, restore-time re-validation, and
//! the `fifo` per-key bound.

use peregrin_core::Event;
use peregrin_queue::{
    Disposition, Job, MAX_PENDING_PER_KEY, MemoryQueueStore, Mode, Queue, QueueSnapshot,
    QueueStore, RestoreOutcome, Supersede,
};
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

fn pending_shas(queue: &Queue) -> Vec<&str> {
    queue
        .pending()
        .map(|job| job.event.payload["sha"].as_str().unwrap())
        .collect()
}

// ---------------------------------------------------------------------------
// snapshot / restore round-trip
// ---------------------------------------------------------------------------

#[test]
fn snapshot_restore_round_trips_pending_and_running() {
    let mut q = Queue::new(policy("{repo}", Mode::Fifo, false));
    q.submit(ev(json!({"repo": "a", "sha": "1"})));
    q.submit(ev(json!({"repo": "b", "sha": "2"})));
    let active = q.drain().unwrap(); // repo a runs
    q.submit(ev(json!({"repo": "c", "sha": "3"})));

    let snapshot = q.snapshot();
    assert_eq!(snapshot.running, Some(active));
    assert_eq!(snapshot.pending.len(), 2);

    let RestoreOutcome { queue, dropped } =
        Queue::restore(policy("{repo}", Mode::Fifo, false), snapshot);
    assert!(dropped.is_empty());
    // The interrupted run re-queues at the FRONT — at-least-once.
    assert_eq!(pending_shas(&queue), ["1", "2", "3"]);
    assert!(queue.active().is_none(), "restore never marks a run active");
}

#[test]
fn restore_orders_across_keys_by_seq() {
    // Interleaved keys: a(1) b(2) a(3) — fifo appends per key.
    let mut q = Queue::new(policy("{repo}", Mode::Fifo, false));
    q.submit(ev(json!({"repo": "a", "sha": "1"})));
    q.submit(ev(json!({"repo": "b", "sha": "2"})));
    q.submit(ev(json!({"repo": "a", "sha": "3"})));
    let snapshot = q.snapshot();

    let restored = Queue::restore(policy("{repo}", Mode::Fifo, false), snapshot).queue;
    assert_eq!(pending_shas(&restored), ["1", "2", "3"]);
}

#[test]
fn latest_wins_replacement_keeps_drain_order_through_restore() {
    // a-old(1) b(2); replace a → newest inherits seq 1 and the front
    // slot. Without seq inheritance a seq-sort would reorder to b,a.
    let mut q = Queue::new(policy("{repo}", Mode::LatestWins, false));
    q.submit(ev(json!({"repo": "a", "sha": "old"})));
    q.submit(ev(json!({"repo": "b", "sha": "2"})));
    q.submit(ev(json!({"repo": "a", "sha": "new"})));
    let snapshot = q.snapshot();

    let restored = Queue::restore(policy("{repo}", Mode::LatestWins, false), snapshot).queue;
    assert_eq!(pending_shas(&restored), ["new", "2"]);
}

#[test]
fn restore_continues_the_seq_counter() {
    let mut q = Queue::new(policy("{repo}", Mode::Fifo, false));
    for sha in ["1", "2", "3"] {
        q.submit(ev(json!({"repo": "a", "sha": sha})));
    }
    let restored = Queue::restore(policy("{repo}", Mode::Fifo, false), q.snapshot()).queue;
    let mut restored = restored;
    restored.submit(ev(json!({"repo": "a", "sha": "4"})));
    let last = restored.pending().last().unwrap();
    assert_eq!(last.event.payload["sha"], "4");
    assert!(last.seq > 3, "post-restore submits continue the counter");
}

// ---------------------------------------------------------------------------
// restore-time validation
// ---------------------------------------------------------------------------

#[test]
fn restore_drops_entries_whose_key_no_longer_resolves() {
    // A job persisted under key "{repo}" whose event now resolves a
    // different key — the `supersede:` block changed (or the row is
    // corrupt). Dropped, not re-keyed.
    let mut q = Queue::new(policy("{repo}", Mode::Fifo, false));
    q.submit(ev(json!({"repo": "a", "sha": "ok"})));
    let mut snapshot = q.snapshot();
    let mut stale = Job {
        seq: 99,
        key: "stale-key".to_string(),
        event: ev(json!({"repo": "b", "sha": "stale"})),
    };
    stale.key = "not-the-resolved-key".to_string();
    snapshot.pending.push(stale);

    let outcome = Queue::restore(policy("{repo}", Mode::Fifo, false), snapshot);
    assert_eq!(outcome.dropped.len(), 1);
    assert_eq!(outcome.dropped[0].event.payload["sha"], "stale");
    assert_eq!(pending_shas(&outcome.queue), ["ok"]);
}

#[test]
fn restore_dedupes_pending_like_live_submits() {
    // A hand-built snapshot with two same-key latest-wins entries —
    // restore runs them through `place`, so only the newest survives
    // (in the earlier job's position).
    let snapshot = QueueSnapshot {
        pending: vec![
            Job {
                seq: 1,
                key: "a".to_string(),
                event: ev(json!({"repo": "a", "sha": "old"})),
            },
            Job {
                seq: 2,
                key: "b".to_string(),
                event: ev(json!({"repo": "b", "sha": "2"})),
            },
            Job {
                seq: 3,
                key: "a".to_string(),
                event: ev(json!({"repo": "a", "sha": "new"})),
            },
        ],
        running: None,
    };
    let restored = Queue::restore(policy("{repo}", Mode::LatestWins, false), snapshot).queue;
    assert_eq!(pending_shas(&restored), ["new", "2"]);
}

#[test]
fn running_marker_requeues_at_front_under_latest_wins() {
    // running key "a" seq 1 + pending same-key "a" seq 5 (newer):
    // latest-wins supersedes the interrupted run with the newer
    // trigger — it inherits the front position.
    let snapshot = QueueSnapshot {
        pending: vec![Job {
            seq: 5,
            key: "a".to_string(),
            event: ev(json!({"repo": "a", "sha": "newer"})),
        }],
        running: Some(Job {
            seq: 1,
            key: "a".to_string(),
            event: ev(json!({"repo": "a", "sha": "interrupted"})),
        }),
    };
    let restored = Queue::restore(policy("{repo}", Mode::LatestWins, false), snapshot).queue;
    assert_eq!(pending_shas(&restored), ["newer"]);
}

#[test]
fn running_marker_requeues_at_front_under_fifo() {
    let snapshot = QueueSnapshot {
        pending: vec![Job {
            seq: 5,
            key: "a".to_string(),
            event: ev(json!({"repo": "a", "sha": "pending"})),
        }],
        running: Some(Job {
            seq: 1,
            key: "a".to_string(),
            event: ev(json!({"repo": "a", "sha": "interrupted"})),
        }),
    };
    let restored = Queue::restore(policy("{repo}", Mode::Fifo, false), snapshot).queue;
    // fifo: every trigger runs — interrupted first, then pending.
    assert_eq!(pending_shas(&restored), ["interrupted", "pending"]);
}

// ---------------------------------------------------------------------------
// fifo per-key bound
// ---------------------------------------------------------------------------

#[test]
fn fifo_evicts_the_oldest_pending_job_at_the_bound() {
    let mut q = Queue::new(policy("{repo}", Mode::Fifo, false));
    for i in 0..MAX_PENDING_PER_KEY {
        q.submit(ev(json!({"repo": "a", "sha": i.to_string()})));
    }
    assert_eq!(q.pending_len(), MAX_PENDING_PER_KEY);
    // One more same-key submit evicts the OLDEST ("0"); the new one
    // queues at the back. An unrelated key is untouched.
    q.submit(ev(json!({"repo": "b", "sha": "other"})));
    let decision = q.submit(ev(json!({"repo": "a", "sha": "new"})));
    let Disposition::ReplacedPending(evicted) = decision.disposition else {
        panic!("expected eviction, got {:?}", decision.disposition);
    };
    assert_eq!(evicted.event.payload["sha"], "0");
    let shas = pending_shas(&q);
    assert_eq!(shas.len(), MAX_PENDING_PER_KEY + 1);
    assert_eq!(shas[0], "1", "oldest for the key dropped");
    assert_eq!(shas[shas.len() - 1], "new", "newest queued at back");
    assert!(shas.contains(&"other"));
}

#[test]
fn latest_wins_never_hits_the_fifo_bound() {
    let mut q = Queue::new(policy("{repo}", Mode::LatestWins, false));
    for i in 0..(MAX_PENDING_PER_KEY + 10) {
        q.submit(ev(json!({"repo": "a", "sha": i.to_string()})));
    }
    assert_eq!(q.pending_len(), 1, "latest-wins keeps one per key");
}

// ---------------------------------------------------------------------------
// MemoryQueueStore — the in-memory port impl
// ---------------------------------------------------------------------------

#[test]
fn memory_store_round_trips_a_snapshot() {
    let mut q = Queue::new(policy("{repo}", Mode::Fifo, false));
    q.submit(ev(json!({"repo": "a", "sha": "1"})));
    q.submit(ev(json!({"repo": "b", "sha": "2"})));
    let active = q.drain().unwrap();
    q.submit(ev(json!({"repo": "c", "sha": "3"})));

    let store = MemoryQueueStore::with_snapshot(q.snapshot());
    // Simulate what the supervisor persists: running marker + the
    // pending set (per key).
    store.set_running(Some(&active)).unwrap();
    for job in q.pending() {
        // persist_key replaces the whole key set — collect per key.
        let jobs: Vec<Job> = q.pending().filter(|j| j.key == job.key).cloned().collect();
        store.persist_key(&job.key, &jobs).unwrap();
    }

    let snapshot = store.load().unwrap();
    assert_eq!(snapshot.running, Some(active));
    assert_eq!(snapshot.pending.len(), 2);
    let restored = Queue::restore(policy("{repo}", Mode::Fifo, false), snapshot).queue;
    assert_eq!(pending_shas(&restored), ["1", "2", "3"]);
}

#[test]
fn memory_store_persist_key_is_per_key() {
    let store = MemoryQueueStore::new();
    let job_a = Job {
        seq: 1,
        key: "a".to_string(),
        event: ev(json!({"repo": "a"})),
    };
    let job_b = Job {
        seq: 2,
        key: "b".to_string(),
        event: ev(json!({"repo": "b"})),
    };
    store
        .persist_key("a", std::slice::from_ref(&job_a))
        .unwrap();
    store
        .persist_key("b", std::slice::from_ref(&job_b))
        .unwrap();
    // Rewriting key "a" leaves "b" intact — no clobber.
    store.persist_key("a", &[]).unwrap();
    let snapshot = store.load().unwrap();
    assert_eq!(snapshot.pending, vec![job_b]);
    assert!(snapshot.running.is_none());
}
