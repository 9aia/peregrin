#![allow(unknown_lints, clippy::assert_is_empty)]
#![allow(clippy::unwrap_used)]
use std::collections::BTreeMap;
use std::rc::Rc;

use hypha_secret::Secrets;
use peregrin_core::Event;
use peregrin_driver::{Driver, DriverError, ExecutionContext, NoopDriver, RunnerDriver};
use serde_json::json;

struct Echo;
impl Driver for Echo {
    fn name(&self) -> &'static str {
        "echo"
    }

    fn invoke(
        &self,
        _node: &peregrin_core::Node,
        _triggers: &[Event],
        _ctx: &ExecutionContext,
    ) -> Result<Vec<Event>, DriverError> {
        Err(DriverError::new("no"))
    }
}

#[test]
fn outcome_and_error() {
    let err = Echo
        .invoke(
            &peregrin_core::Node {
                id: "n".into(),
                driver: "x".into(),
                ..Default::default()
            },
            &[],
            &ExecutionContext::default(),
        )
        .unwrap_err();
    assert_eq!(format!("{err}"), "no");
    assert_eq!(err.reason, "no");
}

#[test]
fn noop_driver_emits_node_done() {
    let node = peregrin_core::Node {
        id: "a".into(),
        ..Default::default()
    };
    let events = NoopDriver
        .invoke(&node, &[], &ExecutionContext::default())
        .unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, "node.done");
    assert_eq!(events[0].source, "a");
    assert_eq!(NoopDriver.name(), "noop");
}

#[test]
fn runner_driver_injects_nodes() {
    let node = peregrin_core::Node {
        id: "r".into(),
        driver: "runner".into(),
        ..Default::default()
    };
    let ctx = ExecutionContext::default();
    // No triggers → no events.
    assert!(RunnerDriver.invoke(&node, &[], &ctx).unwrap().is_empty());

    let trigger = Event::new(
        "start",
        json!({ "nodes": [{ "id": "x", "on": ["start"] }, "junk"] }),
        "__runner__",
    );
    let events = RunnerDriver.invoke(&node, &[trigger], &ctx).unwrap();
    assert_eq!(events[0].payload["injected"], 1);
    assert_eq!(ctx.take_injected().len(), 1);
    assert_eq!(events[0].kind, "r.done");

    // dry_run short-circuits with injected: 0.
    let dry = ExecutionContext {
        dry_run: true,
        ..ExecutionContext::default()
    };
    let events = RunnerDriver
        .invoke(&node, std::slice::from_ref(&Event::start()), &dry)
        .unwrap();
    assert_eq!(events[0].payload["injected"], 0);
}

#[test]
fn runner_driver_reads_specs_from_shell_stdout() {
    let node = peregrin_core::Node {
        id: "r".into(),
        driver: "runner".into(),
        ..Default::default()
    };
    let ctx = ExecutionContext::default();

    // A shell planner's payload — `nodes` can only arrive as JSON
    // text on `stdout` (the `{stdout, rc, cmd}` envelope).
    let trigger = Event::new(
        "plan",
        json!({
            "stdout": "{\"nodes\": [{\"id\": \"w1\"}, {\"id\": \"w2\"}]}",
            "rc": 0,
        }),
        "plan",
    );
    let events = RunnerDriver.invoke(&node, &[trigger], &ctx).unwrap();
    assert_eq!(events[0].payload["injected"], 2);
    assert_eq!(ctx.take_injected().len(), 2);

    // A bare JSON array on `stdout`, and a JSON-string `nodes` field.
    for payload in [
        json!({ "stdout": "[{\"id\": \"w3\"}]" }),
        json!({ "nodes": "[{\"id\": \"w4\"}]" }),
        json!({ "stdout": "{\"workflow\": {\"nodes\": [{\"id\": \"w5\"}]}}" }),
    ] {
        let events = RunnerDriver
            .invoke(
                &node,
                std::slice::from_ref(&Event::new("plan", payload.clone(), "plan")),
                &ctx,
            )
            .unwrap();
        assert_eq!(events[0].payload["injected"], 1, "payload: {payload}");
        assert_eq!(ctx.take_injected().len(), 1);
    }

    // Non-JSON stdout injects nothing — quietly, no error.
    let trigger = Event::new("plan", json!({ "stdout": "hello world", "rc": 0 }), "plan");
    let events = RunnerDriver.invoke(&node, &[trigger], &ctx).unwrap();
    assert_eq!(events[0].payload["injected"], 0);
    assert!(ctx.take_injected().is_empty());
}

#[test]
fn exec_ctx_channels_and_secrets() {
    let ctx = ExecutionContext::default();
    ctx.emit(Event::new("a.b", json!({}), "n"));
    ctx.inject(&[json!({ "id": "x" })]);
    ctx.request_shutdown();
    assert!(ctx.shutdown_requested());
    assert_eq!(ctx.take_emitted().len(), 1);
    assert_eq!(ctx.take_injected().len(), 1);

    // Permissive: missing secrets return "" / default, nothing recorded.
    assert_eq!(ctx.get_secret("MISSING"), "");
    assert_eq!(ctx.get_secret_or("MISSING", "fb"), "fb");
    assert!(ctx.missing_secrets.borrow().is_empty());
    assert_eq!(ctx.secret("MISSING"), "");

    // Strict: misses are recorded unless a non-empty default covers.
    let strict = ExecutionContext {
        strict: true,
        ..ExecutionContext::default()
    };
    assert_eq!(strict.get_secret("TOKEN"), "");
    assert_eq!(strict.get_secret_or("OTHER", "d"), "d");
    assert_eq!(*strict.missing_secrets.borrow(), vec!["TOKEN"]);
}

/// A `Secrets` stand-in for the fallback legs — the real keyring
/// spawns `secret-tool`; tests exercise the chain, not the binary.
struct FakeStore(BTreeMap<String, String>);

impl Secrets for FakeStore {
    fn get(&self, name: &str) -> Option<String> {
        self.0.get(name).cloned()
    }

    fn set(&self, _name: &str, _value: &str) -> std::io::Result<()> {
        Ok(())
    }
}

fn store(pairs: &[(&str, &str)]) -> peregrin_driver::SecretSource {
    peregrin_driver::SecretSource {
        store: Rc::new(FakeStore(
            pairs
                .iter()
                .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                .collect(),
        )),
        declared_only: false,
        batch: None,
    }
}

/// A name that cannot exist in the ambient environment or any store.
const UNSET: &str = "PEREGRIN_TEST_DEFINITELY_UNSET_018";

#[test]
fn secret_chain_ambient_env_after_record_miss() {
    // Edition-2024 forbids `std::env::set_var` (unsafe) — the env leg
    // is verified against a variable every test process carries.
    let expected = std::env::var("PATH").unwrap();
    let ctx = ExecutionContext::default();
    assert_eq!(ctx.get_secret("PATH"), expected);
    // An env hit is not a missing secret — the ledger stays empty.
    assert!(ctx.missing_secrets.borrow().is_empty());
}

#[test]
fn secret_chain_record_beats_env_and_store() {
    let ctx = ExecutionContext {
        secrets: Rc::new(BTreeMap::from([(
            "PATH".to_string(),
            "from-record".to_string(),
        )])),
        secret_stores: Rc::new(vec![store(&[("PATH", "from-store"), (UNSET, "x")])]),
        ..ExecutionContext::default()
    };
    assert_eq!(ctx.get_secret("PATH"), "from-record");
}

#[test]
fn secret_chain_env_beats_store() {
    let ctx = ExecutionContext {
        secret_stores: Rc::new(vec![store(&[("PATH", "from-store")])]),
        ..ExecutionContext::default()
    };
    assert_eq!(ctx.get_secret("PATH"), std::env::var("PATH").unwrap());
}

#[test]
fn secret_chain_store_leg_on_env_miss() {
    let ctx = ExecutionContext {
        secret_stores: Rc::new(vec![store(&[("ONLY_IN_STORE", "v1")])]),
        ..ExecutionContext::default()
    };
    assert_eq!(ctx.get_secret("ONLY_IN_STORE"), "v1");
    assert_eq!(ctx.resolve_secret(UNSET), None);
}

#[test]
fn secret_chain_declared_only_gate() {
    // `declared_only` (the keyring leg) is skipped unless the merged
    // `capabilities.secrets` grant matches the name.
    let ctx = ExecutionContext {
        secret_stores: Rc::new(vec![peregrin_driver::SecretSource {
            declared_only: true,
            ..store(&[("KR_TOKEN", "ring"), ("OTHER", "o")])
        }]),
        ..ExecutionContext::default()
    };
    // No `capabilities.secrets` at all → the store is never consulted.
    assert_eq!(ctx.get_secret("KR_TOKEN"), "");

    let caps = |grant: serde_json::Value| {
        Rc::new(serde_json::Map::from_iter([("secrets".to_string(), grant)]))
    };
    // A list grant declares the named secrets (globs work too).
    let ctx = ExecutionContext {
        capabilities: caps(json!(["KR_TOKEN"])),
        ..ctx
    };
    assert_eq!(ctx.get_secret("KR_TOKEN"), "ring");
    assert_eq!(ctx.get_secret("OTHER"), ""); // undeclared — still gated out

    let ctx = ExecutionContext {
        capabilities: caps(json!(["KR_*"])),
        ..ctx
    };
    assert_eq!(ctx.get_secret("KR_TOKEN"), "ring");

    // `secrets: true` declares every name.
    let ctx = ExecutionContext {
        capabilities: caps(json!(true)),
        ..ctx
    };
    assert_eq!(ctx.get_secret("OTHER"), "o");

    // `secrets: false` declares nothing.
    let ctx = ExecutionContext {
        capabilities: caps(json!(false)),
        ..ctx
    };
    assert_eq!(ctx.get_secret("KR_TOKEN"), "");
}

#[test]
fn secret_chain_missing_records_ledger_once() {
    let ctx = ExecutionContext {
        strict: true,
        secret_stores: Rc::new(vec![store(&[("ONLY_IN_STORE", "v")])]),
        ..ExecutionContext::default()
    };
    // Store hit — no ledger entry.
    assert_eq!(ctx.get_secret("ONLY_IN_STORE"), "v");
    // Full-chain miss — recorded.
    assert_eq!(ctx.get_secret(UNSET), "");
    assert_eq!(*ctx.missing_secrets.borrow(), vec![UNSET.to_string()]);
}

#[test]
fn get_secrets_batch_channel_preserves_chain_order() {
    use std::sync::{Arc, Mutex};

    // A batch-capable first leg: `get` would answer from the same
    // data, but the test proves `get_secrets` prefers `batch` — the
    // closure records exactly which names it was offered.
    let seen = Arc::new(Mutex::new(Vec::new()));
    let data = BTreeMap::from([
        ("BATCH_A".to_string(), "a-value".to_string()),
        ("BATCH_B".to_string(), "b-value".to_string()),
        ("SHARED".to_string(), "first-store".to_string()),
    ]);
    let batch_data = data.clone();
    let batch_seen = Arc::clone(&seen);
    let batched = peregrin_driver::SecretSource {
        store: Rc::new(FakeStore(data)),
        declared_only: false,
        batch: Some(Rc::new(move |names: &[&str]| {
            batch_seen
                .lock()
                .unwrap()
                .extend(names.iter().map(|name| (*name).to_string()));
            names
                .iter()
                .map(|name| batch_data.get(*name).cloned())
                .collect()
        })),
    };
    // Second leg — only sees names the batch leg missed.
    let ctx = ExecutionContext {
        strict: true,
        secrets: Rc::new(BTreeMap::from([(
            "REC".to_string(),
            "rec-value".to_string(),
        )])),
        secret_stores: Rc::new(vec![
            batched,
            store(&[("SHARED", "second-store"), ("LATE", "late-value")]),
        ]),
        ..ExecutionContext::default()
    };

    let out = ctx.get_secrets(&[
        "REC", "PATH", // ambient env leg — every test process has it
        "BATCH_A", "SHARED", "LATE", "BATCH_B", UNSET,
    ]);
    assert_eq!(
        out,
        vec![
            "rec-value".to_string(),
            std::env::var("PATH").unwrap(),
            "a-value".to_string(),
            "first-store".to_string(), // first-hit store wins over "second-store"
            "late-value".to_string(),
            "b-value".to_string(),
            String::new(),
        ]
    );
    // The batch channel saw only the names record+env missed —
    // record/env hits are never offered to stores.
    assert_eq!(
        *seen.lock().unwrap(),
        vec![
            "BATCH_A".to_string(),
            "SHARED".to_string(),
            "LATE".to_string(),
            "BATCH_B".to_string(),
            UNSET.to_string(),
        ]
    );
    // Strict ledger: only the full-chain miss, once — store hits
    // (LATE via leg 2) record nothing.
    assert_eq!(*ctx.missing_secrets.borrow(), vec![UNSET.to_string()]);
}

#[test]
fn get_secrets_without_batch_falls_back_per_name() {
    let ctx = ExecutionContext {
        secret_stores: Rc::new(vec![store(&[("K1", "v1"), ("K2", "v2")])]),
        ..ExecutionContext::default()
    };
    assert_eq!(ctx.get_secrets(&["K1", "K2", UNSET]), vec!["v1", "v2", ""]);
    assert!(ctx.get_secrets(&[]).is_empty());
}
