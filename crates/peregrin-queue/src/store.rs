//! The [`QueueStore`] port — the durable half of the scheduler.
//! Where [`crate::Queue`] is pure in-memory logic, a `QueueStore`
//! persists the queue's state across process restarts: the pending
//! submissions (per supersede key, so concurrent writers don't
//! clobber each other) plus the in-flight marker.
//!
//! Semantics mirror bupha's proven `PersistedQueue{pending, running}`
//! lesson: on boot the `running` job re-queues at the *front*
//! (at-least-once — an interrupted run gets another shot) and pending
//! entries restore in drain order. [`crate::Queue::restore`] does the
//! re-validation and re-enqueueing; the store only moves bytes.
//!
//! [`MemoryQueueStore`] is the in-memory impl — tests and embedders
//! that want the port without a file. `peregrin-queue-sqlite` ships
//! the production [`SqliteQueueStore`](https://docs.rs) impl (per-key
//! rows in a sqlite file, same pattern as hypha-outbox).

use std::collections::BTreeMap;
use std::fmt;
use std::sync::Mutex;

use crate::{Job, QueueSnapshot};

/// A [`QueueStore`] failure — I/O from the backing medium or a
/// store-level error string (the impl owns the detail).
#[derive(Debug)]
pub enum QueueStoreError {
    /// Filesystem-level failure (open, read, write).
    Io(std::io::Error),
    /// Store-level failure — bad schema, corrupt document, backend
    /// error. The message is the impl's rendering.
    Store(String),
}

impl fmt::Display for QueueStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "queue store I/O: {e}"),
            Self::Store(msg) => write!(f, "queue store: {msg}"),
        }
    }
}

impl std::error::Error for QueueStoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Store(_) => None,
        }
    }
}

impl From<std::io::Error> for QueueStoreError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

/// The durable queue port. All methods are called from the single
/// supervisor thread (submit/drain/finish are sequential), but the
/// trait requires `Send + Sync` so the store can live behind an
/// `Arc` shared with CLI/setup code.
///
/// Write granularity is *per supersede key*: [`Self::persist_key`]
/// replaces one key's pending set and nothing else, so two queues —
/// or two processes — persisting different keys never clobber each
/// other (bupha's `persist_key` semantics). [`Self::set_running`]
/// owns the single in-flight marker.
pub trait QueueStore: Send + Sync {
    /// Load the persisted snapshot — pending jobs merged across all
    /// keys in drain order (`seq` ascending) plus the in-flight
    /// marker. Corrupt *entries* may be skipped by the impl (they're
    /// re-validated again by [`crate::Queue::restore`]); a failure to
    /// read the store itself is an error.
    ///
    /// # Errors
    /// [`QueueStoreError`] when the backing store can't be read.
    fn load(&self) -> Result<QueueSnapshot, QueueStoreError>;

    /// Replace the persisted pending set for `key` — `jobs` in drain
    /// order; an empty slice clears the key. Jobs for other keys are
    /// untouched.
    ///
    /// # Errors
    /// [`QueueStoreError`] when the write fails.
    fn persist_key(&self, key: &str, jobs: &[Job]) -> Result<(), QueueStoreError>;

    /// Record the in-flight job, or clear the marker with `None`
    /// (the active run ended).
    ///
    /// # Errors
    /// [`QueueStoreError`] when the write fails.
    fn set_running(&self, job: Option<&Job>) -> Result<(), QueueStoreError>;
}

/// The in-memory [`QueueStore`] — same semantics, no durability.
/// For tests and embedders that want the port shape without a
/// backing file; a restart obviously loses everything.
#[derive(Default)]
pub struct MemoryQueueStore {
    state: Mutex<MemoryState>,
}

#[derive(Default)]
struct MemoryState {
    /// Pending jobs grouped by supersede key — each vec in drain
    /// order.
    pending: BTreeMap<String, Vec<Job>>,
    running: Option<Job>,
}

impl MemoryQueueStore {
    /// An empty store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A store pre-seeded with `snapshot` — pending jobs grouped by
    /// their `key` field, as if a previous process had persisted
    /// them. Tests use this to simulate "state left by a dead run".
    #[must_use]
    pub fn with_snapshot(snapshot: QueueSnapshot) -> Self {
        let mut pending: BTreeMap<String, Vec<Job>> = BTreeMap::new();
        for job in snapshot.pending {
            pending.entry(job.key.clone()).or_default().push(job);
        }
        Self {
            state: Mutex::new(MemoryState {
                pending,
                running: snapshot.running,
            }),
        }
    }
}

/// Recover from mutex poisoning — the state inside is still
/// consistent (writes are short and can't panic mid-update), so a
/// panicked holder shouldn't take the store down with it.
fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

impl QueueStore for MemoryQueueStore {
    fn load(&self) -> Result<QueueSnapshot, QueueStoreError> {
        let state = lock(&self.state);
        // Merge the per-key vecs and re-sort by `seq` — the global
        // drain order the per-key grouping doesn't preserve.
        let mut pending: Vec<Job> = state.pending.values().flatten().cloned().collect();
        pending.sort_by_key(|job| job.seq);
        Ok(QueueSnapshot {
            pending,
            running: state.running.clone(),
        })
    }

    fn persist_key(&self, key: &str, jobs: &[Job]) -> Result<(), QueueStoreError> {
        let mut state = lock(&self.state);
        if jobs.is_empty() {
            state.pending.remove(key);
        } else {
            state.pending.insert(key.to_string(), jobs.to_vec());
        }
        Ok(())
    }

    fn set_running(&self, job: Option<&Job>) -> Result<(), QueueStoreError> {
        lock(&self.state).running = job.cloned();
        Ok(())
    }
}
