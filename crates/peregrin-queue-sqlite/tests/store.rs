#![allow(clippy::unwrap_used, clippy::pedantic)]
//! SqliteQueueStore tests — file round-trips, per-key write
//! isolation, the running marker, reopen-persistence, and corrupt
//! row tolerance.

use std::path::PathBuf;

use peregrin_core::Event;
use peregrin_queue::{Job, QueueSnapshot, QueueStore};
use peregrin_queue_sqlite::SqliteQueueStore;
use serde_json::json;

fn job(seq: u64, key: &str, sha: &str) -> Job {
    Job {
        seq,
        key: key.to_string(),
        event: Event::new("push", json!({"repo": key, "sha": sha}), "test"),
    }
}

fn store_path(name: &str) -> PathBuf {
    let tmp = tempfile::tempdir().unwrap();
    // Leak the tempdir — the store must outlive the test fn for
    // reopen checks; /tmp files are small and the OS reaps them.
    tmp.keep().join(name)
}

#[test]
fn round_trip_pending_and_running() {
    let path = store_path("queue.db");
    let store = SqliteQueueStore::open(&path).unwrap();
    store
        .persist_key("a", &[job(1, "a", "1"), job(3, "a", "3")])
        .unwrap();
    store.persist_key("b", &[job(2, "b", "2")]).unwrap();
    let running = job(0, "r", "run");
    store.set_running(Some(&running)).unwrap();

    let snapshot = store.load().unwrap();
    // seq order merges the per-key rows into drain order.
    assert_eq!(
        snapshot
            .pending
            .iter()
            .map(|j| (j.key.as_str(), j.seq))
            .collect::<Vec<_>>(),
        [("a", 1), ("b", 2), ("a", 3)]
    );
    assert_eq!(snapshot.running, Some(running));
}

#[test]
fn persist_key_replaces_only_its_key() {
    let path = store_path("queue.db");
    let store = SqliteQueueStore::open(&path).unwrap();
    store.persist_key("a", &[job(1, "a", "1")]).unwrap();
    store.persist_key("b", &[job(2, "b", "2")]).unwrap();
    store.persist_key("a", &[job(3, "a", "3")]).unwrap();
    store.persist_key("b", &[]).unwrap();

    let snapshot = store.load().unwrap();
    assert_eq!(snapshot.pending.len(), 1);
    assert_eq!(snapshot.pending[0].key, "a");
    assert_eq!(snapshot.pending[0].event.payload["sha"], "3");
}

#[test]
fn set_running_clears_the_marker() {
    let path = store_path("queue.db");
    let store = SqliteQueueStore::open(&path).unwrap();
    store.set_running(Some(&job(1, "a", "1"))).unwrap();
    assert!(store.load().unwrap().running.is_some());
    store.set_running(None).unwrap();
    assert!(store.load().unwrap().running.is_none());
}

#[test]
fn state_survives_reopen() {
    let path = store_path("queue.db");
    {
        let store = SqliteQueueStore::open(&path).unwrap();
        store.persist_key("a", &[job(1, "a", "1")]).unwrap();
        store.set_running(Some(&job(2, "a", "run"))).unwrap();
    }
    let store = SqliteQueueStore::open(&path).unwrap();
    let snapshot = store.load().unwrap();
    assert_eq!(snapshot.pending.len(), 1);
    assert_eq!(snapshot.running.unwrap().event.payload["sha"], "run");
}

#[test]
fn corrupt_rows_are_skipped_on_load() {
    let path = store_path("queue.db");
    let store = SqliteQueueStore::open(&path).unwrap();
    store.persist_key("a", &[job(1, "a", "1")]).unwrap();
    // Inject garbage directly — a torn write or foreign writer.
    {
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute(
            "INSERT INTO queue_pending (key, seq, job) VALUES ('bad', 2, 'not-json')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO queue_running (slot, job) VALUES (0, 'also-not-json')",
            [],
        )
        .unwrap();
    }
    let snapshot = store.load().unwrap();
    assert_eq!(snapshot.pending.len(), 1, "corrupt pending row skipped");
    assert_eq!(snapshot.pending[0].key, "a");
    assert!(snapshot.running.is_none(), "corrupt running marker ignored");
}

#[test]
fn empty_store_loads_an_empty_snapshot() {
    let path = store_path("queue.db");
    let store = SqliteQueueStore::open(&path).unwrap();
    let snapshot = store.load().unwrap();
    assert_eq!(snapshot, QueueSnapshot::default());
}
