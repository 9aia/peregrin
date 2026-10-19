#![allow(clippy::unwrap_used, clippy::expect_used, clippy::pedantic)]
//! `SepiaService` against a real `sepia serve` — skipped when no sepia
//! binary resolves (CI without the sibling checkout can set
//! `PEREGRIN_SEPIA` to a built binary).

use peregrin_session::{SepiaService, Spec, sepia_binary};
use serde_json::json;

fn spec() -> Option<(Spec, tempfile::TempDir, tempfile::TempDir)> {
    let home = tempfile::tempdir().ok()?;
    let drivers = tempfile::tempdir().ok()?; // empty → no drivers probed
    sepia_binary().map(|binary| {
        (
            Spec {
                binary,
                env: vec![
                    ("SEPIA_HOME".into(), home.path().display().to_string()),
                    (
                        "SEPIA_DRIVER_DIR".into(),
                        drivers.path().display().to_string(),
                    ),
                    // Hermetic: no installed drivers under HOME/XDG/PATH.
                    ("HOME".into(), home.path().display().to_string()),
                    ("XDG_DATA_HOME".into(), home.path().display().to_string()),
                    ("PATH".into(), "/usr/bin:/bin".into()),
                ],
            },
            home,
            drivers,
        )
    })
}

#[tokio::test]
async fn manifest_handshake_negotiates_protocol() {
    let Some((spec, _home, _drivers)) = spec() else {
        eprintln!("no sepia binary — skipping");
        return;
    };
    let service = SepiaService::spawn(spec).await.expect("spawn sepia serve");
    assert_eq!(service.manifest.name, "sepia");
    assert_eq!(
        service.manifest.protocol,
        peregrin_session::EXPECTED_PROTOCOL
    );
    assert!(service.manifest.methods.contains(&"sessions.list".into()));
    service.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn typed_calls_and_respawn_after_shutdown() {
    let Some((spec, _home, _drivers)) = spec() else {
        eprintln!("no sepia binary — skipping");
        return;
    };
    let service = SepiaService::spawn(spec).await.expect("spawn");
    // Fresh home → no agents/sessions, but the calls must succeed.
    service.agents().await.expect("agents.list");
    match service.sessions(false).await {
        Ok(_) => {}
        Err(e) => panic!(
            "sessions.list failed: {e:?}\nstderr: {:?}",
            service.stderr_tail().await
        ),
    }
    // Graceful exit marks the client dead; the next call respawns and
    // retries transparently.
    service.shutdown().await.expect("shutdown");
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    service
        .sessions(false)
        .await
        .expect("sessions.list after respawn");
}

#[tokio::test]
async fn unknown_method_surfaces_rpc_error() {
    let Some((spec, _home, _drivers)) = spec() else {
        eprintln!("no sepia binary — skipping");
        return;
    };
    let service = SepiaService::spawn(spec).await.expect("spawn");
    let err = service
        .call("nope.method", json!({}))
        .await
        .expect_err("unknown method must error");
    assert_eq!(err.code, -32601);
}
