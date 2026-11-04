//! peregrin-store-json — the durable impl of the [`EventStore`] port:
//! an append-only JSON-lines file, one serialized [`Event`] per line
//! (port of the v1 `ac-store-json` `JsonStore`).
//!
//! Semantics, matching the TS:
//! - [`JsonStore::open`] loads any existing log and rebuilds the
//!   in-memory indexes — publish order (`all`), per-type order
//!   (`by_type`), and first-seen order (`seen_types`). First-seen
//!   order is a hard contract: the wildcard trigger collection in
//!   `peregrin-bus` iterates `seen_types` and depends on it.
//! - `append` indexes in memory, then writes one JSON line. The
//!   `File` is unbuffered, so each append is a `write` syscall on an
//!   `O_APPEND` descriptor — an event survives a *process* crash once
//!   `append` returns. Like the TS `appendFileSync` there is no
//!   fsync, so an OS crash can still lose recent lines.
//! - `clear` drops the indexes and removes the file; the next
//!   `append` recreates it — truncating the durable record.
//! - Log files are private: created `0o600` on unix, and an existing
//!   file's permissions are tightened to `0o600` on `open`.
//! - A malformed (non-JSON) line fails `open` — the TS `JSON.parse`
//!   throw inside `make`, surfaced as [`JsonStoreError::Parse`]. A
//!   line that parses to a non-object (or an object with wrong-typed
//!   fields) decodes through the TS `toEvent` coercion into a
//!   default-ish event rather than failing.
//!
//! Concurrency, unchanged from the TS (which holds no lock): a log
//! file is **single-writer**. `O_APPEND` keeps a second writer's
//! lines atomic, but each `JsonStore` only sees its own appends plus
//! what it loaded at `open` — concurrent instances never observe each
//! other.
//!
//! The `EventStore` port is infallible, so a durability failure on
//! `append`/`clear` can't be returned — it is logged
//! (`tracing::error!`), recorded on [`JsonStore::write_error`], and
//! the in-memory indexes still update (the TS `append` indexes before
//! it writes, so a failed `appendFileSync` likewise leaves the event
//! indexed).

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use peregrin_core::Event;
use peregrin_store::EventStore;
use serde_json::Value;

/// `JsonStore` failures — file I/O and log-line parse errors (the TS
/// `JsonStoreError { reason }`).
#[derive(Debug, thiserror::Error)]
pub enum JsonStoreError {
    /// Filesystem failure — read/chmod at `open`, mkdir/open/write on
    /// `append`, unlink on `clear`.
    #[error("store file: {0}")]
    Io(#[from] std::io::Error),
    /// A non-empty log line didn't parse as JSON — the TS
    /// `JSON.parse` throw inside `make`. `line` is 1-based.
    #[error("store log line {line}: {source}")]
    Parse {
        /// The 1-based line number that failed to parse.
        line: usize,
        /// The `serde_json` parse failure.
        source: serde_json::Error,
    },
    /// An event failed to serialize — unreachable for well-formed
    /// `Event`s; kept so the `serde_json` error channel is explicit.
    #[error("event serialize: {0}")]
    Serialize(serde_json::Error),
}

/// The JSONL [`EventStore`] — the `ac-store-json` `JsonStore` port.
///
/// Construct with [`JsonStore::open`]. `Drop` closes the descriptor —
/// the TS `close()` is a no-op.
pub struct JsonStore {
    path: PathBuf,
    /// The lazily-opened `O_APPEND` descriptor — `None` until the
    /// first `append`, and after `clear` unlinks the file.
    file: Option<File>,
    events: Vec<Event>,
    by_type: HashMap<String, Vec<Event>>,
    /// First-seen event types — a `Vec`, not a `Set`, so `seen_types`
    /// iterates in insertion order like the JS `Set`.
    seen: Vec<String>,
    /// The last `append`/`clear` durability failure — the port is
    /// infallible, so write failures are recorded here instead.
    write_error: Option<JsonStoreError>,
}

impl JsonStore {
    /// `JsonStore.make(filePath)` — open the log at `path`, loading
    /// any existing events. A missing file is an empty store (the log
    /// is created lazily on the first `append`); a present file is
    /// read back and, on unix, tightened to `0o600`.
    ///
    /// # Errors
    /// [`JsonStoreError::Io`] when the file can't be read, and
    /// [`JsonStoreError::Parse`] on the first non-empty line that
    /// isn't JSON.
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, JsonStoreError> {
        let mut store = Self {
            path: path.into(),
            file: None,
            events: Vec::new(),
            by_type: HashMap::new(),
            seen: Vec::new(),
            write_error: None,
        };
        match std::fs::read_to_string(&store.path) {
            Ok(content) => {
                for (index, line) in content.lines().enumerate() {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    let raw: Value =
                        serde_json::from_str(trimmed).map_err(|source| JsonStoreError::Parse {
                            line: index + 1,
                            source,
                        })?;
                    store.index(&to_event(&raw));
                }
                tighten_permissions(&store.path)?;
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(JsonStoreError::Io(e)),
        }
        Ok(store)
    }

    /// The log's path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The last `append`/`clear` durability failure, if any. The
    /// `EventStore` port can't return errors, so a write failure is
    /// logged and recorded here; the next successful write clears it.
    #[must_use]
    pub fn write_error(&self) -> Option<&JsonStoreError> {
        self.write_error.as_ref()
    }

    /// The TS `index` — update every in-memory index in append order.
    fn index(&mut self, event: &Event) {
        self.events.push(event.clone());
        self.by_type
            .entry(event.kind.clone())
            .or_default()
            .push(event.clone());
        if !self.seen.contains(&event.kind) {
            self.seen.push(event.kind.clone());
        }
    }

    /// Serialize + append one line; the `File` is unbuffered so
    /// `write_all` *is* the flush. Lazily creates the parent dirs and
    /// the file itself (`0o600` on unix), like the TS `append`.
    fn write(&mut self, event: &Event) -> Result<(), JsonStoreError> {
        // `parent()` of a bare filename is `""` — `create_dir_all("")`
        // is a no-op Ok, so no empty-path guard is needed.
        if let Some(dir) = self.path.parent()
            && !dir.exists()
        {
            std::fs::create_dir_all(dir)?;
        }
        if self.file.is_none() {
            let mut options = OpenOptions::new();
            options.create(true).append(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            self.file = Some(options.open(&self.path)?);
        }
        let mut line = serde_json::to_vec(event).map_err(JsonStoreError::Serialize)?;
        line.push(b'\n');
        if let Some(file) = self.file.as_mut() {
            file.write_all(&line)?;
        }
        Ok(())
    }
}

impl EventStore for JsonStore {
    fn append(&mut self, event: &Event) {
        self.index(event);
        match self.write(event) {
            Ok(()) => self.write_error = None,
            Err(e) => {
                tracing::error!(path = %self.path.display(), "event store append failed: {e}");
                self.write_error = Some(e);
            }
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
        // Drop the descriptor before unlinking — required on Windows,
        // harmless elsewhere. The next `append` recreates the file.
        self.file = None;
        match std::fs::remove_file(&self.path) {
            Ok(()) => self.write_error = None,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => {
                tracing::error!(path = %self.path.display(), "event store clear failed: {e}");
                self.write_error = Some(JsonStoreError::Io(e));
            }
        }
    }
}

/// The TS `coerceToString` — numbers and bools stringify, everything
/// else (absent, null, objects, arrays) falls back to `""`.
fn coerce_string(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Bool(b)) => b.to_string(),
        _ => String::new(),
    }
}

/// The TS `toEvent` — lenient per-field coercion: a non-object line,
/// absent fields, or wrong-typed fields decode to defaults rather
/// than failing the load.
fn to_event(raw: &Value) -> Event {
    Event {
        kind: coerce_string(raw.get("type")),
        payload: match raw.get("payload") {
            Some(value) if !value.is_null() => value.clone(),
            _ => Value::Object(serde_json::Map::new()),
        },
        source: coerce_string(raw.get("source")),
        id: coerce_string(raw.get("id")),
        ts: raw
            .get("ts")
            .and_then(Value::as_f64)
            .map_or(0, |t| t as i64),
    }
}

/// `fs.chmodSync(filePath, 0o600)` — the TS tightens an existing
/// log's permissions on open. No-op off unix.
#[cfg(unix)]
fn tighten_permissions(path: &Path) -> Result<(), JsonStoreError> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    Ok(())
}

/// Non-unix placeholder — see the unix variant.
#[cfg(not(unix))]
fn tighten_permissions(_path: &Path) -> Result<(), JsonStoreError> {
    Ok(())
}
