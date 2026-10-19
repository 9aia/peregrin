#![allow(clippy::unwrap_used, clippy::expect_used, clippy::pedantic)]
//! Raw `DriverClient` against `sepia serve` — isolates the subprocess
//! client machinery from `SepiaService`'s respawn/watch layers.

use peregrin_session::sepia_binary;
use serde_json::json;

#[tokio::test]
async fn raw_client_drives_serve() {
    let Some(binary) = sepia_binary() else {
        eprintln!("no sepia binary — skipping");
        return;
    };
    let home = tempfile::tempdir().unwrap();
    let drivers = tempfile::tempdir().unwrap();
    let env = vec![
        ("SEPIA_HOME".into(), home.path().display().to_string()),
        (
            "SEPIA_DRIVER_DIR".into(),
            drivers.path().display().to_string(),
        ),
        ("HOME".into(), home.path().display().to_string()),
        ("XDG_DATA_HOME".into(), home.path().display().to_string()),
        ("PATH".into(), "/usr/bin:/bin".into()),
    ];
    let client = sepia_host::DriverClient::spawn_with_env(&binary, &["serve"], &env)
        .await
        .unwrap();
    let manifest = client.call("manifest", json!({})).await.unwrap();
    assert_eq!(manifest["protocol"], 1);
    client.call("agents.list", json!({})).await.unwrap();
    client.call("sessions.list", json!({})).await.unwrap();
    client.call("shutdown", json!({})).await.unwrap();
}
