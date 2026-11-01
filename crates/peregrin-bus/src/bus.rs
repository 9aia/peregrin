//! The [`EventBus`] port and the in-process [`LocalBus`] — port of the
//! TS `EventBusLive`. Publishing an event appends it to the
//! [`EventStore`], retires `retire_on` nodes, and computes the next
//! `ready` set: nodes whose `on` triggers are satisfied by the log so
//! far. Node iteration follows registration order, matching JS `Map`
//! insertion order.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use serde_json::{Map, Value};

use peregrin_core::{Event, MaxFires, Node, TriggerMode};
use peregrin_store::EventStore;

use crate::matching::matches;
use crate::observer::{NoopObserver, Observer};

/// What one publish (or a ready-check) produced.
#[derive(Debug, Default)]
pub struct PublishResult {
    /// Nodes that fired, with their trigger events (TS `ready`).
    pub ready: Vec<(Node, Vec<Event>)>,
    /// Side events the bus itself produced (`node.exhausted`).
    pub side_events: Vec<Event>,
    /// Node ids retired by `retire_on` this publish.
    pub retired: Vec<String>,
}

/// The bus port the runner drives (TS `EventBus`).
pub trait EventBus: Send {
    /// Rebind to a new campaign's nodes + config; clears fire counts
    /// and the exhausted set (TS `reset`).
    fn reset(&mut self, nodes: &[Node], config: &Map<String, Value>);
    /// Append an event and compute what it readies (TS `publish`).
    fn publish(&mut self, event: &Event) -> PublishResult;
    /// Load replay events without computing readiness (TS `seed`).
    fn seed(&mut self, events: &[Event]);
    /// Compute readiness against the current log — used after `seed`
    /// (TS `checkReady`).
    fn check_ready(&mut self) -> PublishResult;
    /// The full event log (TS `log`).
    fn log(&self) -> Vec<Event>;
    /// Times a node has been scheduled to fire (TS `fireCount`).
    fn fire_count(&self, node_id: &str) -> u64;
    /// Add nodes mid-run; invalid specs are skipped (TS `inject`).
    fn inject(&mut self, node_specs: &[Value]);
}

/// A snapshot of seen event types: `order` iterates in first-seen
/// order (JS `Set` semantics), `set` answers membership.
struct Seen {
    order: Vec<String>,
    set: HashSet<String>,
}

impl Seen {
    fn contains(&self, kind: &str) -> bool {
        self.set.contains(kind)
    }
}

/// The in-process bus — direct port of `EventBusLive`.
pub struct LocalBus {
    nodes: HashMap<String, Node>,
    /// Registration order — JS `Map` iterates in insertion order and
    /// `ready` order follows it.
    order: Vec<String>,
    fire_counts: HashMap<String, u64>,
    exhausted: HashSet<String>,
    config: Map<String, Value>,
    store: Box<dyn EventStore>,
    observer: Arc<dyn Observer>,
}

impl LocalBus {
    /// A bus over `store` reporting to `observer`.
    pub fn new(store: Box<dyn EventStore>, observer: Arc<dyn Observer>) -> Self {
        Self {
            nodes: HashMap::new(),
            order: Vec::new(),
            fire_counts: HashMap::new(),
            exhausted: HashSet::new(),
            config: Map::new(),
            store,
            observer,
        }
    }

    /// In-memory store + no-op observer — the common case.
    #[must_use]
    pub fn memory() -> Self {
        Self::new(
            Box::new(peregrin_store::MemoryStore::default()),
            Arc::new(NoopObserver),
        )
    }

    /// Read the store back (e.g. the run's event log after a run).
    pub fn store(&self) -> &dyn EventStore {
        &*self.store
    }

    fn seen(&self) -> Seen {
        let order = self.store.seen_types();
        let set = order.iter().cloned().collect();
        Seen { order, set }
    }

    /// `checkRetirements` — drop nodes whose `retire_on` type has been
    /// seen; returns the retired ids in registration order.
    fn check_retirements(&mut self, seen: &Seen) -> Vec<String> {
        let mut to_retire = Vec::new();
        for id in &self.order {
            let Some(node) = self.nodes.get(id) else {
                continue;
            };
            if node.retire_on.iter().any(|t| seen.contains(t)) {
                to_retire.push(id.clone());
            }
        }
        for id in &to_retire {
            self.nodes.remove(id);
            self.observer.on_retire(id);
        }
        to_retire
    }

    fn is_max_fires_exhausted(&self, node: &Node) -> bool {
        match node.max_fires {
            MaxFires::Unlimited => false,
            MaxFires::Fires(cap) => self.fire_counts.get(&node.id).copied().unwrap_or(0) >= cap,
        }
    }

    fn is_until_blocked(node: &Node, seen: &Seen) -> bool {
        node.until.iter().any(|t| seen.contains(t))
    }

    fn is_eligible(&self, node: &Node, seen: &Seen) -> bool {
        !self.is_max_fires_exhausted(node)
            && !Self::is_until_blocked(node, seen)
            && self.is_fan_in_satisfied(node, seen)
    }

    /// `resolveFanInCount` — `count` wins, then `count_from` (a config
    /// path → array length or number), default 1 (ADR-0005: fan-in
    /// always has a bounded count).
    fn resolve_fan_in_count(&self, fan_in: &Map<String, Value>) -> u64 {
        if let Some(raw) = fan_in.get("count") {
            let c = js_number(raw);
            return if c.is_nan() { 1 } else { c.max(1.0) as u64 };
        }
        if let Some(raw) = fan_in.get("count_from") {
            let path = peregrin_core::js_string(raw);
            let mut current = self.config.get(path.split('.').next().unwrap_or_default());
            for part in path.split('.').skip(1) {
                current = current
                    .and_then(Value::as_object)
                    .and_then(|map| map.get(part));
            }
            let Some(value) = current else {
                return 1;
            };
            if let Value::Array(items) = value {
                return items.len() as u64;
            }
            let n = js_number(value);
            return if n.is_nan() { 1 } else { n.max(1.0) as u64 };
        }
        1
    }

    fn is_fan_in_satisfied(&self, node: &Node, seen: &Seen) -> bool {
        let Some(fan_in) = &node.fan_in else {
            return true;
        };
        let count = self.resolve_fan_in_count(fan_in);
        let Some(pattern) = node.on.first() else {
            return false;
        };
        self.count_matching(pattern, seen) >= count
    }

    /// `countMatching` — number of logged events matching `pattern`.
    fn count_matching(&self, pattern: &str, seen: &Seen) -> u64 {
        if !pattern.contains('*') {
            return self.store.by_type(pattern).len() as u64;
        }
        let mut total = 0_u64;
        for kind in &seen.order {
            if matches(pattern, kind) {
                total += self.store.by_type(kind).len() as u64;
            }
        }
        total
    }

    /// `eventsMatching` — every logged event whose type matches.
    fn events_matching(&self, pattern: &str, seen: &Seen) -> Vec<Event> {
        if !pattern.contains('*') {
            return self.store.by_type(pattern).to_vec();
        }
        let mut result = Vec::new();
        for kind in &seen.order {
            if matches(pattern, kind) {
                result.extend_from_slice(self.store.by_type(kind));
            }
        }
        result
    }

    /// `collectFanInTriggers` — all events matching `on[0]`.
    fn collect_fan_in_triggers(&self, node: &Node, seen: &Seen) -> Vec<Event> {
        match node.on.first() {
            None => Vec::new(),
            Some(pattern) => self.events_matching(pattern, seen),
        }
    }

    /// `matchesAny` — does this event type match any `on` pattern?
    fn matches_any(node: &Node, event: &Event) -> bool {
        node.on.iter().any(|p| matches(p, &event.kind))
    }

    /// `matchesAnyType` — has any `on` pattern been seen at all?
    fn matches_any_type(node: &Node, seen: &Seen) -> bool {
        node.on.iter().any(|pattern| {
            if pattern.contains('*') {
                seen.order.iter().any(|kind| matches(pattern, kind))
            } else {
                seen.contains(pattern)
            }
        })
    }

    /// `firstMatchingAnyPattern` — first logged event matching any
    /// `on` pattern, in `on` order.
    fn first_matching_any_pattern(&self, node: &Node, seen: &Seen) -> Option<Event> {
        for pattern in &node.on {
            if let Some(event) = self.first_matching(pattern, seen) {
                return Some(event);
            }
        }
        None
    }

    /// `allPatternsSatisfied` — every `on` pattern has been seen.
    fn all_patterns_satisfied(node: &Node, seen: &Seen) -> bool {
        node.on.iter().all(|pattern| {
            if pattern.contains('*') {
                seen.order.iter().any(|kind| matches(pattern, kind))
            } else {
                seen.contains(pattern)
            }
        })
    }

    /// `firstMatching` — the first logged event matching `pattern`
    /// (first seen type for wildcards, in first-seen order).
    fn first_matching(&self, pattern: &str, seen: &Seen) -> Option<Event> {
        if !pattern.contains('*') {
            return self.store.by_type(pattern).first().cloned();
        }
        for kind in &seen.order {
            if matches(pattern, kind) {
                if let Some(event) = self.store.by_type(kind).first() {
                    return Some(event.clone());
                }
            }
        }
        None
    }

    /// `collectAllTriggers` — first matching event per distinct `on`
    /// pattern.
    fn collect_all_triggers(&self, node: &Node, seen: &Seen) -> Vec<Event> {
        let mut triggers = Vec::new();
        let mut seen_patterns = HashSet::new();
        for pattern in &node.on {
            if !seen_patterns.insert(pattern) {
                continue;
            }
            if let Some(first) = self.first_matching(pattern, seen) {
                triggers.push(first);
            }
        }
        triggers
    }

    /// The `node.exhausted` side-event check shared by `publish` and
    /// `check_ready` (ADR-0005/ADR-0014).
    fn exhaustion_side_event(&mut self, node: &Node, matched: bool, seen: &Seen) -> Option<Event> {
        if self.is_max_fires_exhausted(node)
            && !Self::is_until_blocked(node, seen)
            && !self.exhausted.contains(&node.id)
            && matched
            && (node.mode != TriggerMode::All || Self::all_patterns_satisfied(node, seen))
        {
            self.exhausted.insert(node.id.clone());
            return Some(Event::node_exhausted(&node.id, node.max_fires));
        }
        None
    }
}

impl EventBus for LocalBus {
    fn reset(&mut self, nodes: &[Node], config: &Map<String, Value>) {
        self.nodes = nodes.iter().map(|n| (n.id.clone(), n.clone())).collect();
        self.order = nodes.iter().map(|n| n.id.clone()).collect();
        self.fire_counts.clear();
        self.exhausted.clear();
        self.config.clone_from(config);
    }

    /// `publishSync` — append, retire, then for every node in
    /// registration order decide eligibility and readiness.
    fn publish(&mut self, event: &Event) -> PublishResult {
        self.store.append(event);
        self.observer.on_event(event);
        let seen = self.seen();
        let retired = self.check_retirements(&seen);
        let mut result = PublishResult {
            retired,
            ..PublishResult::default()
        };
        let order = self.order.clone();
        for id in &order {
            let Some(node) = self.nodes.get(id).cloned() else {
                continue;
            };
            if !self.is_eligible(&node, &seen) {
                let matched = Self::matches_any(&node, event);
                if let Some(side) = self.exhaustion_side_event(&node, matched, &seen) {
                    result.side_events.push(side);
                }
                continue;
            }
            if !Self::matches_any(&node, event) {
                continue;
            }
            let triggers = if node.mode == TriggerMode::All {
                if !Self::all_patterns_satisfied(&node, &seen) {
                    continue;
                }
                self.collect_all_triggers(&node, &seen)
            } else if node.fan_in.is_some() {
                self.collect_fan_in_triggers(&node, &seen)
            } else {
                vec![event.clone()]
            };
            result.ready.push((node.clone(), triggers));
            *self.fire_counts.entry(node.id.clone()).or_insert(0) += 1;
        }
        result
    }

    /// `seedSync` — append replay events and count prior fires per
    /// `source` (runner-sourced events don't count).
    fn seed(&mut self, events: &[Event]) {
        for event in events {
            self.store.append(event);
            if !event.source.is_empty() && event.source != "__runner__" {
                *self.fire_counts.entry(event.source.clone()).or_insert(0) += 1;
            }
        }
    }

    /// `checkReadySync` — readiness from the log alone (post-`seed`);
    /// per node, the trigger is the first matching logged event (or
    /// all/fan-in collections).
    fn check_ready(&mut self) -> PublishResult {
        let seen = self.seen();
        let retired = self.check_retirements(&seen);
        let mut result = PublishResult {
            retired,
            ..PublishResult::default()
        };
        let mut added = HashSet::new();
        let order = self.order.clone();
        for id in &order {
            let Some(node) = self.nodes.get(id).cloned() else {
                continue;
            };
            if !self.is_eligible(&node, &seen) {
                let matched = Self::matches_any_type(&node, &seen);
                if let Some(side) = self.exhaustion_side_event(&node, matched, &seen) {
                    result.side_events.push(side);
                }
                continue;
            }
            if !Self::matches_any_type(&node, &seen) {
                continue;
            }
            let triggers = if node.mode == TriggerMode::All {
                if !Self::all_patterns_satisfied(&node, &seen) {
                    continue;
                }
                self.collect_all_triggers(&node, &seen)
            } else if node.fan_in.is_some() {
                self.collect_fan_in_triggers(&node, &seen)
            } else {
                match self.first_matching_any_pattern(&node, &seen) {
                    Some(event) => vec![event],
                    None => continue,
                }
            };
            if !added.insert(node.id.clone()) {
                continue;
            }
            result.ready.push((node.clone(), triggers));
            *self.fire_counts.entry(node.id.clone()).or_insert(0) += 1;
        }
        result
    }

    fn log(&self) -> Vec<Event> {
        self.store.all().to_vec()
    }

    fn fire_count(&self, node_id: &str) -> u64 {
        self.fire_counts.get(node_id).copied().unwrap_or(0)
    }

    /// `inject` — parse specs into nodes and register them; a spec
    /// without a usable node shape is skipped (the TS `try/catch`).
    fn inject(&mut self, node_specs: &[Value]) {
        for spec in node_specs {
            // `Node.fromDict` is total (unlike the TS `try/catch`), so
            // every spec registers — even an id-less one, like the TS.
            let node = Node::from_dict(spec);
            if !self.nodes.contains_key(&node.id) {
                self.order.push(node.id.clone());
            }
            self.nodes.insert(node.id.clone(), node.clone());
            self.fire_counts.insert(node.id, 0);
        }
        self.observer.on_inject(node_specs);
    }
}

/// JS `Number(v)` for the fields fan-in reads: numbers pass, numeric
/// strings parse, everything else is `NaN`.
fn js_number(value: &Value) -> f64 {
    match value {
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::String(s) => s.parse::<f64>().unwrap_or(f64::NAN),
        Value::Bool(b) => f64::from(u8::from(*b)),
        _ => f64::NAN,
    }
}
