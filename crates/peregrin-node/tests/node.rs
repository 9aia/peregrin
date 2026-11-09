#![allow(clippy::unwrap_used, clippy::pedantic)]

//! End-to-end: the peregrin-node binary discovers a real driver on
//! SEPIA_DRIVER_DIR, merges its store, and serves /api/sessions.

/// An ephemeral port — racy if something binds between probe and
/// spawn, but far less collision-prone than fixed test ports (a live
/// node on the default 18787 answers the test's HTTP poll instead).
fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .and_then(|l| l.local_addr())
        .map(|a| a.port())
        .unwrap_or(18787)
}

/// SIGINT the child — the node's graceful-shutdown path exits normally,
/// which is the only way an instrumented binary flushes its profraw
/// (SIGKILL discards it). Fall back to `kill()` when it won't die, so
/// teardown never hangs the suite.
fn stop(child: &mut std::process::Child) {
    stop_sig(child, "-INT");
}

fn stop_sig(child: &mut std::process::Child, sig: &str) {
    let _ = std::process::Command::new("kill")
        .args([sig, &child.id().to_string()])
        .status();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    loop {
        match child.try_wait() {
            Ok(Some(_)) | Err(_) => return,
            Ok(None) if std::time::Instant::now() < deadline => {
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            Ok(None) => break,
        }
    }
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn node_serves_sessions_over_http() {
    let tmp = tempfile::tempdir().unwrap();
    let driver_dir = tmp.path().join("drivers");
    let home = tmp.path().join("home");
    let db_dir = tmp.path().join("devin");
    std::fs::create_dir_all(&driver_dir).unwrap();
    std::fs::create_dir_all(&db_dir).unwrap();
    std::fs::create_dir_all(&home).unwrap();

    let driver = sepia_testkit::ensure_driver_bin("sepia-driver-devin");
    std::fs::copy(&driver, driver_dir.join("sepia-driver-devin")).unwrap(); // cross-device-safe: TMPDIR may be tmpfs

    let db = db_dir.join("sessions.db");
    let store = sepia_driver_devin::store::DevinStore::open(&db, false).unwrap();
    let mut session = sepia_testkit::contract::session("devin-1", "Node test", 1_700_000_000.0);
    session.backend_type = "windsurf".into();
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(sepia_core::storage::SessionRepository::save(
            &store, &session,
        ))
        .unwrap();

    let port = free_port();
    let mut child = std::process::Command::new(sepia_testkit::ensure_driver_bin("peregrin-node"))
        .env("SEPIA_DRIVER_DIR", &driver_dir)
        .env("SEPIA_DEVIN_DB", &db)
        .env("PEREGRIN_HOME", &home)
        .env("PEREGRIN_META", home.join("meta.json"))
        .env("PEREGRIN_NODE", home.join("node.json"))
        .env("PEREGRIN_PORT", port.to_string())
        .env("PEREGRIN_HOST", "127.0.0.1")
        .env("HOME", tmp.path())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    let body = loop {
        match ureq::get(format!("http://127.0.0.1:{port}/api/sessions")).call() {
            Ok(mut resp) => break resp.body_mut().read_to_string().unwrap(),
            Err(_) if std::time::Instant::now() < deadline => {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = child.wait();
                panic!("GET /api/sessions never came up: {e}");
            }
        }
    };
    // RSS smoke budget — a headless node must idle small (plan's
    // lightweight rule; adjust if real work lands, never silently).
    // Read *before* stopping: a reaped child has no /proc entry.
    if let Ok(status) = std::fs::read_to_string(format!("/proc/{}/status", child.id())) {
        let rss_kb = status
            .lines()
            .find(|l| l.starts_with("VmRSS"))
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);
        assert!(
            rss_kb < 100 * 1024,
            "node RSS {rss_kb} kB over 100 MB budget"
        );
    }
    assert!(body.contains("devin-1"), "body: {body}");

    // Convert + import ride the plane seams wired in `serve` — a real
    // node with a devin store exercises both closures (the conversion
    // itself may fail on the mock session; the route still ran).
    for (path, payload) in [
        (
            "/api/sessions/devin-1/convert",
            serde_json::json!({"agent": "devin"}),
        ),
        // `agent: cline` → `ImportTarget::Cline` → `install_cline` arm.
        (
            "/api/sessions/devin-1/convert",
            serde_json::json!({"agent": "cline"}),
        ),
        (
            "/api/sessions/import",
            serde_json::json!({
                "agent": "devin",
                "history": [{"role":"user","content":"hi","createdAt":1_700_000_000_000.0}]
            }),
        ),
    ] {
        let _ = ureq::post(format!("http://127.0.0.1:{port}{path}")).send_json(&payload);
    }
    stop(&mut child);
}

/// Node with no drivers at all still boots and serves — the plane
/// comes up empty and the convert/import seams surface agent errors.
#[test]
fn node_without_devin_store_serves() {
    let tmp = tempfile::tempdir().unwrap();
    let driver_dir = tmp.path().join("drivers");
    let home = tmp.path().join("home");
    std::fs::create_dir_all(&driver_dir).unwrap();
    std::fs::create_dir_all(&home).unwrap();

    let port = free_port();
    let mut child = std::process::Command::new(sepia_testkit::ensure_driver_bin("peregrin-node"))
        .env("SEPIA_DRIVER_DIR", &driver_dir)
        .env("PEREGRIN_HOME", &home)
        .env("PEREGRIN_META", home.join("meta.json"))
        .env("PEREGRIN_NODE", home.join("node.json"))
        .env("PEREGRIN_PORT", port.to_string())
        .env("PEREGRIN_HOST", "127.0.0.1")
        // No drivers anywhere — PATH scan mustn't find a machine-wide
        // `sepia-driver-*` install (discovery also walks PATH).
        .env("PATH", &driver_dir)
        .env("HOME", tmp.path())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    loop {
        match ureq::get(format!("http://127.0.0.1:{port}/api/health")).call() {
            Ok(_) => break,
            Err(_) if std::time::Instant::now() < deadline => {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                panic!("node never came up: {e}");
            }
        }
    }
    stop(&mut child);
}

/// Full stack: node -> driver store + mock ACP agent over HTTP.
/// attach -> prompt -> SSE stream sees the echo chunk.
#[test]
fn node_attach_prompt_stream_e2e() {
    let tmp = tempfile::tempdir().unwrap();
    let driver_dir = tmp.path().join("drivers");
    let home = tmp.path().join("home");
    let db_dir = tmp.path().join("devin");
    std::fs::create_dir_all(&driver_dir).unwrap();
    std::fs::create_dir_all(&db_dir).unwrap();
    std::fs::create_dir_all(&home).unwrap();

    let driver = sepia_testkit::ensure_driver_bin("sepia-driver-devin");
    std::fs::copy(&driver, driver_dir.join("sepia-driver-devin")).unwrap(); // cross-device-safe: TMPDIR may be tmpfs
    let mock = sepia_testkit::ensure_mock_acp_agent();

    let db = db_dir.join("sessions.db");
    let store = sepia_driver_devin::store::DevinStore::open(&db, false).unwrap();
    let mut session = sepia_testkit::contract::session("devin-1", "Live test", 1_700_000_000.0);
    session.backend_type = "windsurf".into();
    session.working_directory = "/tmp".into();
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(sepia_core::storage::SessionRepository::save(
            &store, &session,
        ))
        .unwrap();

    let port = free_port();
    let mut child = std::process::Command::new(sepia_testkit::ensure_driver_bin("peregrin-node"))
        .env("SEPIA_DRIVER_DIR", &driver_dir)
        .env("SEPIA_DEVIN_DB", &db)
        .env("PEREGRIN_HOME", &home)
        .env("PEREGRIN_META", home.join("meta.json"))
        .env("PEREGRIN_NODE", home.join("node.json"))
        .env("PEREGRIN_PORT", port.to_string())
        .env("PEREGRIN_HOST", "127.0.0.1")
        .env("SEPIA_AGENT_DEVIN_COMMAND", mock.to_str().unwrap())
        .env("MOCK_CAPS", "load,list")
        .env(
            "MOCK_SESSIONS",
            "devin-1|/tmp|Live test|2024-01-01T00:00:00Z",
        )
        .env("HOME", tmp.path())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    // Generous windows — under `xtask coverage` the spawned node is
    // instrumented and the whole suite runs in parallel, so startup
    // and attach take far longer than a standalone run.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(120);
    let client = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(90)))
        .build()
        .new_agent();
    let base = format!("http://127.0.0.1:{port}");
    // Wait for health.
    loop {
        match client.get(format!("{base}/api/health")).call() {
            Ok(_) => break,
            Err(_) if std::time::Instant::now() < deadline => {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = child.wait();
                panic!("node never came up: {e}");
            }
        }
    }

    // Attach -> the plane loads the session on the mock agent.
    let resp = client
        .post(format!("{base}/api/sessions/devin-1/attach"))
        .send_json(serde_json::json!({}));
    assert!(resp.is_ok(), "attach failed: {resp:?}");

    // Subscribe BEFORE prompting — the broadcast drops pre-subscribe
    // events.
    let mut resp = client
        .get(format!("{base}/api/sessions/devin-1/stream"))
        .call()
        .unwrap();

    // Prompt -> mock echoes -> RunFinished.
    let resp2 = client
        .post(format!("{base}/api/sessions/devin-1/prompt"))
        .send_json(serde_json::json!({ "text": "hello" }));
    assert!(resp2.is_ok(), "prompt failed: {resp2:?}");

    let mut body = String::new();
    let mut buf = [0u8; 8192];
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    while !body.contains("echo: hello") && std::time::Instant::now() < deadline {
        match std::io::Read::read(&mut resp.body_mut().as_reader(), &mut buf) {
            Ok(0) => break,
            Ok(n) => body.push_str(&String::from_utf8_lossy(&buf[..n])),
            Err(_) => break,
        }
    }
    assert!(body.contains("echo: hello"), "stream body: {body}");
    // The SSE stream is an in-flight request — graceful shutdown waits
    // for it forever unless the client drops the connection first.
    drop(resp);
    drop(client);
    stop(&mut child);
}

/// Ephemeral mode end-to-end: `PEREGRIN_EPHEMERAL=1` + SIGTERM flushes
/// live/locked entity snapshots to `$PEREGRIN_HOME/snapshots/` before
/// exit — the CI-runner contract (PL-006 P2).
#[test]
fn ephemeral_node_sigterm_flushes_snapshots() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    std::fs::create_dir_all(&home).unwrap();

    let port = free_port();
    let mut child = std::process::Command::new(sepia_testkit::ensure_driver_bin("peregrin-node"))
        .env("PEREGRIN_EPHEMERAL", "1")
        // In-process session plane — no `sepia serve` subprocess, no
        // driver discovery; the campaign plane is what we exercise.
        .env("PEREGRIN_SESSION_PLANE", "local")
        .env("PEREGRIN_HOME", &home)
        .env("PEREGRIN_META", home.join("meta.json"))
        .env("PEREGRIN_NODE", home.join("node.json"))
        .env("PEREGRIN_PORT", port.to_string())
        .env("PEREGRIN_HOST", "127.0.0.1")
        .env("HOME", tmp.path())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    let base = format!("http://127.0.0.1:{port}");
    loop {
        match ureq::get(format!("{base}/api/health")).call() {
            Ok(_) => break,
            Err(_) if std::time::Instant::now() < deadline => {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                panic!("ephemeral node never came up: {e}");
            }
        }
    }

    // A `paused` campaign entity — `locked: true` in its index doc, so
    // the flush must pick it up. Imported snapshots land paused by
    // contract (no runner thread exists on this side of the wire).
    let snapshot = serde_json::json!({
        "format": "campaign-snapshot/v1",
        "kind": "campaign",
        "entity": {"id": "run-eph1", "kind": "campaign"},
        "state": "paused",
        "log": [{"type": "node.started", "source": "test"}],
        "pendingTimers": [],
        "counters": {}
    });
    let resp = ureq::post(format!("{base}/api/entities/snapshot"))
        .send_json(&snapshot)
        .unwrap();
    assert_eq!(resp.status(), 201);

    // And an idle entity that must NOT flush (queued, unlocked).
    let resp = ureq::post(format!("{base}/api/entities"))
        .send_json(serde_json::json!({
            "kind": "campaign",
            "indexDoc": {"id": "run-idle", "name": "idle"}
        }))
        .unwrap();
    assert_eq!(resp.status(), 201);

    // SIGTERM — the signal CI runners/supervisors actually send.
    stop_sig(&mut child, "-TERM");

    let snap = home.join("snapshots/campaign/run-eph1.snapshot.json");
    let manifest = home.join("snapshots/index.json");
    assert!(
        snap.exists(),
        "expected flushed snapshot at {}",
        snap.display()
    );
    assert!(manifest.exists(), "expected index.json manifest");
    let body: serde_json::Value = serde_json::from_slice(&std::fs::read(&snap).unwrap()).unwrap();
    assert_eq!(body["format"], "campaign-snapshot/v1");
    assert_eq!(body["entity"]["id"], "run-eph1");
    let idx: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&manifest).unwrap()).unwrap();
    let files: Vec<&str> = idx["snapshots"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["file"].as_str().unwrap())
        .collect();
    assert_eq!(files, vec!["campaign/run-eph1.snapshot.json"]);
    assert!(
        !home
            .join("snapshots/campaign/run-idle.snapshot.json")
            .exists(),
        "idle entity must not flush"
    );
}
