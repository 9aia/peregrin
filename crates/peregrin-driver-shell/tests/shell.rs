#![allow(clippy::unwrap_used)]
use peregrin_driver::Driver;
use peregrin_driver_shell::ShellDriver;
use serde_json::json;

fn node(params: serde_json::Value) -> peregrin_core::Node {
    peregrin_core::Node {
        id: "n".into(),
        driver: "shell".into(),
        params,
        on: vec![],
    }
}
fn event() -> peregrin_core::Event {
    peregrin_core::Event {
        kind: "k".into(),
        payload: json!({}),
    }
}

#[tokio::test]
async fn run_command_paths() {
    let d = ShellDriver;
    assert!(d.run(&node(json!({})), &event()).await.is_err());
    let out = d
        .run(&node(json!({"command":"true"})), &event())
        .await
        .unwrap();
    assert!(out.events.is_empty());
    assert!(
        d.run(&node(json!({"command":"exit 3"})), &event())
            .await
            .is_err()
    );
}
