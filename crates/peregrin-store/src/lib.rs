//! peregrin-store — the persistence ports the engine calls: the
//! append-only event log ([`EventStore`]), the key/value run state
//! ([`StateStore`]), and the dead-letter queue ([`DeadLetterQueue`],
//! ADR-0002). In-memory impls ship here; durable impls (jsonl, redb)
//! are separate crates.

use std::collections::HashMap;

use serde_json::{Map, Value};

use peregrin_core::Event;

/// The event-log port — the live log of published events the bus
/// queries for triggers and the runner returns as the run record
/// (TS `EventStore`).
pub trait EventStore: Send {
    /// Append an event to the log (TS `append`).
    fn append(&mut self, event: &Event);
    /// Every event in publish order (TS `all`).
    fn all(&self) -> &[Event];
    /// Events of one type in publish order (TS `byType`).
    fn by_type(&self, kind: &str) -> &[Event];
    /// The event types seen so far, in first-seen order (TS `seenTypes`
    /// — the JS `Set` iterates in insertion order and trigger
    /// collection depends on it).
    fn seen_types(&self) -> Vec<String>;
    /// The most recent event (TS `latest`).
    fn latest(&self) -> Option<&Event>;
    /// Drop all events (TS `clear`).
    fn clear(&mut self);
}

/// The run-state port — node/workflow `state:` values shared across
/// fires (TS `StateStore`). Values are arbitrary JSON.
pub trait StateStore: Send {
    /// Read a key (TS `get` — `None` for absent).
    fn get(&self, key: &str) -> Option<Value>;
    /// Write a key (TS `set`).
    fn set(&mut self, key: &str, value: Value);
    /// Snapshot the whole store (TS `toRecord`).
    fn to_record(&self) -> Map<String, Value>;
    /// Drop all keys (TS `clear`).
    fn clear(&mut self);
}

/// The dead-letter port — events that exhausted retries land here
/// instead of the normal log (TS `DeadLetterQueue`, ADR-0002).
pub trait DeadLetterQueue: Send {
    /// Record a dead-lettered event with its reason (TS `enqueue`).
    fn enqueue(&mut self, event: &Event, reason: &str);
    /// Everything enqueued so far (TS `all`).
    fn all(&self) -> Vec<Event>;
}

/// In-memory [`EventStore`] + [`StateStore`] — the `ac-store-memory`
/// equivalent; the default for CLI runs and tests.
#[derive(Default)]
pub struct MemoryStore {
    events: Vec<Event>,
    by_type: HashMap<String, Vec<Event>>,
    seen: Vec<String>,
    state: Map<String, Value>,
}

impl EventStore for MemoryStore {
    fn append(&mut self, event: &Event) {
        self.events.push(event.clone());
        self.by_type
            .entry(event.kind.clone())
            .or_default()
            .push(event.clone());
        if !self.seen.contains(&event.kind) {
            self.seen.push(event.kind.clone());
        }
    }

    fn all(&self) -> &[Event] {
        &self.events
    }

    fn by_type(&self, kind: &str) -> &[Event] {
        self.by_type.get(kind).map_or(&[], Vec::as_slice)
    }

    fn seen_types(&self) -> Vec<String> {
        self.seen.clone()
    }

    fn latest(&self) -> Option<&Event> {
        self.events.last()
    }

    fn clear(&mut self) {
        self.events.clear();
        self.by_type.clear();
        self.seen.clear();
        self.state.clear();
    }
}

impl StateStore for MemoryStore {
    fn get(&self, key: &str) -> Option<Value> {
        self.state.get(key).cloned()
    }

    fn set(&mut self, key: &str, value: Value) {
        self.state.insert(key.to_string(), value);
    }

    fn to_record(&self) -> Map<String, Value> {
        self.state.clone()
    }

    fn clear(&mut self) {
        self.state.clear();
    }
}

/// In-memory [`DeadLetterQueue`].
#[derive(Default)]
pub struct MemoryDlq {
    events: Vec<Event>,
}

impl DeadLetterQueue for MemoryDlq {
    fn enqueue(&mut self, event: &Event, _reason: &str) {
        self.events.push(event.clone());
    }

    fn all(&self) -> Vec<Event> {
        self.events.clone()
    }
}

/// A [`DeadLetterQueue`] that drops everything — the TS
/// `noOpDeadLetterQueue` the default runner wires in.
pub struct NoopDlq;

impl DeadLetterQueue for NoopDlq {
    fn enqueue(&mut self, _event: &Event, _reason: &str) {}

    fn all(&self) -> Vec<Event> {
        Vec::new()
    }
}
