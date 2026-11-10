#![allow(clippy::unwrap_used, clippy::expect_used, clippy::pedantic)]
//! `campaign serve` e2e — N supervised campaigns under one process:
//! per-campaign supervisors draining on shutdown, the shared
//! `github-webhook` listener demuxing verified pushes by repo, and
//! per-campaign queue-store/history namespacing.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use peregrin_core::{Campaign, Event};
use peregrin_driver_github_webhook::signature_header;
use peregrin_loader::Loader;
use peregrin_loader_yaml::YamlLoader;
use peregrin_node::serve::{self, HostedCampaign, ServeConfig};
use serde_json::json;

fn load(yaml: &str) -> Campaign {
    YamlLoader.load_str(yaml, Path::new(".")).unwrap()
}

fn hosted(name: &str, campaign: Campaign) -> HostedCampaign {
    HostedCampaign {
        name: name.to_string(),
        campaign,
        source: PathBuf::from(format!("{name}.yaml")),
    }
}

/// A `supersede:` campaign whose only node is a `shell` worker on
/// `push` — trigger-driven, no ingress of its own (fed through
/// `Served::injectors` or the shared webhook demux).
fn worker_campaign(name: &str) -> String {
    format!(
        "name: {name}\n\
         supersede:\n  key: \"{{repo}}\"\n  mode: latest-wins\n\
         nodes:\n  - id: work\n    driver: shell\n    on: [push]\n    \
         emits: [work.done]\n    params:\n      cmd: \"true\"\n      \
         emit:\n        \"0\": work.done\n"
    )
}

/// The same worker campaign plus a `github-webhook` ingress node
/// watching `repo` — under `serve` it registers into the shared
/// demux instead of binding `listen`.
fn webhook_campaign(name: &str, repo: &str) -> String {
    format!(
        "name: {name}\n\
         capabilities:\n  secrets: true\n\
         supersede:\n  key: \"{{repo}}\"\n  mode: latest-wins\n\
         nodes:\n  - id: hook\n    driver: github-webhook\n    \
         on: [start]\n    emits: [hook.listening, push]\n    params:\n      \
         listen: \"127.0.0.1:29998\"\n      repos:\n        - name: {repo}\n          \
         secret_env: PEREGRIN_SERVE_TEST_WH_SECRET\n  - id: work\n    \
         driver: shell\n    on: [push]\n    emits: [work.done]\n    \
         params:\n      cmd: \"true\"\n      emit:\n        \"0\": work.done\n"
    )
}

fn client() -> ureq::Agent {
    ureq::Agent::new_with_config(
        ureq::Agent::config_builder()
            .http_status_as_error(false)
            .timeout_global(Some(Duration::from_secs(10)))
            .build(),
    )
}

fn post_push(addr: &std::net::SocketAddr, body: &[u8], secret: &str) -> u16 {
    let sig = signature_header(secret, body);
    let resp = client()
        .post(format!("http://{addr}/hook"))
        .header("x-github-event", "push")
        .header("x-hub-signature-256", &sig)
        .send(body)
        .unwrap();
    resp.status().as_u16()
}

fn trigger(repo: &str, sha: &str) -> Event {
    Event::new(
        "push",
        json!({"repo": repo, "ref": "refs/heads/main", "sha": sha}),
        "test",
    )
}

/// `(campaign name, finished-run log kinds)` captures — the
/// `ServeConfig::on_run` seam tests assert through.
type RunLog = Arc<Mutex<Vec<(String, Vec<String>)>>>;

fn capture() -> RunLog {
    Arc::new(Mutex::new(Vec::new()))
}

fn on_run(logs: &RunLog) -> serve::RunCallback {
    let logs = Arc::clone(logs);
    Arc::new(move |name: &str, log: &[Event]| {
        logs.lock().unwrap().push((
            name.to_string(),
            log.iter().map(|e| e.kind.clone()).collect(),
        ));
    })
}

fn config(logs: &RunLog) -> ServeConfig {
    ServeConfig {
        listen: "127.0.0.1:0".into(),
        path: "/hook".into(),
        history_dir: None,
        queue_store_dir: None,
        on_run: Some(on_run(logs)),
        secrets: Default::default(),
    }
}

/// Poll `cond` until it holds or ~5s passes — listener-run route
/// registration and worker runs are async.
fn wait_for(mut cond: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if cond() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    cond()
}

fn kinds_of(logs: &RunLog, name: &str) -> Vec<String> {
    logs.lock()
        .unwrap()
        .iter()
        .find(|(n, _)| n == name)
        .map(|(_, kinds)| kinds.clone())
        .unwrap_or_default()
}

#[test]
fn serve_hosts_n_campaigns_and_drains_on_disconnect() {
    let logs = capture();
    let mut served = serve::serve_campaigns(
        vec![
            hosted("alpha", load(&worker_campaign("alpha"))),
            hosted("beta", load(&worker_campaign("beta"))),
        ],
        &config(&logs),
    )
    .unwrap();
    // No github-webhook nodes → no shared listener.
    assert!(served.listener.is_none());
    assert_eq!(served.campaigns, vec!["alpha", "beta"]);
    assert_eq!(served.injectors.len(), 2);

    // One trigger per campaign through the external injectors.
    served.injectors[0].inject(trigger("a/r", "1")).unwrap();
    served.injectors[1].inject(trigger("b/r", "2")).unwrap();
    // End of external triggers + request_shutdown → drain + exit.
    served.injectors.clear();
    served.join().unwrap();

    for name in ["alpha", "beta"] {
        let kinds = kinds_of(&logs, name);
        assert!(
            kinds.contains(&"workflow.done".to_string()),
            "{name} ran to done: {kinds:?}"
        );
    }
}

#[test]
fn serve_shared_listener_demuxes_by_repo_with_per_campaign_stores() {
    let tmp = tempfile::tempdir().unwrap();
    let logs = capture();
    let mut cfg = config(&logs);
    cfg.queue_store_dir = Some(tmp.path().join("queues"));
    cfg.history_dir = Some(tmp.path().join("history"));
    // The webhook secret resolves through the record leg — no env
    // mutation needed (env::set_var is unsafe in edition 2024).
    cfg.secrets.insert(
        "PEREGRIN_SERVE_TEST_WH_SECRET".to_string(),
        "s3cr3t".to_string(),
    );
    let served = serve::serve_campaigns(
        vec![
            hosted(
                "wh-alpha",
                load(&webhook_campaign("wh-alpha", "acme/widgets")),
            ),
            hosted(
                "wh-beta",
                load(&webhook_campaign("wh-beta", "acme/gadgets")),
            ),
        ],
        &cfg,
    )
    .unwrap();

    // ONE listener for both campaigns — the per-node `listen`
    // params (127.0.0.1:29998) are ignored under the shared demux.
    let addr = served
        .listener
        .expect("webhook campaigns bind the shared listener");
    let demux = served.demux.clone().unwrap();
    // Listener runs fire async — wait for both campaigns' routes.
    assert!(
        wait_for(|| demux.route_count() == 2),
        "both campaigns registered routes: {:?}",
        demux.repos()
    );

    // A verified push to widgets lands only on wh-alpha's queue.
    let body = json!({
        "ref": "refs/heads/main",
        "after": "2222222222222222222222222222222222222222",
        "repository": { "full_name": "acme/widgets" },
    })
    .to_string()
    .into_bytes();
    assert_eq!(post_push(&addr, &body, "s3cr3t"), 202);

    assert!(
        wait_for(|| kinds_of(&logs, "wh-alpha").contains(&"workflow.done".to_string())),
        "wh-alpha ran: {:?}",
        logs.lock().unwrap()
    );
    std::thread::sleep(Duration::from_millis(200));
    assert!(
        !kinds_of(&logs, "wh-beta")
            .iter()
            .any(|k| k == "workflow.done"),
        "wh-beta must not run for widgets' push"
    );

    // Per-campaign durable state: one sqlite queue db each, one
    // history jsonl for the campaign that ran.
    let qdir = tmp.path().join("queues");
    assert!(qdir.join("wh-alpha.db").is_file());
    assert!(qdir.join("wh-beta.db").is_file());
    assert!(tmp.path().join("history/wh-alpha.jsonl").is_file());

    // Unwatched repo → 404 like the solo driver.
    let foreign = json!({
        "ref": "refs/heads/main",
        "after": "2222222222222222222222222222222222222222",
        "repository": { "full_name": "other/repo" },
    })
    .to_string()
    .into_bytes();
    assert_eq!(post_push(&addr, &foreign, "s3cr3t"), 404);

    served.join().unwrap();
}

#[test]
fn serve_rejects_duplicate_campaign_names() {
    let logs = capture();
    let err = serve::serve_campaigns(
        vec![
            hosted("same", load(&worker_campaign("same"))),
            hosted("same", load(&worker_campaign("same"))),
        ],
        &config(&logs),
    )
    .err()
    .unwrap();
    assert!(err.to_string().contains("duplicate campaign name"), "{err}");
}

#[test]
fn shutdown_drains_the_active_run() {
    // A slow worker: the shutdown flag lands mid-run and the loop
    // must still finish it (the disconnect-drain path), not kill it.
    let yaml = "name: slow\n\
                supersede:\n  key: \"{repo}\"\n  mode: latest-wins\n\
                nodes:\n  - id: work\n    driver: shell\n    on: [push]\n    \
                emits: [work.done]\n    params:\n      cmd: \"sleep 0.4\"\n      \
                emit:\n        \"0\": work.done\n";
    let logs = capture();
    let served = serve::serve_campaigns(vec![hosted("slow", load(yaml))], &config(&logs)).unwrap();
    served.injectors[0].inject(trigger("a/r", "1")).unwrap();
    std::thread::sleep(Duration::from_millis(100)); // let the run start
    let start = Instant::now();
    served.join().unwrap();
    // The in-flight `sleep 0.4` ran to completion — join blocked
    // well past the 100ms mark.
    assert!(start.elapsed() >= Duration::from_millis(250));
    assert!(kinds_of(&logs, "slow").contains(&"workflow.done".to_string()));
}

#[test]
fn campaign_name_falls_back_to_the_file_stem() {
    assert_eq!(
        serve::campaign_name(&load(&worker_campaign("yaml-name")), Path::new("x.yaml")),
        "yaml-name"
    );
    let unnamed = load("nodes:\n  - id: w\n    driver: noop\n    on: [start]\n");
    assert_eq!(
        serve::campaign_name(&unnamed, Path::new("dir/my-camp.yaml")),
        "my-camp"
    );
}
