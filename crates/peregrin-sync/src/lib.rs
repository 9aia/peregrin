//! peregrin-sync — the hub-side sync layer (entity plane, ADR-0091).
//!
//! Keeps a local SQLite projection ([`ProjectionStore`]) of every paired
//! node's entity index: `index_doc` rows only — opaque integration
//! docs — never detail payloads (those stay node-fetched). The
//! [`SyncEngine`] runs one supervised loop per node: connect →
//! `list_entities` → drain the durable [`Outbox`] → tail `/api/events`
//! for index diffs → reconnect with capped exponential backoff + jitter.
//!
//! Writes while a node is down go through [`SyncHandle::submit_op`],
//! which posts directly when the node is up and enqueues into
//! `peregrin-outbox` otherwise.

pub mod client;
pub mod engine;
pub mod kind;

use std::path::Path;
use std::sync::{Arc, Mutex};

use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use client::{ClientConfig, EventStream, FeedEvent, NodeClient, NodeError};
pub use engine::{NodeTarget, SubmitOutcome, SyncEngine, SyncHandle, SyncOptions};
pub use kind::{DiffTombstone, IndexColumns, KindPolicies, KindPolicy, Notice, NoticeKind};
pub use peregrin_outbox::{OpKind, Outbox, OutboxEntry, Status as OutboxStatus};

/// Failures of the sync layer — store, outbox, and node calls share one
/// error type at the engine boundary.
#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    /// Projection-store failure.
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    /// Outbox failure.
    #[error("outbox: {0}")]
    Outbox(#[from] peregrin_outbox::OutboxError),
    /// A node call failed (transport, HTTP status, malformed body).
    #[error("node: {0}")]
    Node(#[from] NodeError),
    /// JSON (de)serialization failure.
    #[error("serde: {0}")]
    Serde(#[from] serde_json::Error),
    /// The node id isn't registered with the engine.
    #[error("unknown node {0}")]
    UnknownNode(String),
    /// The engine is already shut down (command channel closed).
    #[error("sync engine is shut down")]
    Shutdown,
}

/// A registered peer node — the identity the hub syncs and writes to.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeRef {
    /// Stable node id (`/api/node`'s `id`, or a hub-assigned label-safe id).
    pub id: String,
    /// Base URL, e.g. `http://192.168.1.10:18787` (no trailing slash needed).
    pub url: String,
    /// Display name in the hub UI.
    pub label: String,
}

/// Last known reachability of a node — written to `nodes.status`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NodeStatus {
    /// Never contacted since registration.
    #[default]
    Unknown,
    /// A request (list/events) succeeded recently.
    Up,
    /// The last connection attempt failed or the SSE stream dropped.
    Down,
}

impl NodeStatus {
    /// The wire/storage spelling: `"unknown"`, `"up"`, `"down"`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Up => "up",
            Self::Down => "down",
        }
    }

    fn parse(s: &str) -> Self {
        match s {
            "up" => Self::Up,
            "down" => Self::Down,
            _ => Self::Unknown,
        }
    }
}

/// A `nodes` row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodeRow {
    pub id: String,
    pub url: String,
    pub label: String,
    /// RFC 3339 — last time a request to the node succeeded.
    pub last_seen_at: Option<String>,
    pub status: NodeStatus,
}

/// An `entities` row — one node's index doc for an entity. `raw` keeps
/// the whole wire object; the projection columnizes only the fields it
/// needs to merge/route on (`kind`, `locked`, `live`, `updatedAt`) —
/// the doc's *shape* is the integration's, never the substrate's.
#[derive(Clone, Debug, PartialEq)]
pub struct IndexedEntity {
    pub node_id: String,
    pub entity_id: String,
    /// Entity kind tag (`sepia-session`, `campaign`, …).
    pub kind: String,
    /// RFC 3339 `updatedAt` from the node.
    pub updated_at: String,
    pub locked: bool,
    /// A live driver/agent is attached (`live` on the wire).
    pub live: bool,
    /// The full entity doc `Value` the node sent (the `indexDoc`).
    pub raw: Value,
}

/// What [`ProjectionStore::apply_patch`] did with an index diff.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PatchStatus {
    /// Merged into an existing row.
    Applied,
    /// No row for `(node, session)` — the caller should refetch the
    /// full summary.
    Missing,
}

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS nodes (
    id TEXT PRIMARY KEY,
    url TEXT NOT NULL,
    label TEXT NOT NULL,
    last_seen_at TEXT,
    status TEXT NOT NULL DEFAULT 'unknown'
);
CREATE TABLE IF NOT EXISTS entities (
    node_id TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    kind TEXT NOT NULL DEFAULT '',
    updated_at TEXT NOT NULL DEFAULT '',
    locked INTEGER NOT NULL DEFAULT 0,
    live INTEGER NOT NULL DEFAULT 0,
    raw_json TEXT NOT NULL,
    PRIMARY KEY (node_id, entity_id)
);
CREATE INDEX IF NOT EXISTS entities_node ON entities (node_id);
CREATE TABLE IF NOT EXISTS sync_state (
    node_id TEXT PRIMARY KEY,
    cursor TEXT,
    updated_at TEXT NOT NULL
);
";

fn now_rfc3339() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default()
}

/// Pull the column values out of one wire entity doc — unknown/missing
/// fields degrade to defaults rather than failing the row. Docs that
/// carry `indexDoc` (the `{id,kind,indexDoc}` entity wrap) derive
/// their fields from inside it; kind-specific aliases (`busy` →
/// `live`) are the kind policy's [`KindPolicy::columns`] override.
pub(crate) fn columns_of(raw: &Value) -> IndexedEntity {
    let doc = match raw.get("indexDoc") {
        Some(d) if d.is_object() => d,
        _ => raw,
    };
    let get = |k: &str| doc.get(k).and_then(Value::as_str).unwrap_or_default();
    let flag = |k: &str| doc.get(k).and_then(Value::as_bool).unwrap_or(false);
    IndexedEntity {
        node_id: String::new(),
        entity_id: raw
            .get("id")
            .or_else(|| doc.get("id"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        kind: raw
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        updated_at: get("updatedAt").to_string(),
        locked: flag("locked"),
        live: flag("live"),
        raw: raw.clone(),
    }
}

fn write_row(
    conn: &Connection,
    node_id: &str,
    doc: &Value,
    policies: &crate::kind::KindPolicies,
) -> Result<(), SyncError> {
    let mut row = policies.columns_of(doc);
    row.node_id = node_id.to_string();
    conn.execute(
        "INSERT INTO entities
         (node_id, entity_id, kind, updated_at, locked, live, raw_json)
         VALUES (?,?,?,?,?,?,?)
         ON CONFLICT (node_id, entity_id) DO UPDATE SET
            kind=excluded.kind, updated_at=excluded.updated_at,
            locked=excluded.locked, live=excluded.live,
            raw_json=excluded.raw_json",
        params![
            row.node_id,
            row.entity_id,
            row.kind,
            row.updated_at,
            row.locked,
            row.live,
            // `Value`'s `Display` is infallible compact JSON — no `?`
            // arm to leave uncovered.
            row.raw.to_string(),
        ],
    )?;
    Ok(())
}

fn row_to_indexed(row: &rusqlite::Row<'_>) -> rusqlite::Result<IndexedEntity> {
    let raw_json: String = row.get("raw_json")?;
    let raw = serde_json::from_str(&raw_json).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    })?;
    Ok(IndexedEntity {
        node_id: row.get("node_id")?,
        entity_id: row.get("entity_id")?,
        kind: row.get("kind")?,
        updated_at: row.get("updated_at")?,
        locked: row.get::<_, i64>("locked")? != 0,
        live: row.get::<_, i64>("live")? != 0,
        raw,
    })
}

/// The db holds entity index docs the hub caches — keep it owner-only.
/// sqlite creates files at umask defaults; tighten on every open.
#[cfg(unix)]
fn restrict_file_perms(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    if path.exists() {
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
    }
}

#[cfg(not(unix))]
fn restrict_file_perms(_path: &Path) {}

/// The local projection — a materialized view of every paired node's
/// entity index plus per-node liveness and resync cursors. `Clone`
/// shares one connection (same pattern as `peregrin-outbox`).
#[derive(Clone)]
pub struct ProjectionStore {
    conn: Arc<Mutex<Connection>>,
    /// Kind-policy registry — column extraction consults it so a
    /// kind's wire doc can override the generic column rules.
    policies: crate::kind::KindPolicies,
}

impl ProjectionStore {
    /// Open (and create) a projection at `path` — WAL + busy_timeout,
    /// matching `peregrin-outbox`'s durability posture.
    ///
    /// # Errors
    /// On sqlite open/schema failure.
    pub fn open(path: &Path) -> Result<Self, SyncError> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).ok();
        }
        let conn = Connection::open(path)?;
        restrict_file_perms(path);
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "busy_timeout", 5000)?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            policies: crate::kind::KindPolicies::new(),
        })
    }

    /// Attach a kind-policy registry — column extraction consults it
    /// on every write.
    #[must_use]
    pub fn with_kind_policies(mut self, policies: crate::kind::KindPolicies) -> Self {
        self.policies = policies;
        self
    }

    /// In-memory projection for tests.
    ///
    /// # Errors
    /// On schema failure.
    pub fn in_memory() -> Result<Self, SyncError> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            policies: crate::kind::KindPolicies::new(),
        })
    }

    /* ---- nodes ------------------------------------------------------*/

    /// Register (or refresh) a node row. Status is untouched on update
    /// so a re-register doesn't flap liveness.
    ///
    /// # Errors
    /// On sqlite failure.
    pub fn upsert_node(&self, node: &NodeRef) -> Result<(), SyncError> {
        let conn = self
            .conn
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        conn.execute(
            "INSERT INTO nodes (id, url, label, status) VALUES (?,?,?,'unknown')
             ON CONFLICT (id) DO UPDATE SET url=excluded.url, label=excluded.label",
            params![node.id, node.url, node.label],
        )?;
        Ok(())
    }

    /// Drop a node and all of its projected state.
    ///
    /// # Errors
    /// On sqlite failure.
    pub fn remove_node(&self, id: &str) -> Result<(), SyncError> {
        let conn = self
            .conn
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        conn.execute("DELETE FROM entities WHERE node_id=?", params![id])?;
        conn.execute("DELETE FROM sync_state WHERE node_id=?", params![id])?;
        conn.execute("DELETE FROM nodes WHERE id=?", params![id])?;
        Ok(())
    }

    /// # Errors
    /// On sqlite failure.
    pub fn node(&self, id: &str) -> Result<Option<NodeRow>, SyncError> {
        let conn = self
            .conn
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut stmt =
            conn.prepare("SELECT id, url, label, last_seen_at, status FROM nodes WHERE id=?")?;
        let mut rows = stmt.query_map(params![id], node_row)?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    /// Every registered node row.
    ///
    /// # Errors
    /// On sqlite failure.
    pub fn nodes(&self) -> Result<Vec<NodeRow>, SyncError> {
        let conn = self
            .conn
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut stmt =
            conn.prepare("SELECT id, url, label, last_seen_at, status FROM nodes ORDER BY id")?;
        let rows = stmt
            .query_map([], node_row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Record reachability. `Up` also stamps `last_seen_at`.
    ///
    /// # Errors
    /// On sqlite failure.
    pub fn set_node_status(&self, id: &str, status: NodeStatus) -> Result<(), SyncError> {
        let conn = self
            .conn
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if status == NodeStatus::Up {
            conn.execute(
                "UPDATE nodes SET status='up', last_seen_at=? WHERE id=?",
                params![now_rfc3339(), id],
            )?;
        } else {
            conn.execute(
                "UPDATE nodes SET status=? WHERE id=?",
                params![status.as_str(), id],
            )?;
        }
        Ok(())
    }

    /* ---- entities ---------------------------------------------------*/

    /// Upsert one wire entity doc under `node_id`.
    ///
    /// # Errors
    /// On sqlite/serde failure.
    pub fn upsert_entity(&self, node_id: &str, doc: &Value) -> Result<(), SyncError> {
        let conn = self
            .conn
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        write_row(&conn, node_id, doc, &self.policies)
    }

    /// Replace a node's whole index: upsert every listed entity doc,
    /// delete rows the list no longer reports (tombstones), and advance
    /// the `sync_state` cursor to the max `updatedAt` seen.
    ///
    /// # Errors
    /// On sqlite/serde failure.
    pub fn apply_listing(&self, node_id: &str, docs: &[Value]) -> Result<(), SyncError> {
        let conn = self
            .conn
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let tx = conn.unchecked_transaction()?;
        let mut keep = Vec::with_capacity(docs.len());
        let mut max_updated = String::new();
        for doc in docs {
            write_row(&tx, node_id, doc, &self.policies)?;
            if let Some(id) = doc.get("id").and_then(Value::as_str) {
                keep.push(id.to_string());
            }
            if let Some(u) = doc.get("updatedAt").and_then(Value::as_str) {
                if u > max_updated.as_str() {
                    max_updated = u.to_string();
                }
            }
        }
        if keep.is_empty() {
            tx.execute("DELETE FROM entities WHERE node_id=?", params![node_id])?;
        } else {
            // `IN` list — ids come from the node, not the user, but
            // still go through bind params.
            let marks = std::iter::repeat_n("?", keep.len())
                .collect::<Vec<_>>()
                .join(",");
            let sql =
                format!("DELETE FROM entities WHERE node_id=? AND entity_id NOT IN ({marks})");
            let mut bind: Vec<&dyn rusqlite::ToSql> = vec![&node_id];
            bind.extend(keep.iter().map(|id| id as &dyn rusqlite::ToSql));
            tx.execute(&sql, rusqlite::params_from_iter(bind.iter()))?;
        }
        if !max_updated.is_empty() {
            tx.execute(
                "INSERT INTO sync_state (node_id, cursor, updated_at) VALUES (?,?,?)
                 ON CONFLICT (node_id) DO UPDATE SET cursor=excluded.cursor, updated_at=excluded.updated_at",
                params![node_id, max_updated, now_rfc3339()],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Merge a `{patch}` diff into an indexed row's `raw_json` and
    /// re-derive its columns. [`PatchStatus::Missing`] when no row
    /// exists — the caller should fetch the full entity doc.
    ///
    /// # Errors
    /// On sqlite/serde failure.
    pub fn apply_patch(
        &self,
        node_id: &str,
        entity_id: &str,
        patch: &Value,
    ) -> Result<PatchStatus, SyncError> {
        let conn = self
            .conn
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let raw_json: Option<String> = conn
            .query_row(
                "SELECT raw_json FROM entities WHERE node_id=? AND entity_id=?",
                params![node_id, entity_id],
                |r| r.get(0),
            )
            .ok();
        let Some(raw_json) = raw_json else {
            return Ok(PatchStatus::Missing);
        };
        let mut raw: Value = serde_json::from_str(&raw_json)?;
        // Diffs patch the entity's fields — under the interim
        // `indexDoc` wrap that's the inner doc, else the doc itself.
        let target = match raw.get_mut("indexDoc") {
            Some(d) if d.is_object() => d.as_object_mut(),
            _ => raw.as_object_mut(),
        };
        if let (Some(obj), Some(patch_obj)) = (target, patch.as_object()) {
            for (k, v) in patch_obj {
                obj.insert(k.clone(), v.clone());
            }
        }
        write_row(&conn, node_id, &raw, &self.policies)?;
        Ok(PatchStatus::Applied)
    }

    /// Drop every row belonging to a node id that's no longer
    /// registered — a renamed id (`node`→`main`), a node removed from
    /// `PEREGRIN_NODES`, or the single-node shorthand id replaced by a
    /// real one. Their entity rows have no sync loop anymore and
    /// would serve dead entities forever (click → "node not found").
    /// The store is a cache — a still-configured node resyncs itself.
    ///
    /// # Errors
    /// On sqlite failure.
    pub fn purge_unregistered(&self, keep: &[String]) -> Result<(), SyncError> {
        let conn = self
            .conn
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if keep.is_empty() {
            // No nodes configured — dropping everything would also
            // wipe the visible list; only a bug could get here, keep
            // the data and let the caller decide.
            return Ok(());
        }
        let marks = std::iter::repeat_n("?", keep.len())
            .collect::<Vec<_>>()
            .join(",");
        let bind: Vec<&dyn rusqlite::ToSql> =
            keep.iter().map(|id| id as &dyn rusqlite::ToSql).collect();
        let tx = conn.unchecked_transaction()?;
        for table in ["entities", "sync_state"] {
            tx.execute(
                &format!("DELETE FROM {table} WHERE node_id NOT IN ({marks})"),
                rusqlite::params_from_iter(bind.iter()),
            )?;
        }
        tx.execute(
            &format!("DELETE FROM nodes WHERE id NOT IN ({marks})"),
            rusqlite::params_from_iter(bind.iter()),
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Drop one projected entity (tombstone applied).
    ///
    /// # Errors
    /// On sqlite failure.
    pub fn delete_entity(&self, node_id: &str, entity_id: &str) -> Result<(), SyncError> {
        let conn = self
            .conn
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        conn.execute(
            "DELETE FROM entities WHERE node_id=? AND entity_id=?",
            params![node_id, entity_id],
        )?;
        Ok(())
    }

    /// One projected entity row.
    ///
    /// # Errors
    /// On sqlite/serde failure.
    pub fn entity(
        &self,
        node_id: &str,
        entity_id: &str,
    ) -> Result<Option<IndexedEntity>, SyncError> {
        let conn = self
            .conn
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut stmt = conn.prepare("SELECT * FROM entities WHERE node_id=? AND entity_id=?")?;
        let mut rows = stmt.query_map(params![node_id, entity_id], row_to_indexed)?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    /// Every projected entity — `node_id` scopes to one node, `None`
    /// lists all nodes (the hub's merged entity list).
    ///
    /// # Errors
    /// On sqlite/serde failure.
    pub fn entities(&self, node_id: Option<&str>) -> Result<Vec<IndexedEntity>, SyncError> {
        let conn = self
            .conn
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(node_id) = node_id {
            let mut stmt =
                conn.prepare("SELECT * FROM entities WHERE node_id=? ORDER BY updated_at DESC")?;
            let rows = stmt
                .query_map(params![node_id], row_to_indexed)?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(rows)
        } else {
            let mut stmt = conn.prepare("SELECT * FROM entities ORDER BY updated_at DESC")?;
            let rows = stmt
                .query_map([], row_to_indexed)?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(rows)
        }
    }

    /// The resync cursor for a node (max `updatedAt` from the last full
    /// listing — informational; today's resync is always a full list).
    ///
    /// # Errors
    /// On sqlite failure.
    pub fn cursor(&self, node_id: &str) -> Result<Option<String>, SyncError> {
        let conn = self
            .conn
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let cursor = conn
            .query_row(
                "SELECT cursor FROM sync_state WHERE node_id=?",
                params![node_id],
                |r| r.get::<_, Option<String>>(0),
            )
            .ok()
            .flatten();
        Ok(cursor)
    }

    /// Record a resync cursor explicitly.
    ///
    /// # Errors
    /// On sqlite failure.
    pub fn set_cursor(&self, node_id: &str, cursor: &str) -> Result<(), SyncError> {
        let conn = self
            .conn
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        conn.execute(
            "INSERT INTO sync_state (node_id, cursor, updated_at) VALUES (?,?,?)
             ON CONFLICT (node_id) DO UPDATE SET cursor=excluded.cursor, updated_at=excluded.updated_at",
            params![node_id, cursor, now_rfc3339()],
        )?;
        Ok(())
    }
}

fn node_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<NodeRow> {
    Ok(NodeRow {
        id: row.get("id")?,
        url: row.get("url")?,
        label: row.get("label")?,
        last_seen_at: row.get("last_seen_at")?,
        status: NodeStatus::parse(&row.get::<_, String>("status")?),
    })
}

/// A write destined for a node — the input to [`SyncHandle::submit_op`].
/// When the node is down it becomes an [`OutboxEntry`]; `policy` decides
/// retry-vs-dead-letter on replay failure.
#[derive(Clone, Debug)]
pub struct NodeWrite {
    /// Target node (must be registered).
    pub node_id: String,
    /// Entity the write applies to.
    pub entity_id: String,
    /// Entity kind for `?kind=` disambiguation — the projection's `kind`
    /// column is a good source.
    pub entity_kind: Option<String>,
    /// Outbox op name: `<kind>.<verb>` namespaced (`sepia.prompt`,
    /// `entity.delete`, …).
    pub op: String,
    /// [`OpKind::Metadata`] retries; [`OpKind::Turn`] dead-letters on a
    /// failed replay.
    pub policy: OpKind,
    /// The JSON body (empty object for `entity.delete` etc.).
    pub payload: Value,
    /// Client-supplied dedupe key — re-submitting the same key returns
    /// the same queued entry instead of a duplicate write.
    pub idempotency_key: String,
    /// Seconds until the queued write expires (dead-letters); `None`
    /// never expires.
    pub ttl_seconds: Option<i64>,
}
