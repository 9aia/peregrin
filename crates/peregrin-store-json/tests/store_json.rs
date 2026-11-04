#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::assert_is_empty
)]
//! `JsonStore` tests — append/read-back, first-seen order, crash
//! recovery (drop + reopen), `by_type`/`latest`/`clear`, malformed
//! lines, and the infallible-port write-error channel.

use std::fs;

use peregrin_core::Event;
use peregrin_store::EventStore;
use peregrin_store_json::{JsonStore, JsonStoreError};
use serde_json::{Value, json};
use tempfile::TempDir;

fn event(kind: &str) -> Event {
    Event::new(kind, json!({}), "test")
}

#[test]
fn appends_and_serves_all_indexes() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("log.jsonl");
    let mut store = JsonStore::open(&path).unwrap();

    // A missing file is an empty store; nothing is created yet.
    assert!(store.all().is_empty());
    assert!(store.latest().is_none());
    assert!(store.by_type("a").is_empty());
    assert!(store.seen_types().is_empty());
    assert!(store.write_error().is_none());
    assert_eq!(store.path(), path.as_path());
    assert!(!path.exists());

    store.append(&Event::new("a", json!({"n": 1}), "node1"));
    store.append(&event("b"));
    store.append(&Event::new("a", json!({"n": 2}), "node1"));

    assert_eq!(store.all().len(), 3);
    assert_eq!(store.by_type("a").len(), 2);
    assert_eq!(store.by_type("b").len(), 1);
    assert_eq!(store.seen_types(), vec!["a", "b"]);
    assert_eq!(store.latest().unwrap().kind, "a");

    // One JSON object per line, in append order (TS serialize shape:
    // type, payload, source, id, ts).
    let content = fs::read_to_string(&path).unwrap();
    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(lines.len(), 3);
    let first: Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(first["type"], "a");
    assert_eq!(first["payload"]["n"], 1);
    assert_eq!(first["source"], "node1");
    assert!(first["id"].is_string());
    assert!(first["ts"].is_number());
}

#[test]
fn reopen_reconstructs_log_and_first_seen_order() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("log.jsonl");
    let first_id;
    {
        let mut store = JsonStore::open(&path).unwrap();
        store.append(&Event::new("b", json!({"i": 0}), "n"));
        store.append(&Event::new("a", json!({"i": 1}), "n"));
        store.append(&Event::new("b", json!({"i": 2}), "n"));
        store.append(&Event::new("c", json!({"i": 3}), "n"));
        first_id = store.all()[0].id.clone();
    } // drop closes the descriptor — the process-crash case

    let store = JsonStore::open(&path).unwrap();
    assert_eq!(store.seen_types(), vec!["b", "a", "c"]);
    assert_eq!(store.all().len(), 4);
    assert_eq!(store.by_type("b").len(), 2);
    assert_eq!(store.by_type("b")[1].payload["i"], 2);
    assert_eq!(store.latest().unwrap().kind, "c");
    // Stored fields round-trip — no regeneration on load.
    assert_eq!(store.all()[0].id, first_id);
    assert_eq!(store.all()[0].source, "n");

    // Appends continue the durable log after a reopen.
    let mut store = store;
    store.append(&event("d"));
    let store = JsonStore::open(&path).unwrap();
    assert_eq!(store.seen_types(), vec!["b", "a", "c", "d"]);
    assert_eq!(store.all().len(), 5);
}

#[test]
fn append_creates_missing_parent_dirs() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("deep").join("nested").join("log.jsonl");
    let mut store = JsonStore::open(&path).unwrap();
    store.append(&event("a"));
    assert!(path.exists());
    let reopened = JsonStore::open(&path).unwrap();
    assert_eq!(reopened.all().len(), 1);
}

#[test]
fn clear_empties_and_truncates_the_log() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("log.jsonl");
    let mut store = JsonStore::open(&path).unwrap();
    store.append(&event("a"));
    store.append(&event("b"));
    assert!(path.exists());

    EventStore::clear(&mut store);
    assert!(store.all().is_empty());
    assert!(store.by_type("a").is_empty());
    assert!(store.seen_types().is_empty());
    assert!(store.latest().is_none());
    assert!(!path.exists(), "clear must truncate the durable record");

    // Appends after clear recreate the file and re-index from empty.
    store.append(&event("x"));
    assert!(path.exists());
    let reopened = JsonStore::open(&path).unwrap();
    assert_eq!(reopened.seen_types(), vec!["x"]);
    assert_eq!(reopened.all().len(), 1);
}

#[test]
fn clear_without_a_file_is_a_noop() {
    let dir = TempDir::new().unwrap();
    let mut store = JsonStore::open(dir.path().join("log.jsonl")).unwrap();
    EventStore::clear(&mut store);
    assert!(store.all().is_empty());
    assert!(store.write_error().is_none());
}

#[test]
fn malformed_line_fails_open_with_line_number() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("log.jsonl");
    fs::write(&path, "{\"type\":\"a\"}\nnot json\n{\"type\":\"b\"}\n").unwrap();
    let err = JsonStore::open(&path).err().unwrap();
    match err {
        JsonStoreError::Parse { line, .. } => assert_eq!(line, 2),
        other => panic!("expected Parse, got {other}"),
    }
}

#[test]
fn blank_lines_are_skipped_on_load() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("log.jsonl");
    fs::write(&path, "\n   \n{\"type\":\"a\"}\n\n{\"type\":\"b\"}\n").unwrap();
    let store = JsonStore::open(&path).unwrap();
    assert_eq!(store.seen_types(), vec!["a", "b"]);
    assert_eq!(store.all().len(), 2);
}

#[test]
fn non_object_and_wrongly_typed_fields_decode_to_defaults() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("log.jsonl");
    fs::write(
        &path,
        concat!(
            "42\n",
            "{\"type\": 7, \"payload\": null, \"source\": true, \"id\": 9, \"ts\": \"x\"}\n",
            "{\"type\": \"y\"}\n",
            "{\"type\": \"z\", \"payload\": 5, \"ts\": 1.5}\n"
        ),
    )
    .unwrap();
    let store = JsonStore::open(&path).unwrap();
    assert_eq!(store.all().len(), 4);

    // A non-object line is a fully-defaulted event (TS `toEvent`).
    let defaulted = &store.all()[0];
    assert_eq!(defaulted.kind, "");
    assert_eq!(defaulted.payload, json!({}));
    assert_eq!(defaulted.source, "");
    assert_eq!(defaulted.id, "");
    assert_eq!(defaulted.ts, 0);

    // `coerceToString` on numbers/bools; `null` payload → {}; a
    // non-number `ts` → 0.
    let coerced = &store.all()[1];
    assert_eq!(coerced.kind, "7");
    assert_eq!(coerced.payload, json!({}));
    assert_eq!(coerced.source, "true");
    assert_eq!(coerced.id, "9");
    assert_eq!(coerced.ts, 0);

    // Missing fields default.
    let sparse = &store.all()[2];
    assert_eq!(sparse.kind, "y");
    assert_eq!(sparse.source, "");

    // A non-object payload passes through as-is; float `ts`
    // truncates to the i64 the port models.
    let pass_through = &store.all()[3];
    assert_eq!(pass_through.payload, json!(5));
    assert_eq!(pass_through.ts, 1);
}

#[test]
fn open_errors_when_the_path_is_unreadable() {
    let dir = TempDir::new().unwrap();
    let blocker = dir.path().join("blocker");
    fs::write(&blocker, "a file, not a dir").unwrap();
    let err = JsonStore::open(blocker.join("sub").join("log.jsonl"))
        .err()
        .unwrap();
    assert!(matches!(err, JsonStoreError::Io(_)), "{err}");
}

#[test]
fn append_failure_is_recorded_and_recovers() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("sub").join("log.jsonl");
    let mut store = JsonStore::open(&path).unwrap();

    // `sub` exists but is a file: mkdir is skipped and the open fails.
    fs::write(dir.path().join("sub"), "blocker").unwrap();
    store.append(&event("a"));
    assert!(
        matches!(store.write_error(), Some(JsonStoreError::Io(_))),
        "{:?}",
        store.write_error()
    );
    // The in-memory index still updates — the TS indexes before
    // writing, so the live view keeps the event.
    assert_eq!(store.all().len(), 1);
    assert!(!path.exists());

    // Clear the blocker: the next append recreates the dirs and the
    // error clears.
    fs::remove_file(dir.path().join("sub")).unwrap();
    store.append(&event("b"));
    assert!(store.write_error().is_none());
    let reopened = JsonStore::open(&path).unwrap();
    assert_eq!(reopened.seen_types(), vec!["b"]);
    assert_eq!(reopened.all().len(), 1);
}

#[cfg(unix)]
#[test]
fn clear_failure_is_recorded() {
    use std::os::unix::fs::PermissionsExt;
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("log.jsonl");
    let mut store = JsonStore::open(&path).unwrap();
    store.append(&event("a"));

    // A read-only parent makes the unlink fail — `clear` still drops
    // the in-memory indexes and records the error.
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o555)).unwrap();
    EventStore::clear(&mut store);
    assert!(matches!(store.write_error(), Some(JsonStoreError::Io(_))));
    assert!(store.all().is_empty());
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o755)).unwrap();
}

#[cfg(unix)]
#[test]
fn files_are_private_and_tightened_on_open() {
    use std::os::unix::fs::PermissionsExt;
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("log.jsonl");
    let mut store = JsonStore::open(&path).unwrap();
    store.append(&event("a"));
    let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600, "created log must be 0o600, got {mode:o}");

    // An existing log's permissions are tightened to 0o600 on open.
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    let _reopened = JsonStore::open(&path).unwrap();
    let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
    assert_eq!(
        mode, 0o600,
        "open must tighten the log to 0o600, got {mode:o}"
    );
}
