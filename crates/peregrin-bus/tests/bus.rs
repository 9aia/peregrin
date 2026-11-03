#![allow(clippy::assert_is_empty)]
#![allow(clippy::unwrap_used)]
use peregrin_bus::{EventBus, LocalBus, capture_wildcards, matches};
use peregrin_core::{Event, MaxFires, Node, TriggerMode};
use serde_json::{Map, json};

fn node(id: &str, on: &[&str]) -> Node {
    Node {
        id: id.into(),
        on: on.iter().map(|s| (*s).to_string()).collect(),
        ..Node::default()
    }
}

fn bus_with(nodes: &[Node]) -> LocalBus {
    let mut bus = LocalBus::memory();
    bus.reset(nodes, &Map::new());
    bus
}

fn ev(kind: &str) -> Event {
    Event::new(kind, json!({}), "test")
}

// --- TS suite: pattern matching (ac-engine/tests) --------------------

#[test]
fn matches_exact_patterns() {
    assert!(matches("a.b", "a.b"));
    assert!(!matches("a.b", "a.c"));
}

#[test]
fn matches_single_wildcards() {
    assert!(matches("a.*", "a.b"));
    assert!(!matches("a.*", "a.b.c"));
}

#[test]
fn matches_double_wildcards() {
    assert!(matches("a.**", "a.b"));
    assert!(matches("a.**.c", "a.x.y.c"));
    // `**` captures zero or more segments.
    assert!(matches("a.**.c", "a.c"));
    assert!(!matches("a.**.c", "a.x.y"));
}

#[test]
fn captures_wildcard_values() {
    let caps = capture_wildcards("a.*.c", "a.b.c").unwrap();
    assert_eq!(caps["1"], "b");
    assert_eq!(caps["shard"], "b");

    let caps = capture_wildcards("a.**", "a.x.y").unwrap();
    assert_eq!(caps["1"], "x.y");

    assert!(capture_wildcards("a.b", "a.c").is_none());
}

// --- LocalBus ------------------------------------------------------

#[test]
fn publish_readies_any_mode_node() {
    let mut bus = bus_with(&[node("a", &["start"])]);
    let result = bus.publish(&Event::start());
    assert_eq!(result.ready.len(), 1);
    assert_eq!(result.ready[0].0.id, "a");
    assert_eq!(result.ready[0].1[0].kind, "start");
    assert_eq!(bus.fire_count("a"), 1);
    assert_eq!(bus.log().len(), 1);
}

#[test]
fn max_fires_one_retires_after_first() {
    let mut bus = bus_with(&[node("a", &["x"])]);
    bus.publish(&ev("x"));
    // max_fires default 1 → ineligible now; a second `x` produces the
    // node.exhausted side event once.
    let result = bus.publish(&ev("x"));
    assert!(result.ready.is_empty());
    assert_eq!(result.side_events.len(), 1);
    assert_eq!(result.side_events[0].kind, "node.exhausted");
    assert_eq!(result.side_events[0].payload["node"], "a");
    // And only once.
    let result = bus.publish(&ev("x"));
    assert!(result.side_events.is_empty());
}

#[test]
fn until_blocks_and_skips_exhausted_event() {
    let mut n = node("a", &["tick"]);
    n.max_fires = MaxFires::Fires(2);
    n.until = vec!["stop".into()];
    let mut bus = bus_with(&[n]);
    bus.publish(&ev("tick"));
    bus.publish(&ev("stop"));
    // until blocks firing entirely — no ready, no exhausted signal.
    let result = bus.publish(&ev("tick"));
    assert!(result.ready.is_empty());
    assert!(result.side_events.is_empty());
}

#[test]
fn retire_on_removes_node() {
    let mut n = node("a", &["go"]);
    n.max_fires = MaxFires::Unlimited;
    n.retire_on = vec!["done".into()];
    let mut bus = bus_with(&[n]);
    let result = bus.publish(&ev("done"));
    assert_eq!(result.retired, vec!["a"]);
    // Retired nodes never fire again.
    assert!(bus.publish(&ev("go")).ready.is_empty());
}

#[test]
fn mode_all_waits_for_every_trigger() {
    let mut n = node("a", &["one", "two"]);
    n.mode = TriggerMode::All;
    let mut bus = bus_with(&[n]);
    // `one` alone: matches a pattern but `all` isn't satisfied — since
    // matches_any is true the node isn't readied, no exhaust either.
    let result = bus.publish(&ev("one"));
    assert!(result.ready.is_empty());
    let result = bus.publish(&ev("two"));
    assert_eq!(result.ready.len(), 1);
    // collectAllTriggers: first matching event per pattern.
    let kinds: Vec<&str> = result.ready[0].1.iter().map(|e| e.kind.as_str()).collect();
    assert_eq!(kinds, vec!["one", "two"]);
}

#[test]
fn fan_in_waits_for_count() {
    let mut n = node("fanin", &["shard.*.ready"]);
    n.fan_in = Some(Map::from_iter([("count".into(), json!(2))]));
    let mut bus = bus_with(&[n]);
    // Not eligible until 2 matching events have been seen.
    let result = bus.publish(&Event::new("shard.1.ready", json!({"id": 1}), "m"));
    assert!(result.ready.is_empty());
    let result = bus.publish(&Event::new("shard.2.ready", json!({"id": 2}), "m"));
    assert_eq!(result.ready.len(), 1);
    // collectFanInTriggers hands over every matching event.
    assert_eq!(result.ready[0].1.len(), 2);
}

#[test]
fn fan_in_count_from_config() {
    let mut n = node("fanin", &["shard.*.ready"]);
    n.fan_in = Some(Map::from_iter([("count_from".into(), json!("ids"))]));
    let mut bus = LocalBus::memory();
    let mut config = Map::new();
    config.insert("ids".into(), json!(["a", "b", "c"]));
    bus.reset(&[n], &config);
    bus.publish(&Event::new("shard.1.ready", json!({}), "m"));
    bus.publish(&Event::new("shard.2.ready", json!({}), "m"));
    let result = bus.publish(&Event::new("shard.3.ready", json!({}), "m"));
    assert_eq!(result.ready.len(), 1);
    assert_eq!(result.ready[0].1.len(), 3);
}

#[test]
fn seed_counts_fires_and_check_ready_readies() {
    let mut bus = bus_with(&[node("a", &["seen.x"]), node("b", &["seen.x"])]);
    bus.seed(&[
        Event::new("seen.x", json!({}), "a"),
        Event::new("seen.x", json!({}), "__runner__"),
    ]);
    // seed counts non-runner sources as fires; "a" fired once.
    assert_eq!(bus.fire_count("a"), 1);
    let result = bus.check_ready();
    // "a" is max-fires exhausted (counted 1 ≥ 1) → exhausted side event;
    // "b" is ready on the seeded event.
    assert_eq!(result.side_events.len(), 1);
    assert_eq!(result.side_events[0].kind, "node.exhausted");
    assert_eq!(result.ready.len(), 1);
    assert_eq!(result.ready[0].0.id, "b");
}

#[test]
fn inject_registers_node_specs() {
    let mut bus = bus_with(&[]);
    bus.inject(&[json!({"id": "late", "on": ["go"]})]);
    assert_eq!(bus.fire_count("late"), 0);
    let result = bus.publish(&ev("go"));
    assert_eq!(result.ready.len(), 1);
    assert_eq!(result.ready[0].0.id, "late");
}

#[test]
fn wildcard_trigger_matches() {
    let mut bus = bus_with(&[node("a", &["shard.*.ready"])]);
    let result = bus.publish(&Event::new("shard.9.ready", json!({"id": 9}), "m"));
    assert_eq!(result.ready.len(), 1);
    assert_eq!(result.ready[0].1[0].kind, "shard.9.ready");
}
