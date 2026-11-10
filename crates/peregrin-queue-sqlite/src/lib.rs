//! `peregrin-queue-sqlite` — the production [`QueueStore`] impl: a
//! single sqlite file holding the durable supersede queue behind
//! `peregrin campaign run --queue-store`.
//!
//! Layout (the same pattern as hypha-outbox's sqlite stores):
//!
//! - `queue_pending(key, seq, job)` — one row per pending submission,
//!   `job` the serde-JSON [`Job`]. Primary key `(key, seq)`; writes
//!   are per-key `DELETE`+`INSERT` inside a transaction, so peers
//!   persisting different keys never clobber each other.
//! - `queue_running(slot, job)` — a singleton row (slot always 0)
//!   holding the in-flight job, if any.
//!
//! `load` merges the per-key rows back into drain order (`seq`
//! ascending) and hands the snapshot to [`Queue::restore`], which
//! re-validates every entry against the campaign's `supersede:`
//! policy — a corrupt row is skipped here with a warning and dropped
//! there if its key doesn't resolve.

use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use peregrin_queue::{Job, QueueSnapshot, QueueStore, QueueStoreError};
use rusqlite::{Connection, OptionalExtension, params};

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS queue_pending (
    key TEXT NOT NULL,
    seq INTEGER NOT NULL,
    job TEXT NOT NULL,
    PRIMARY KEY (key, seq)
);
CREATE TABLE IF NOT EXISTS queue_running (
    slot INTEGER PRIMARY KEY CHECK (slot = 0),
    job TEXT NOT NULL
);
";

/// A [`QueueStore`] over a sqlite file. `Send + Sync` via a mutexed
/// connection — all queue mutations are sequential anyway (the
/// supervisor thread), the lock just satisfies the port.
pub struct SqliteQueueStore {
    conn: Mutex<Connection>,
}

/// Recover from mutex poisoning — a panicked writer can't leave the
/// schema half-applied (every mutation is one transaction), so the
/// connection stays usable.
fn lock(mutex: &Mutex<Connection>) -> MutexGuard<'_, Connection> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn sql_err(e: &rusqlite::Error) -> QueueStoreError {
    QueueStoreError::Store(e.to_string())
}

fn seq_to_i64(seq: u64) -> Result<i64, QueueStoreError> {
    i64::try_from(seq).map_err(|_| QueueStoreError::Store(format!("job seq {seq} overflows i64")))
}

fn seq_from_i64(seq: i64) -> Result<u64, QueueStoreError> {
    u64::try_from(seq).map_err(|_| QueueStoreError::Store(format!("negative job seq {seq}")))
}

impl SqliteQueueStore {
    /// Open (creating if needed) the queue store at `path`, applying
    /// the schema. Parent directories are created.
    ///
    /// # Errors
    /// [`QueueStoreError`] when the file can't be opened or the
    /// schema can't be applied.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, QueueStoreError> {
        let path = path.as_ref();
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path).map_err(|e| sql_err(&e))?;
        conn.execute_batch(SCHEMA).map_err(|e| sql_err(&e))?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }
}

impl QueueStore for SqliteQueueStore {
    fn load(&self) -> Result<QueueSnapshot, QueueStoreError> {
        let conn = lock(&self.conn);
        let mut stmt = conn
            .prepare("SELECT seq, job FROM queue_pending ORDER BY seq")
            .map_err(|e| sql_err(&e))?;
        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|e| sql_err(&e))?;
        let mut pending = Vec::new();
        for row in rows {
            let (seq, job_json) = row.map_err(|e| sql_err(&e))?;
            match serde_json::from_str::<Job>(&job_json) {
                Ok(mut job) => {
                    // Trust the row's seq column over the blob's —
                    // they're written together, but the column is the
                    // ordering key.
                    job.seq = seq_from_i64(seq)?;
                    pending.push(job);
                }
                Err(e) => {
                    tracing::warn!("queue store: skipping corrupt pending row: {e}");
                }
            }
        }
        drop(stmt);
        let running_json: Option<String> = conn
            .query_row("SELECT job FROM queue_running WHERE slot = 0", [], |row| {
                row.get(0)
            })
            .optional()
            .map_err(|e| sql_err(&e))?;
        let running = running_json.and_then(|json| match serde_json::from_str::<Job>(&json) {
            Ok(job) => Some(job),
            Err(e) => {
                tracing::warn!("queue store: ignoring corrupt running marker: {e}");
                None
            }
        });
        Ok(QueueSnapshot { pending, running })
    }

    fn persist_key(&self, key: &str, jobs: &[Job]) -> Result<(), QueueStoreError> {
        let mut conn = lock(&self.conn);
        let tx = conn.transaction().map_err(|e| sql_err(&e))?;
        tx.execute("DELETE FROM queue_pending WHERE key = ?1", params![key])
            .map_err(|e| sql_err(&e))?;
        for job in jobs {
            let json = serde_json::to_string(job)
                .map_err(|e| QueueStoreError::Store(format!("job serialize: {e}")))?;
            tx.execute(
                "INSERT INTO queue_pending (key, seq, job) VALUES (?1, ?2, ?3)",
                params![job.key, seq_to_i64(job.seq)?, json],
            )
            .map_err(|e| sql_err(&e))?;
        }
        tx.commit().map_err(|e| sql_err(&e))
    }

    fn set_running(&self, job: Option<&Job>) -> Result<(), QueueStoreError> {
        let mut conn = lock(&self.conn);
        let tx = conn.transaction().map_err(|e| sql_err(&e))?;
        tx.execute("DELETE FROM queue_running", [])
            .map_err(|e| sql_err(&e))?;
        if let Some(job) = job {
            let json = serde_json::to_string(job)
                .map_err(|e| QueueStoreError::Store(format!("job serialize: {e}")))?;
            tx.execute(
                "INSERT INTO queue_running (slot, job) VALUES (0, ?1)",
                params![json],
            )
            .map_err(|e| sql_err(&e))?;
        }
        tx.commit().map_err(|e| sql_err(&e))
    }
}
