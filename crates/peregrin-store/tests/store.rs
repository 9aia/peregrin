#![allow(clippy::assert_is_empty)]
#![allow(clippy::unwrap_used)]
use peregrin_core::Event;
use peregrin_store::{DeadLetterQueue, EventStore, MemoryDlq, MemoryStore, NoopDlq, StateStore};
use serde_json::json;

#[test]
fn stores_and_retrieves_events_by_type() {
    // Port of ac-store-memory's test.
    let mut store = MemoryStore::default();
    store.append(&Event::new("a", json!({"n": 1}), "t"));
    store.append(&Event::new("b", json!({}), "t"));
    store.append(&Event::new("a", json!({"n": 2}), "t"));
    assert_eq!(store.all().len(), 3);
    assert_eq!(store.by_type("a").len(), 2);
    assert!(store.seen_types().contains(&"a".to_string()));
    assert_eq!(store.latest().unwrap().kind, "a");
    EventStore::clear(&mut store);
    assert_eq!(store.all().len(), 0);
}

#[test]
fn seen_types_preserve_first_seen_order() {
    let mut store = MemoryStore::default();
    store.append(&Event::new("b", json!({}), "t"));
    store.append(&Event::new("a", json!({}), "t"));
    store.append(&Event::new("b", json!({}), "t"));
    assert_eq!(store.seen_types(), vec!["b", "a"]);
}

#[test]
fn state_store_roundtrip_and_clear() {
    let mut store = MemoryStore::default();
    assert!(store.get("k").is_none());
    store.set("k", json!({"n": 1}));
    assert_eq!(store.get("k"), Some(json!({"n": 1})));
    assert_eq!(store.to_record()["k"], json!({"n": 1}));
    StateStore::clear(&mut store);
    assert!(store.get("k").is_none());
    assert!(store.to_record().is_empty());
    // EventStore::clear also drops state (TS parity).
    store.set("k", json!(1));
    EventStore::clear(&mut store);
    assert!(store.get("k").is_none());
}

#[test]
fn memory_dlq_collects_noop_drops() {
    let mut dlq = MemoryDlq::default();
    dlq.enqueue(
        &Event::new("node.failed", json!({}), "n"),
        "retry exhausted",
    );
    assert_eq!(dlq.all().len(), 1);

    let mut noop = NoopDlq;
    noop.enqueue(
        &Event::new("node.failed", json!({}), "n"),
        "retry exhausted",
    );
    assert!(noop.all().is_empty());
}
