//! `peregrin import bupha` e2e — a representative multi-repo bupha
//! config converts to a structurally-complete node-config workflow:
//! every repo lands in the webhook `repos` watch list, every job gets
//! its `on:` subscription, secret_env passes through as names only.
//! Plus the `libraries/ci-gate` + `libraries/release` bundles'
//! manifest/pack/verify path.
#![allow(
    clippy::unwrap_used,
    unknown_lints,
    clippy::assert_is_empty,
    clippy::default_constructed_unit_structs
)]

use std::fs;
use std::path::{Path, PathBuf};

use peregrin_cli::import::import_bupha;
use tempfile::TempDir;

/// The workspace `libraries/` dir — tests run with cwd = the crate
/// dir, so `../../libraries` is the stable anchor.
fn libraries() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../libraries")
        .canonicalize()
        .unwrap()
}

/// A synthesized multi-repo config mirroring the live `bupha.yaml`'s
/// shape: single-job repos, one multi-job repo, a sturn-opted repo,
/// runner inheritance, the global dagger default, tunnel/worktree/
/// status sections. (The live file is ~190 repos of the same shape —
/// NOT committed; the e2e coverage is structural.)
const FIXTURE_YAML: &str = r#"
repos:
  - name: "9aia/sturn"
    secret_env: "BUPHA_WH_9AIA_STURN"
    dir: "~/GitHub/9aia/sturn"
    jobs:
      - name: "docs"
        branch: "refs/heads/docs"
        runner:
          kind: "script"
          cmd: "bash scripts/deploy-docs.sh"
          cancel_running: true
      - name: "main"
        branch: "refs/heads/main"
        runner:
          kind: "script"
          cmd: "cargo xtask check && cargo xtask test"
          cancel_running: true
  - name: "9aia/kuda"
    secret_env: "BUPHA_WH_9AIA_KUDA"
    dir: "~/GitHub/9aia/kuda"
    branch: "refs/heads/main"
    runner:
      kind: "script"
      cmd: "bash scripts/ci.sh"
      cancel_running: true
  - name: "9aia/peregrin"
    secret_env: "BUPHA_WH_9AIA_PEREGRIN"
    dir: "~/GitHub/9aia/peregrin-v2"
    sturn: true
    jobs:
      - name: "v2"
        branch: "refs/heads/v2"
        runner:
          kind: "script"
          cmd: "bash scripts/ci.sh"
          cancel_running: true
  - name: "handl-lang/lib-base64"
    secret_env: "BUPHA_WH_HANDL_LANG_LIB_BASE64"
    dir: "~/GitHub/handl-lang/lib-base64"
    branch: "refs/heads/main"
    runner:
      kind: "script"
      cmd: "bash ci.sh"
      cancel_running: true

trigger:
  kind: "github"
  listen: "127.0.0.1:29173"
  secret_env: "BUPHA_WEBHOOK_SECRET"
  sturn:
    remote: ""
    poll_ms: 5000

runner:
  kind: "dagger"
  function: "ci"
  token_env: "BUPHA_TOKEN"
  cancel_running: true
  sccache_conf: "~/.config/bupha/sccache-env"

status:
  kind: "github"
  context: "ci/bupha"

tunnel:
  provider: "cloudflared"
  name: "bupha"
  local_url: "http://127.0.0.1:29173"

worktree:
  base: "~/bupha/worktrees/bupha"
"#;

#[test]
fn bupha_import_emits_a_structurally_complete_workflow() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("bupha.yaml");
    fs::write(&src, FIXTURE_YAML).unwrap();
    let out = tmp.path().join("peregrin.yaml");
    let report = import_bupha(&src, Some(&out)).unwrap();

    assert_eq!(report.repos, 4);
    assert_eq!(report.jobs, 5); // sturn×2 + kuda + peregrin-v2 + lib-base64
    assert_eq!(report.nodes, 4 + 5 * 3); // webhook, sturn-poll, route, failmap, 5 pipelines
    assert_eq!(fs::read_to_string(&out).unwrap(), report.yaml);

    // The emitted file parses as a workflow document — the
    // structural-complete check (nodes/on:/params all reachable).
    let campaign = peregrin_node::campaign::load(&out).unwrap();
    assert_eq!(campaign.name, "ci-gate");
    let nodes = &campaign.nodes;
    assert_eq!(nodes.len(), report.nodes);

    // Every repo appears in the webhook node's repos — secret_env as
    // NAMES (the live file's BUPHA_WH_* names), never values.
    let webhook = nodes.iter().find(|n| n.id == "ci.ingress.webhook").unwrap();
    assert_eq!(webhook.driver, "github-webhook");
    assert_eq!(webhook.on, vec!["start".to_string()]);
    let repos = webhook.params["repos"].as_array().unwrap();
    let watched: Vec<&str> = repos.iter().map(|r| r["name"].as_str().unwrap()).collect();
    for repo in [
        "9aia/sturn",
        "9aia/kuda",
        "9aia/peregrin",
        "handl-lang/lib-base64",
    ] {
        assert!(watched.contains(&repo), "missing {repo}");
    }
    let secrets: Vec<&str> = repos
        .iter()
        .map(|r| r["secret_env"].as_str().unwrap())
        .collect();
    for name in [
        "BUPHA_WH_9AIA_STURN",
        "BUPHA_WH_9AIA_KUDA",
        "BUPHA_WH_9AIA_PEREGRIN",
        "BUPHA_WH_HANDL_LANG_LIB_BASE64",
    ] {
        assert!(secrets.contains(&name), "missing secret name {name}");
    }
    assert!(!report.yaml.contains("ghp_") && !report.yaml.contains("wss"));

    // The sturn-opted repo gets its poller (local mode → dir watch).
    let sturn = nodes
        .iter()
        .find(|n| n.id == "ci.ingress.sturn.9aia-peregrin")
        .unwrap();
    assert_eq!(sturn.driver, "sturn-poll");
    assert!(
        sturn.params["repo"]
            .as_str()
            .unwrap()
            .ends_with("GitHub/9aia/peregrin-v2")
    );
    assert_eq!(sturn.params["poll_ms"], 5000);

    // The router re-types pushes; the failmap re-types materialization
    // failures.
    let route = nodes.iter().find(|n| n.id == "ci.route").unwrap();
    assert_eq!(route.max_fires, peregrin_core::MaxFires::Unlimited);
    assert!(
        route.params["emit"]["0"]
            .as_str()
            .unwrap()
            .contains("${event.payload.repo}")
    );
    assert!(nodes.iter().any(|n| n.id == "ci.failmap"));

    // Every job gets an `on:` subscription — repo+ref literal in the
    // routed type — plus the pending/verdict sink pair.
    let jobs = [
        (
            "ci.9aia-sturn-docs",
            "9aia/sturn",
            "refs/heads/docs",
            "ci/bupha/docs",
        ),
        (
            "ci.9aia-sturn-main",
            "9aia/sturn",
            "refs/heads/main",
            "ci/bupha/main",
        ),
        ("ci.9aia-kuda", "9aia/kuda", "refs/heads/main", "ci/bupha"),
        (
            "ci.9aia-peregrin-v2",
            "9aia/peregrin",
            "refs/heads/v2",
            "ci/bupha/v2",
        ),
        (
            "ci.handl-lang-lib-base64",
            "handl-lang/lib-base64",
            "refs/heads/main",
            "ci/bupha",
        ),
    ];
    for (id, repo, branch, context) in jobs {
        let git = nodes
            .iter()
            .find(|n| n.id == id)
            .unwrap_or_else(|| panic!("{id}"));
        assert_eq!(git.driver, "git", "{id}");
        assert!(
            git.on.contains(&format!("push.{repo}.{branch}.**.*")),
            "{id} on: {:?}",
            git.on
        );
        assert_eq!(git.params["sha"], "${event.match.2}", "{id}");
        let status = nodes
            .iter()
            .find(|n| n.id == format!("{id}.status"))
            .unwrap();
        assert_eq!(status.driver, "github-status");
        assert_eq!(status.params["context"], context, "{id}");
        assert_eq!(status.params["token_env"], "BUPHA_TOKEN", "{id}");
        assert!(
            nodes.iter().any(|n| n.id == format!("{id}.pending")),
            "{id}.pending"
        );
    }
    // The sturn repo's job carries the dir-identity pattern too.
    let v2 = nodes
        .iter()
        .find(|n| n.id == "ci.9aia-peregrin-v2")
        .unwrap();
    assert_eq!(v2.on.len(), 2);
    assert!(
        v2.on
            .iter()
            .any(|p| p.ends_with("peregrin-v2.refs/heads/v2.**.*"))
    );

    // The dagger-defaulted jobs inherit repo-level script runners —
    // no stubs in this fixture (every job overrides); warnings carry
    // the tunnel + dir notes.
    assert!(report.warnings.iter().any(|w| w.contains("[tunnel]")));
    assert!(
        report
            .warnings
            .iter()
            .any(|w| w.contains("9aia/peregrin") && w.contains("peregrin-v2"))
    );
}

#[test]
fn bupha_import_toml_fixture() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("bupha.toml");
    fs::write(
        &src,
        r#"
[[repos]]
name = "a/one"
dir = "/srv/one"
branch = "refs/heads/main"
secret_env = "SEC_ONE"
[repos.runner]
kind = "script"
cmd = "bash ci.sh"
cancel_running = true
token_env = "GH_TOK"

[trigger]
kind = "github"

[runner]
kind = "script"
"#,
    )
    .unwrap();
    let report = import_bupha(&src, None).unwrap();
    assert_eq!((report.repos, report.jobs), (1, 1));
    let campaign_yaml = tmp.path().join("w.yaml");
    fs::write(&campaign_yaml, &report.yaml).unwrap();
    let campaign = peregrin_node::campaign::load(&campaign_yaml).unwrap();
    let git = campaign.nodes.iter().find(|n| n.id == "ci.a-one").unwrap();
    assert_eq!(git.params["token_env"], "GH_TOK");
    assert_eq!(git.params["inner"]["params"]["cmd"], "bash ci.sh");
}

#[test]
fn ci_gate_bundle_packs_validates_and_installs() {
    for bundle in ["ci-gate", "release"] {
        let dir = libraries().join(bundle);
        // Manifest loads + validates.
        let manifest = tamias_core::Manifest::load(&dir).unwrap();
        assert_eq!(manifest.name, bundle);
        let expected = match bundle {
            "ci-gate" => "0.2.2",
            _ => "0.1.0",
        };
        assert_eq!(manifest.version.as_deref(), Some(expected));
        // The campaign payload parses through the real loader.
        let campaign = peregrin_node::campaign::load(&dir.join("main.yaml")).unwrap();
        assert_eq!(campaign.name, bundle);
        assert!(
            campaign
                .nodes
                .iter()
                .any(|n| n.driver == "git" && n.driver != "github-webhook")
        );
        assert!(campaign.nodes.iter().any(|n| n.driver == "github-status"));
        assert!(campaign.nodes.iter().all(|n| !n.on.is_empty()));
        // params.schema.json is valid JSON Schema-ish JSON — ci-gate
        // 0.2.x ships the fleet artifact without one.
        let schema_path = dir.join("params.schema.json");
        if schema_path.exists() {
            let schema: serde_json::Value =
                serde_json::from_str(&fs::read_to_string(schema_path).unwrap()).unwrap();
            assert_eq!(schema["type"], "object");
            assert!(schema["properties"].is_object());
        }
    }
}
