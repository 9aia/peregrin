//! `peregrin create|deploy|destroy|runs|explain|log|doctor` — the P6
//! ops verbs over `OpsEnv` sandboxes: a fake bundle store, a
//! `TcpListener` GitHub API stub for the hook upsert/delete paths,
//! fixture unit files/worktrees, and fixture JSONL history logs.
#![allow(clippy::unwrap_used, unknown_lints, clippy::assert_is_empty)]

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::thread;

use peregrin_cli::ops::{self, OpsEnv};
use tamias_store_local::Store;
use tempfile::TempDir;

// --- fixtures -------------------------------------------------------------------

/// Install a bundle into the env's store — manifest + `main.yaml`.
fn install_bundle(env: &OpsEnv, name: &str, version: Option<&str>) -> PathBuf {
    let dir = env.store_root.join("payload-src");
    fs::create_dir_all(&dir).unwrap();
    let version_line = version.map_or(String::new(), |v| format!("version: {v}\n"));
    fs::write(
        dir.join("tamias.yaml"),
        format!("name: {name}\n{version_line}"),
    )
    .unwrap();
    fs::write(dir.join("main.yaml"), "name: x\nnodes: []\n").unwrap();
    Store::new(&env.store_root)
        .install(
            &dir,
            tamias_core::StoreRecord {
                name: name.into(),
                scope: None,
                version: version.map(str::to_string),
                kind: None,
                source_ref: format!("./{name}"),
                resolved: "feedbeef".into(),
                digest: String::new(),
                installed_at: 0,
            },
        )
        .unwrap()
        .dir
}

/// A node config carrying a `github-webhook` ingress node.
fn webhook_config(env: &OpsEnv, repos: &str, extra: &str) -> PathBuf {
    let path = env.config_dir.join("peregrin.yaml");
    fs::create_dir_all(&env.config_dir).unwrap();
    fs::write(
        &path,
        format!(
            "name: gate\nconfig:\n  token_env: TEST_TOKEN\n  public_url: http://public.example.com\n{extra}nodes:\n  - id: gate.ingress\n    driver: github-webhook\n    on: [start]\n    params:\n      api_base: \"{{API}}\"\n{repos}"
        ),
    )
    .unwrap();
    path
}

/// The env-file secrets the chain's file leg resolves — written 0600
/// (the doctor perms check wants owner-only).
fn seed_env(env: &OpsEnv, entries: &[(&str, &str)]) {
    fs::create_dir_all(&env.config_dir).unwrap();
    let mut body = String::new();
    for (k, v) in entries {
        body.push_str(k);
        body.push('=');
        body.push_str(v);
        body.push('\n');
    }
    fs::write(env.env_file(), body).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(env.env_file(), fs::Permissions::from_mode(0o600)).unwrap();
    }
}

/// A canned GitHub API — records request lines, serves
/// `/repos/*/hooks` for exactly `expected` requests.
struct GitHubStub {
    base: String,
}

/// `hooks` is the JSON array `GET /repos/{repo}/hooks` returns.
fn github_stub(hooks: &str, expected: usize) -> (GitHubStub, thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let hooks = hooks.to_string();
    let handle = thread::spawn(move || {
        let mut requests = Vec::new();
        for stream in listener.incoming().take(expected) {
            let mut stream = stream.unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            requests.push(line.trim().to_string());
            // Drain headers + body.
            let mut content_length = 0usize;
            loop {
                let mut header = String::new();
                reader.read_line(&mut header).unwrap();
                let trimmed = header.trim();
                if trimmed.is_empty() {
                    break;
                }
                if let Some(v) = trimmed
                    .to_lowercase()
                    .strip_prefix("content-length:")
                    .map(str::trim)
                {
                    content_length = v.parse().unwrap_or(0);
                }
            }
            if content_length > 0 {
                let mut body = vec![0u8; content_length];
                reader.read_exact(&mut body).unwrap();
            }
            let body = if line.starts_with("GET") {
                hooks.clone()
            } else {
                "{\"id\": 42}".to_string()
            };
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\nconnection: close\r\ncontent-length: {}\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).unwrap();
        }
        requests
    });
    (
        GitHubStub {
            base: format!("http://127.0.0.1:{port}"),
        },
        handle,
    )
}

// --- create -----------------------------------------------------------------------

#[test]
fn create_materializes_config_and_unit() {
    let tmp = TempDir::new().unwrap();
    let env = OpsEnv::under(tmp.path());
    install_bundle(&env, "ci-gate", Some("0.1.0"));

    let report = ops::create(
        &env,
        "ci-gate",
        Some("my-gate"),
        &["repo=9aia/peregrin".to_string(), "timeout=300".to_string()],
        &[
            "9aia/peregrin".to_string(),
            "9aia/myco:MYCO_SECRET".to_string(),
        ],
        None,
        None,
    )
    .unwrap();

    assert_eq!(report.name, "my-gate");
    assert_eq!(report.unit_name, "my-gate");
    assert!(report.config_path.is_file());
    assert!(report.unit_path.is_file());

    // The materialized config: include the bundle main.yaml, merged
    // config params, the inline ingress node.
    let yaml: serde_norway::Value = serde_norway::from_str(&report.yaml).unwrap();
    assert_eq!(yaml["name"].as_str().unwrap(), "my-gate");
    let include = yaml["include"].as_sequence().unwrap();
    assert_eq!(include.len(), 1);
    assert!(include[0].as_str().unwrap().ends_with("main.yaml"));
    assert_eq!(yaml["config"]["repo"].as_str().unwrap(), "9aia/peregrin");
    assert_eq!(yaml["config"]["timeout"].as_u64().unwrap(), 300);
    let nodes = yaml["nodes"].as_sequence().unwrap();
    let ingress = &nodes[0];
    assert_eq!(ingress["driver"].as_str().unwrap(), "github-webhook");
    let repos = ingress["params"]["repos"].as_sequence().unwrap();
    assert_eq!(repos[0]["name"].as_str().unwrap(), "9aia/peregrin");
    assert_eq!(
        repos[0]["secret_env"].as_str().unwrap(),
        "PEREGRIN_WH_9AIA_PEREGRIN"
    );
    assert_eq!(repos[1]["secret_env"].as_str().unwrap(), "MYCO_SECRET");

    // The emitted yaml actually loads as a campaign (include resolves
    // the bundle's absolute path).
    assert_eq!(report.config_path, env.config_dir.join("my-gate.yaml"));
    let campaign = peregrin_node::campaign::load(&report.config_path).unwrap();
    assert_eq!(campaign.name, "my-gate");
    assert!(campaign.nodes.iter().any(|n| n.driver == "github-webhook"));

    // Unit: ExecStart runs `campaign run` with --history; not enabled.
    let unit = fs::read_to_string(&report.unit_path).unwrap();
    assert!(unit.contains("campaign"));
    assert!(unit.contains("--history"));
    assert!(report.unit_path.ends_with("my-gate.service"));

    // Secrets: the two webhook names + the default token name; the
    // env skeleton was seeded with them.
    assert_eq!(
        report.secrets,
        vec![
            "GITHUB_TOKEN".to_string(),
            "MYCO_SECRET".to_string(),
            "PEREGRIN_WH_9AIA_PEREGRIN".to_string()
        ]
    );
    let env_body = fs::read_to_string(env.env_file()).unwrap();
    assert!(env_body.contains("PEREGRIN_WH_9AIA_PEREGRIN="));

    // Re-run: the env file is not clobbered.
    fs::write(env.env_file(), "CUSTOM=1\n").unwrap();
    ops::create(&env, "ci-gate", None, &[], &[], None, None).unwrap();
    assert_eq!(fs::read_to_string(env.env_file()).unwrap(), "CUSTOM=1\n");
}

#[test]
fn create_errors_and_flags() {
    let tmp = TempDir::new().unwrap();
    let env = OpsEnv::under(tmp.path());

    // Missing bundle → install hint.
    let err = ops::create(&env, "ghost", None, &[], &[], None, None).unwrap_err();
    assert!(err.0.contains("peregrin install"));

    // Bundle without main.yaml.
    let dir = env.store_root.join("bare-src");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("tamias.yaml"), "name: bare\n").unwrap();
    Store::new(&env.store_root)
        .install(
            &dir,
            tamias_core::StoreRecord {
                name: "bare".into(),
                scope: None,
                version: None,
                kind: None,
                source_ref: "./bare".into(),
                resolved: "s".into(),
                digest: String::new(),
                installed_at: 0,
            },
        )
        .unwrap();
    let err = ops::create(&env, "bare", None, &[], &[], None, None).unwrap_err();
    assert!(err.0.contains("main.yaml"));

    install_bundle(&env, "gate", None);
    // --set without '=' / empty key / empty repo name.
    assert!(ops::create(&env, "gate", None, &["noeq".into()], &[], None, None).is_err());
    assert!(ops::create(&env, "gate", None, &["=v".into()], &[], None, None).is_err());
    assert!(ops::create(&env, "gate", None, &[], &["noslash".into()], None, None).is_err());
    // --repo with empty secret env part falls back to the default.
    let report = ops::create(
        &env,
        "gate",
        None,
        &[],
        &["o/r:".to_string()],
        None,
        Some("custom-unit"),
    )
    .unwrap();
    let yaml: serde_norway::Value = serde_norway::from_str(&report.yaml).unwrap();
    assert_eq!(
        yaml["nodes"][0]["params"]["repos"][0]["secret_env"]
            .as_str()
            .unwrap(),
        "PEREGRIN_WH_O_R"
    );
    assert!(report.unit_path.ends_with("custom-unit.service"));

    // --config override lands where asked.
    let out = tmp.path().join("elsewhere.yaml");
    let report = ops::create(&env, "gate", None, &[], &[], Some(&out), None).unwrap();
    assert_eq!(report.config_path, out);
}

// --- webhook deploy/destroy ---------------------------------------------------------

#[test]
fn deploy_webhook_create_and_update() {
    let tmp = TempDir::new().unwrap();
    let env = OpsEnv::under(tmp.path());
    seed_env(&env, &[("TEST_TOKEN", "tok"), ("HOOK_SECRET", "whs")]);

    // Create path — the hook list is empty.
    let (stub, handle) = github_stub("[]", 2);
    let config = webhook_config(
        &env,
        "      repos:\n        - {name: a/b, secret_env: HOOK_SECRET}\n",
        "",
    );
    let text = fs::read_to_string(&config)
        .unwrap()
        .replace("{API}", &stub.base);
    fs::write(&config, text).unwrap();
    let lines = ops::deploy_webhook(&env, Some(&config), None, None).unwrap();
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("created hook -> http://public.example.com/hook"));
    let requests = handle.join().unwrap();
    assert_eq!(requests.len(), 2);
    assert!(requests[0].starts_with("GET /repos/a/b/hooks"));
    assert!(requests[1].starts_with("POST /repos/a/b/hooks"));

    // Update path — the list carries a matching hook.
    let (stub2, handle2) = github_stub(
        "[{\"id\": 7, \"config\": {\"url\": \"http://public.example.com/hook\"}}]",
        2,
    );
    let text = fs::read_to_string(&config)
        .unwrap()
        .replace(&stub.base, &stub2.base);
    fs::write(&config, text).unwrap();
    let lines = ops::deploy_webhook(&env, Some(&config), None, None).unwrap();
    assert!(lines[0].contains("updated hook 7"));
    let requests = handle2.join().unwrap();
    assert!(requests[1].starts_with("PATCH /repos/a/b/hooks/7"));
}

#[test]
fn deploy_webhook_errors() {
    let tmp = TempDir::new().unwrap();
    let env = OpsEnv::under(tmp.path());

    // No config found at all.
    assert!(ops::deploy_webhook(&env, None, None, None).is_err());
    // Explicit missing config.
    assert!(ops::deploy_webhook(&env, Some(Path::new("/nonexistent.yaml")), None, None).is_err());

    // Config with no webhook node.
    let path = env.config_dir.join("peregrin.yaml");
    fs::create_dir_all(&env.config_dir).unwrap();
    fs::write(&path, "name: x\nnodes: []\n").unwrap();
    assert!(ops::deploy_webhook(&env, Some(&path), None, None).is_err());

    // Webhook node with empty repos.
    fs::write(
        &path,
        "nodes:\n  - id: w\n    driver: github-webhook\n    params: {}\n",
    )
    .unwrap();
    let err = ops::deploy_webhook(&env, Some(&path), None, None).unwrap_err();
    assert!(err.0.contains("params.repos"));

    // Repos present, but no public URL anywhere.
    fs::write(
        &path,
        "nodes:\n  - id: w\n    driver: github-webhook\n    params:\n      repos:\n        - {name: a/b, secret_env: S}\n",
    )
    .unwrap();
    let err = ops::deploy_webhook(&env, Some(&path), None, None).unwrap_err();
    assert!(err.0.contains("public"));

    // URL + unresolved token name → the missing-secret error.
    let err = ops::deploy_webhook(&env, Some(&path), Some("http://x"), None).unwrap_err();
    assert!(err.0.contains("secret"));

    // Unresolvable repo secret_env → missing-secret error.
    seed_env(&env, &[("GITHUB_TOKEN", "tok")]);
    let err = ops::deploy_webhook(&env, Some(&path), Some("http://x"), None).unwrap_err();
    assert!(err.0.contains("'S' unresolved"));

    // Templated (non-literal) params are rejected.
    fs::write(
        &path,
        "nodes:\n  - id: w\n    driver: github-webhook\n    params:\n      repos:\n        - {name: \"${config.repo}\", secret_env: S}\n",
    )
    .unwrap();
    let err = ops::deploy_webhook(&env, Some(&path), Some("http://x"), None).unwrap_err();
    assert!(err.0.contains("literal"));

    // Missing secret_env entirely.
    fs::write(
        &path,
        "nodes:\n  - id: w\n    driver: github-webhook\n    params:\n      repos:\n        - {name: a/b}\n",
    )
    .unwrap();
    let err = ops::deploy_webhook(&env, Some(&path), Some("http://x"), None).unwrap_err();
    assert!(err.0.contains("secret_env"));
}

#[test]
fn destroy_webhook_paths() {
    let tmp = TempDir::new().unwrap();
    let env = OpsEnv::under(tmp.path());
    seed_env(&env, &[("TEST_TOKEN", "tok"), ("HOOK_SECRET", "whs")]);
    let (stub, handle) = github_stub(
        "[{\"id\": 9, \"config\": {\"url\": \"http://public.example.com/hook\"}}]",
        2,
    );
    let config = webhook_config(
        &env,
        "      repos:\n        - {name: a/b, secret_env: HOOK_SECRET}\n",
        "",
    );
    fs::write(
        &config,
        fs::read_to_string(&config)
            .unwrap()
            .replace("{API}", &stub.base),
    )
    .unwrap();

    // --repo selects; a non-watched repo errs.
    assert!(ops::destroy_webhook(&env, Some(&config), None, None, Some("x/y"), false).is_err());
    let lines = ops::destroy_webhook(&env, Some(&config), None, None, Some("a/b"), false).unwrap();
    assert!(lines[0].contains("deleted hook 9"));
    let requests = handle.join().unwrap();
    assert!(requests[1].starts_with("DELETE /repos/a/b/hooks/9"));

    // --all on the same config; the no-flag guard arm.
    assert!(ops::destroy_webhook(&env, Some(&config), None, None, None, false).is_err());
}

#[test]
fn destroy_webhook_no_match() {
    let tmp = TempDir::new().unwrap();
    let env = OpsEnv::under(tmp.path());
    seed_env(&env, &[("TEST_TOKEN", "tok")]);
    let (stub, handle) = github_stub("[]", 1);
    let config = webhook_config(
        &env,
        "      repos:\n        - {name: a/b, secret_env: HOOK_SECRET}\n",
        "",
    );
    fs::write(
        &config,
        fs::read_to_string(&config)
            .unwrap()
            .replace("{API}", &stub.base),
    )
    .unwrap();
    let lines = ops::destroy_webhook(&env, Some(&config), None, None, None, true).unwrap();
    assert!(lines[0].contains("no hook for"));
    let requests = handle.join().unwrap();
    assert_eq!(requests.len(), 1);
}

// --- destroy service / worktrees -----------------------------------------------------

#[test]
fn destroy_service_removes_unit() {
    let tmp = TempDir::new().unwrap();
    let env = OpsEnv::under(tmp.path());
    let unit = env.unit_dir.join("gate.service");
    fs::create_dir_all(&env.unit_dir).unwrap();
    fs::write(&unit, "[Unit]\n").unwrap();
    let removed = ops::destroy_service(&env, "gate").unwrap();
    assert_eq!(removed, unit);
    assert!(!removed.exists());
    // Missing unit — still fine.
    ops::destroy_service(&env, "ghost").unwrap();
}

#[test]
fn destroy_worktrees_cleans_and_reports() {
    let tmp = TempDir::new().unwrap();
    let env = OpsEnv::under(tmp.path());
    let base = tmp.path().join("wt");

    // A manifest-tracked worktree + an orphan dir.
    let tracked = base.join("run-1");
    fs::create_dir_all(&tracked).unwrap();
    fs::write(tracked.join(".git"), "gitdir: /x").unwrap();
    let orphan = base.join("stray");
    fs::create_dir_all(&orphan).unwrap();
    fs::write(
        base.join(".peregrin-worktrees.json"),
        serde_json::json!({
            "entries": [{"path": tracked, "repo": "/repo", "sha": "feedbeef", "kept": false}]
        })
        .to_string(),
    )
    .unwrap();

    let lines = ops::destroy_worktrees(&env, None, Some(&base)).unwrap();
    assert!(lines.iter().any(|l| l.contains("removed")));
    assert!(lines.iter().any(|l| l.contains("orphan")));
    assert!(lines.last().unwrap().contains("1 worktree(s) removed"));
    assert!(!tracked.exists());
    assert!(orphan.exists()); // orphans are reported, not deleted

    // A kept worktree survives.
    let kept = base.join("keep-1");
    fs::create_dir_all(&kept).unwrap();
    fs::write(
        base.join(".peregrin-worktrees.json"),
        serde_json::json!({
            "entries": [{"path": kept, "repo": "/repo", "sha": "feedbeef", "kept": true}]
        })
        .to_string(),
    )
    .unwrap();
    let lines = ops::destroy_worktrees(&env, None, Some(&base)).unwrap();
    assert!(kept.exists());
    assert!(lines.last().unwrap().contains("0 worktree(s)"));

    // Config-derived base: worktree_base in the node config.
    fs::create_dir_all(&env.config_dir).unwrap();
    let config = env.config_dir.join("peregrin.yaml");
    fs::write(
        &config,
        format!("name: x\nconfig:\n  worktree_base: {}\n", base.display()),
    )
    .unwrap();
    let lines = ops::destroy_worktrees(&env, Some(&config), None).unwrap();
    assert!(lines.last().unwrap().contains("orphan(s)"));

    // Missing explicit config → error.
    assert!(ops::destroy_worktrees(&env, Some(Path::new("/nonexistent.yaml")), None).is_err());
    // No --base and no config → the default base (whatever it is) is
    // used — the verb still runs.
    let _ = ops::destroy_worktrees(&env, None, None);
}

// --- observability ---------------------------------------------------------------------

/// A fixture history log — two pushes' events plus run-level lines.
fn write_history(path: &Path) {
    let events = [
        serde_json::json!({"type": "start", "source": "__runner__", "ts": 1, "id": "e0"}),
        serde_json::json!({"type": "push", "source": "gate.ingress", "ts": 1000,
            "id": "e1", "payload": {"repo": "a/b", "ref": "refs/heads/main",
                "sha": "aaaa1111bbbb2222", "clone_url": "https://x"}}),
        serde_json::json!({"type": "ci-gate.done", "source": "ci-gate.build", "ts": 5000,
            "id": "e2", "payload": {"sha": "aaaa1111bbbb2222", "rc": 0}}),
        serde_json::json!({"type": "push", "source": "gate.ingress", "ts": 9000,
            "id": "e3", "payload": {"repo": "a/c", "sha": "cccc3333dddd4444"}}),
        serde_json::json!({"type": "node.failed", "source": "ci-gate.build", "ts": 10000,
            "id": "e4", "payload": {"sha": "cccc3333dddd4444", "node": "ci-gate.build"}}),
        serde_json::json!({"type": "push", "source": "gate.ingress", "ts": 12000,
            "id": "e5", "payload": {"repo": "a/b", "sha": "eeee5555ffff6666"}}),
    ];
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut body = String::new();
    for e in &events {
        body.push_str(&e.to_string());
        body.push('\n');
    }
    fs::write(path, body).unwrap();
}

#[test]
fn runs_lists_newest_first() {
    let tmp = TempDir::new().unwrap();
    let env = OpsEnv::under(tmp.path());
    write_history(&env.history_path());

    let lines = ops::runs(&env, None, None, None).unwrap();
    assert_eq!(lines.len(), 4); // header + 3 runs
    assert!(lines[0].contains("status"));
    // Newest first — eeee > cccc > aaaa.
    assert!(lines[1].starts_with("eeee5555ffff"));
    assert!(lines[2].starts_with("cccc3333dddd"));
    assert!(lines[3].starts_with("aaaa1111bbbb"));
    assert!(lines[1].contains("running"));
    assert!(lines[2].contains("failed"));
    assert!(lines[3].contains("done"));
    assert!(lines[3].contains("4s"));

    // --repo filter + -n limit.
    let lines = ops::runs(&env, None, Some("a/b"), Some(1)).unwrap();
    assert_eq!(lines.len(), 2);

    // Missing log → "no runs yet".
    let empty = OpsEnv::under(&tmp.path().join("other"));
    assert!(ops::runs(&empty, None, None, None).unwrap().is_empty());
}

#[test]
fn explain_dump_and_ladder() {
    let tmp = TempDir::new().unwrap();
    let env = OpsEnv::under(tmp.path());
    write_history(&env.history_path());

    // sha prefix → the run's events + outcome.
    let lines = ops::explain(&env, None, None, "aaaa").unwrap();
    assert!(lines[0].contains("done"));
    assert!(lines.iter().any(|l| l.contains("push")));
    assert!(lines.last().unwrap().starts_with("outcome:"));

    // A running (no verdict) group → no outcome line.
    let lines = ops::explain(&env, None, None, "eeee").unwrap();
    assert!(lines[0].contains("running"));
    assert!(!lines.last().unwrap().starts_with("outcome:"));

    // Event id match when no sha group hits.
    let lines = ops::explain(&env, None, None, "e0").unwrap();
    assert!(lines[0].contains("event(s)"));

    // Worktree fallback — a dir named *-<sha> under worktree_base.
    fs::create_dir_all(&env.config_dir).unwrap();
    let base = tmp.path().join("wt");
    let wt = base.join("gate-deadbeef");
    fs::create_dir_all(&wt).unwrap();
    let config = env.config_dir.join("peregrin.yaml");
    fs::write(
        &config,
        format!("name: x\nconfig:\n  worktree_base: {}\n", base.display()),
    )
    .unwrap();
    let lines = ops::explain(&env, None, Some(&config), "deadbeef").unwrap();
    assert!(lines[0].contains("died mid-flight"));

    // Never seen.
    let lines = ops::explain(&env, None, Some(&config), "00000000").unwrap();
    assert!(lines[0].contains("never seen"));
}

#[test]
fn log_tails() {
    let tmp = TempDir::new().unwrap();
    let env = OpsEnv::under(tmp.path());
    write_history(&env.history_path());
    let lines = ops::log(&env, None, 2).unwrap();
    assert_eq!(lines.len(), 2);
    assert!(lines[1].contains("eeee5555ffff6666"));
    let empty = OpsEnv::under(&tmp.path().join("other"));
    assert!(ops::log(&empty, None, 5).unwrap().is_empty());
}

#[test]
fn doctor_reports() {
    let tmp = TempDir::new().unwrap();
    let env = OpsEnv::under(tmp.path());

    // Everything missing → failures.
    let report = ops::doctor(&env, None, false).unwrap();
    assert!(report.failures > 0);
    assert!(report.checks.iter().any(|c| c.name == "config" && !c.ok));

    // Wire a config + secrets + store → green-leaning report.
    install_bundle(&env, "b", None);
    seed_env(&env, &[("TEST_TOKEN", "t"), ("HOOK_SECRET", "s")]);
    let config = webhook_config(
        &env,
        "      repos:\n        - {name: a/b, secret_env: HOOK_SECRET}\n",
        "",
    );
    fs::write(
        &config,
        fs::read_to_string(&config)
            .unwrap()
            .replace("{API}", "http://x"),
    )
    .unwrap();
    let report = ops::doctor(&env, Some(&config), false).unwrap();
    assert!(report.checks.iter().any(|c| c.name == "config" && c.ok));
    assert!(
        report
            .checks
            .iter()
            .any(|c| c.name == "secret resolves" && c.ok)
    );
    assert!(report.checks.iter().any(|c| c.name == "env file" && c.ok));
    assert!(report.checks.iter().any(|c| c.name == "public_url"));

    // --json emits the machine report.
    let report = ops::doctor(&env, Some(&config), true).unwrap();
    assert_eq!(report.failures, 0);
}

#[test]
fn env_and_config_helpers() {
    let tmp = TempDir::new().unwrap();
    let env = OpsEnv::under(tmp.path());
    assert!(env.env_file().ends_with("env"));
    assert!(env.history_path().ends_with("history.jsonl"));
    let default = OpsEnv::default();
    assert!(default.config_dir.ends_with("peregrin"));

    // find_config: ./peregrin.yaml in cwd wins over the config dir.
    let cwd = std::env::current_dir().unwrap();
    let local = cwd.join("peregrin.yaml");
    if local.exists() {
        return; // a real file in the way — don't stomp it
    }
    fs::write(&local, "name: local\n").unwrap();
    let found = ops::find_config(&env, None).unwrap();
    fs::remove_file(&local).unwrap();
    assert!(found.ends_with("peregrin.yaml"));
}
