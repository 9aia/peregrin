#![allow(clippy::assert_is_empty)]
#![allow(clippy::unwrap_used)]
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
