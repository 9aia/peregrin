#![allow(clippy::unwrap_used, clippy::expect_used, clippy::pedantic)]
//! `LocalPlane` (in-process `ControlPlane`) and `ServicePlane` (real
//! `sepia serve`) — the whole `SessionPlane` + typed `SepiaService`
//! surface. Error paths count: a hermetic service answers every call
//! with `not_found`/`unavailable`, which still runs the decode/map
//! arms.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use peregrin_session::{LocalPlane, ServicePlane, SessionPlane, Spec, sepia_binary};
use sepia_core::storage::SessionRepository;
use sepia_core::{Session, StorageError};
use sepia_session::{ControlPlane, ControlPlaneOptions, RestoreRequest, RewindRequest};
use serde_json::json;

struct MemStore {
    sessions: Mutex<HashMap<String, Session>>,
}

#[async_trait::async_trait]
impl SessionRepository for MemStore {
    async fn save(&self, session: &Session) -> Result<(), StorageError> {
        self.sessions
            .lock()
            .unwrap()
            .insert(session.id.clone(), session.clone());
        Ok(())
    }
    async fn get_by_id(
        &self,
        id: &str,
        _agent: Option<&str>,
    ) -> Result<Option<Session>, StorageError> {
        Ok(self.sessions.lock().unwrap().get(id).cloned())
    }
    async fn list(&self) -> Result<Vec<Session>, StorageError> {
        Ok(self.sessions.lock().unwrap().values().cloned().collect())
    }
    async fn delete(&self, id: &str) -> Result<(), StorageError> {
        self.sessions.lock().unwrap().remove(id);
        Ok(())
    }
    async fn has_session(&self, id: &str) -> Result<bool, StorageError> {
        Ok(self.sessions.lock().unwrap().contains_key(id))
    }
}

async fn local() -> LocalPlane {
    let store = Arc::new(MemStore {
        sessions: Mutex::new(HashMap::new()),
    });
    let s: Session = serde_json::from_value(json!({
        "id": "s1",
        "title": "T",
        "workingDirectory": "/w",
        "backendType": "devin",
        "model": "m",
        "createdAt": 1_700_000_000.0,
        "lastActivityAt": 1_700_000_100.0,
        "mainChainId": 0
    }))
    .unwrap();
    store.save(&s).await.unwrap();
    LocalPlane(ControlPlane::new(store, ControlPlaneOptions::default()))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn local_plane_full_surface() {
    let plane = local().await;
    // Reads on the seeded session.
    let list = plane.list_sessions(true).await.unwrap();
    assert_eq!(list.len(), 1);
    plane.list_sessions(false).await.unwrap();
    assert_eq!(plane.get_session("s1", None).await.unwrap().id, "s1");
    assert_eq!(plane.get_summary("s1", None).await.unwrap().id, "s1");
    plane.get_session("s1", Some("devin")).await.unwrap();
    plane
        .get_history("s1", &sepia_session::HistoryOptions::default(), None)
        .await
        .unwrap();
    let opts = sepia_session::HistoryOptions {
        limit: Some(10),
        before: Some(5),
        ..sepia_session::HistoryOptions::default()
    };
    let _ = plane.get_history("s1", &opts, None).await;
    // Mutations — no agent runtime, so the agent-bound calls fail;
    // every line still runs.
    let _ = plane.attach("s1", false, None, None, None).await;
    let _ = plane
        .attach(
            "s1",
            true,
            Some("m".into()),
            Some(vec!["f".into()]),
            Some("devin".into()),
        )
        .await;
    let _ = plane
        .create_session(
            "/w",
            Some("devin".into()),
            Some("t".into()),
            Some("m".into()),
            Some(vec!["f".into()]),
        )
        .await;
    plane.detach("s1").await;
    let _ = plane.prompt("s1", Vec::new(), None).await;
    let _ = plane.cancel("s1", None).await;
    let _ = plane
        .respond_to_permission("s1", "r1", Some("o"), None)
        .await;
    let _ = plane.delete_session("s1", None).await;
    let _ = plane
        .restore(
            "s1",
            &RestoreRequest {
                confirm: true,
                path: Some("a.rs".into()),
                tool_call_id: None,
                checkpoint: Some("cp".into()),
                paths: None,
            },
            None,
        )
        .await;
    let _ = plane
        .rewind(
            "s1",
            &RewindRequest {
                confirm: true,
                node_id: Some(1),
                turns: None,
                checkpoint: None,
            },
            None,
        )
        .await;
    let _ = plane.subscribe("s1", None).await;
    let _ = plane.list_agents().await;
    let _ = plane.can_rewind("devin").await;
    plane.close_all().await;
}

fn service_spec() -> Option<(Spec, tempfile::TempDir, tempfile::TempDir)> {
    let home = tempfile::tempdir().ok()?;
    let drivers = tempfile::tempdir().ok()?;
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn service_plane_full_surface() {
    let Some((spec, _home, _drivers)) = service_spec() else {
        eprintln!("no sepia binary — skipping");
        return;
    };
    let _ = Spec::default();
    let service = peregrin_session::SepiaService::spawn(spec)
        .await
        .expect("spawn");
    let plane = ServicePlane(service);

    // Reads — a hermetic service answers empty lists / not_found.
    let _ = plane.list_sessions(true).await;
    let _ = plane.list_sessions(false).await;
    let _ = plane.get_session("s1", None).await;
    let _ = plane.get_session("s1", Some("devin")).await;
    let _ = plane.get_summary("s1", None).await;
    let _ = plane
        .get_history("s1", &sepia_session::HistoryOptions::default(), None)
        .await;
    let opts = sepia_session::HistoryOptions {
        limit: Some(10),
        before: Some(5),
        ..sepia_session::HistoryOptions::default()
    };
    let _ = plane.get_history("s1", &opts, Some("devin")).await;
    // Mutations — errors are fine, the lines run.
    let _ = plane.attach("s1", false, None, None, None).await;
    let _ = plane
        .attach(
            "s1",
            true,
            Some("m".into()),
            Some(vec!["f".into()]),
            Some("d".into()),
        )
        .await;
    let _ = plane
        .create_session(
            "/w",
            Some("devin".into()),
            Some("t".into()),
            Some("m".into()),
            Some(vec!["f".into()]),
        )
        .await;
    plane.detach("s1").await;
    let _ = plane.prompt("s1", Vec::new(), None).await;
    let _ = plane.cancel("s1", Some("devin")).await;
    let _ = plane
        .respond_to_permission("s1", "r1", Some("o"), None)
        .await;
    let _ = plane
        .respond_to_permission("s1", "r1", None, Some("devin"))
        .await;
    let _ = plane.delete_session("s1", None).await;
    let _ = plane
        .restore(
            "s1",
            &RestoreRequest {
                confirm: true,
                path: Some("a.rs".into()),
                tool_call_id: None,
                checkpoint: Some("cp".into()),
                paths: None,
            },
            None,
        )
        .await;
    let _ = plane
        .rewind(
            "s1",
            &RewindRequest {
                confirm: true,
                node_id: Some(1),
                turns: None,
                checkpoint: None,
            },
            None,
        )
        .await;
    let _ = plane.subscribe("s1", None).await;
    let _ = plane.subscribe("s1", Some("devin")).await;
    let _ = plane.list_agents().await;
    let _ = plane.can_rewind("devin").await;
    plane.close_all().await;

    // The raw `SepiaService` surface that `ServicePlane` doesn't use.
    let _ = plane.0.agents().await;
    let _ = plane.0.sessions(true).await;
    let _ = plane.0.attach("s1", false, None).await;
    let _ = plane.0.attach("s1", true, Some("devin")).await;
    let _ = plane.0.prompt("s1", json!([]), None).await;
    let _ = plane.0.cancel("s1", None).await;
    plane.0.detach("s1").await;
    let _ = plane.0.subscribe("s1", None).await;
    let _ = plane.0.delete("s1", None).await;
    let _ = plane.0.stderr_tail().await;
    let _ = plane.0.notifications();
    // `call` respawn arm: shutdown kills the child, the next call
    // respawns it transparently.
    plane.0.shutdown().await.ok();
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    let _ = plane.0.sessions(false).await;
    plane.0.shutdown().await.ok();
}
