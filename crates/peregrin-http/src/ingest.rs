//! `POST /api/events/inject` — the node's authenticated event-ingest
//! route backend. External producers (the bupha CI worker's
//! `ci.job.completed` emit, forge check runners, ops tooling) POST a
//! `{type, payload}` body; the node fans it out to the campaign
//! engine's live runs — the `kind="campaign"` entity plane's per-run
//! [`Injector`](peregrin_engine::Injector) channels — through the
//! [`EventDeliver`] sink the composition root wires
//! ([`AppState::event_ingest`](crate::AppState)).
//!
//! Idempotency: the body's `idempotencyKey` (snake_case alias
//! accepted) feeds a bounded dedup ring — the same `Seen` shape the
//! `github-webhook` driver applies to `X-GitHub-Delivery`. A replayed
//! key answers `deduplicated: true` without re-delivering; keyless
//! bodies are never deduped. The key is *claimed* before the sink
//! runs and rolled back on sink failure, so a retried emit after a
//! 500 still lands.

use std::collections::{HashSet, VecDeque};
use std::sync::{Arc, Mutex, PoisonError};

use axum::http::StatusCode;
use serde_json::Value;

/// Bound on the dedup ring — same trade-off as the webhook driver's
/// `Seen`: replays older than this many keys re-deliver.
const SEEN_CAP: usize = 1024;

/// One validated `POST /api/events/inject` body — everything the
/// sink needs; the route has already enforced shape, auth (the shared
/// bearer gate), and the dedup check.
#[derive(Debug)]
pub struct IngestDelivery {
    /// Event kind — the body's `type` (`"ci.job.completed"`, a forge
    /// `check_run`, …). Also accepts the `Event` wire's `kind` alias.
    pub kind: String,
    /// Event payload verbatim — any JSON value; `{}` when absent.
    pub payload: Value,
    /// `Event.source` — `"api"` when the body omits `source`.
    pub source: String,
    /// Optional `kind="campaign"` entity id — scopes the fan-out to
    /// that one entity; absent → every running campaign entity.
    pub campaign: Option<String>,
    /// The body's idempotency key — `None`/empty deliveries are never
    /// deduped.
    pub idempotency_key: Option<String>,
}

/// Sink failure — `status` + message; the route maps it verbatim so
/// the plane can answer 404 (unknown scoped campaign) vs 500
/// (internal) honestly.
#[derive(Debug)]
pub struct DeliverError {
    /// The HTTP status the route answers with.
    pub status: StatusCode,
    /// The `{error}` body message.
    pub message: String,
}

impl DeliverError {
    /// A `500`-class sink failure.
    #[must_use]
    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: message.into(),
        }
    }
}

/// The fan-out sink — returns how many live runs accepted the event.
/// The node wires it to the campaign plane's running-entity injectors;
/// per-run channel drops count as not-delivered, never `Err`.
pub type EventDeliver = Arc<dyn Fn(&IngestDelivery) -> Result<usize, DeliverError> + Send + Sync>;

/// What [`EventIngest::deliver`] produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IngestOutcome {
    /// Fresh (or keyless) — the count of live runs that accepted the
    /// event (`0` = accepted, but no campaign entity was running).
    Delivered(usize),
    /// The idempotency key was already seen — nothing re-delivered.
    Duplicate,
}

/// Idempotency dedup ring — `insert` claims a key (`false` = replay),
/// evicting oldest-first past [`SEEN_CAP`].
struct Seen {
    ids: HashSet<String>,
    order: VecDeque<String>,
}

impl Seen {
    fn new() -> Self {
        Self {
            ids: HashSet::new(),
            order: VecDeque::new(),
        }
    }

    /// `true` = fresh key, `false` = replay.
    fn insert(&mut self, key: &str) -> bool {
        if !self.ids.insert(key.to_string()) {
            return false;
        }
        self.order.push_back(key.to_string());
        while self.order.len() > SEEN_CAP {
            if let Some(old) = self.order.pop_front() {
                self.ids.remove(&old);
            }
        }
        true
    }

    fn remove(&mut self, key: &str) {
        if self.ids.remove(key) {
            // Order entries are append-only; a stale duplicate in the
            // deque just wastes one slot until eviction — removing by
            // position keeps `order.len()` honest for the cap.
            if let Some(pos) = self.order.iter().position(|k| k == key) {
                self.order.remove(pos);
            }
        }
    }
}

/// `POST /api/events/inject`'s backend — the deliver sink plus the
/// idempotency dedup ring. `Clone` shares both (AppState clones per
/// request).
#[derive(Clone)]
pub struct EventIngest {
    deliver: EventDeliver,
    seen: Arc<Mutex<Seen>>,
}

impl EventIngest {
    /// A sink + empty dedup ring — the composition root's wiring
    /// point (`AppState::with_event_ingest`).
    #[must_use]
    pub fn new(deliver: EventDeliver) -> Self {
        Self {
            deliver,
            seen: Arc::new(Mutex::new(Seen::new())),
        }
    }

    /// Dedup-check then fan out. A present, non-empty
    /// `idempotency_key` is claimed up front; a sink `Err` rolls the
    /// claim back so the client's retry isn't masked as a duplicate.
    ///
    /// # Errors
    /// The sink's [`DeliverError`] — the route maps `status`/`message`
    /// verbatim.
    pub fn deliver(&self, delivery: &IngestDelivery) -> Result<IngestOutcome, DeliverError> {
        let key = delivery
            .idempotency_key
            .as_deref()
            .filter(|key| !key.is_empty());
        let Some(key) = key else {
            return (self.deliver)(delivery).map(IngestOutcome::Delivered);
        };
        {
            let mut seen = self.seen.lock().unwrap_or_else(PoisonError::into_inner);
            if !seen.insert(key) {
                return Ok(IngestOutcome::Duplicate);
            }
        }
        match (self.deliver)(delivery) {
            Ok(delivered) => Ok(IngestOutcome::Delivered(delivered)),
            Err(error) => {
                self.seen
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .remove(key);
                Err(error)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    fn delivery(kind: &str, key: Option<&str>) -> IngestDelivery {
        IngestDelivery {
            kind: kind.to_string(),
            payload: serde_json::json!({"n": 1}),
            source: "test".to_string(),
            campaign: None,
            idempotency_key: key.map(str::to_string),
        }
    }

    /// A sink counting calls and answering `delivered` (or `Err`ing
    /// when `fail` is set — toggled per test via the atomic).
    fn counting_sink(calls: &Arc<AtomicUsize>, fail: Arc<AtomicUsize>) -> EventDeliver {
        let calls = Arc::clone(calls);
        Arc::new(move |_| {
            calls.fetch_add(1, Ordering::SeqCst);
            if fail.load(Ordering::SeqCst) == 1 {
                Err(DeliverError::internal("sink down"))
            } else {
                Ok(3)
            }
        })
    }

    #[test]
    fn keyless_and_empty_key_deliver_every_time() {
        let calls = Arc::new(AtomicUsize::new(0));
        let ingest = EventIngest::new(counting_sink(&calls, Arc::new(AtomicUsize::new(0))));
        assert!(matches!(
            ingest.deliver(&delivery("k", None)),
            Ok(IngestOutcome::Delivered(3))
        ));
        assert!(matches!(
            ingest.deliver(&delivery("k", Some(""))),
            Ok(IngestOutcome::Delivered(3))
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn idempotency_key_dedups_replays() {
        let calls = Arc::new(AtomicUsize::new(0));
        let ingest = EventIngest::new(counting_sink(&calls, Arc::new(AtomicUsize::new(0))));
        let d = delivery("ci.job.completed", Some("job-1:success"));
        assert!(matches!(
            ingest.deliver(&d),
            Ok(IngestOutcome::Delivered(3))
        ));
        assert!(matches!(ingest.deliver(&d), Ok(IngestOutcome::Duplicate)));
        assert!(matches!(ingest.deliver(&d), Ok(IngestOutcome::Duplicate)));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        // A different key is fresh.
        assert!(matches!(
            ingest.deliver(&delivery("ci.job.completed", Some("job-2:success"))),
            Ok(IngestOutcome::Delivered(3))
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn sink_error_rolls_back_the_claim() {
        let calls = Arc::new(AtomicUsize::new(0));
        let fail = Arc::new(AtomicUsize::new(1));
        let ingest = EventIngest::new(counting_sink(&calls, Arc::clone(&fail)));
        let d = delivery("k", Some("retry-me"));
        let err = ingest.deliver(&d).unwrap_err();
        assert_eq!(err.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(err.message, "sink down");
        // The claim rolled back — the client's retry isn't masked.
        fail.store(0, Ordering::SeqCst);
        assert!(matches!(
            ingest.deliver(&d),
            Ok(IngestOutcome::Delivered(3))
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        // ...and now the key IS deduped.
        assert!(matches!(ingest.deliver(&d), Ok(IngestOutcome::Duplicate)));
    }
}
