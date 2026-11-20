#![allow(clippy::unwrap_used, clippy::expect_used, clippy::pedantic)]
//! `/api/flows` e2e (ADR-0093) — live flow lifecycle on the shared
//! `campaign serve` listener: list/add/delete/reload, route-owner
//! unregister on the shared webhook demux, managed-dir persistence,
//! and the bearer gate.

use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use peregrin_core::{Campaign, Event};
use peregrin_driver_github_webhook::{SharedWebhook, signature_header};
use peregrin_engine::Injector;
use peregrin_loader::Loader;
use peregrin_loader_yaml::YamlLoader;
use peregrin_node::flows::{self, FlowHost, FlowSource};
use peregrin_node::inject;
use peregrin_node::serve::{self, HostedCampaign, ServeConfig};
use serde_json::{Value, json};

fn load(yaml: &str) -> Campaign {
    YamlLoader.load_str(yaml, Path::new(".")).unwrap()
}

/// A trigger-driven worker campaign — no ingress of its own (fed
/// through injectors or `/api/flows` POSTs).
fn worker_yaml(name: &str) -> String {
    format!(
        "name: {name}\n\
         supersede:\n  key: \"{{repo}}\"\n  mode: latest-wins\n\
         nodes:\n  - id: work\n    driver: shell\n    on: [push]\n    \
         emits: [work.done]\n    params:\n      cmd: \"true\"\n      \
         emit:\n        \"0\": work.done\n"
    )
}

/// Worker + `github-webhook` ingress watching `repo` — under serve
/// it registers into the shared demux under the flow's name.
fn webhook_yaml(name: &str, repo: &str) -> String {
    format!(
        "name: {name}\n\
         capabilities:\n  secrets: true\n\
         supersede:\n  key: \"{{repo}}\"\n  mode: latest-wins\n\
         nodes:\n  - id: hook\n    driver: github-webhook\n    \
         on: [start]\n    emits: [hook.listening, push]\n    params:\n      \
         listen: \"127.0.0.1:29998\"\n      repos:\n        - name: {repo}\n          \
         secret_env: PEREGRIN_FLOWS_TEST_WH_SECRET\n  - id: work\n    \
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

fn req(method: &str, url: &str, body: Option<&Value>) -> (u16, Value) {
    let agent = client();
    let res = match method {
        "GET" => agent.get(url).call(),
        "DELETE" => agent.delete(url).call(),
        "PUT" => agent.put(url).send_json(body.unwrap_or(&Value::Null)),
        _ => agent.post(url).send_json(body.unwrap_or(&Value::Null)),
    };
    let res = res.unwrap();
    let status = res.status().as_u16();
    let text = res.into_body().read_to_string().unwrap_or_default();
    let json = serde_json::from_str(&text).unwrap_or(Value::Null);
    (status, json)
}

fn get_flows(addr: SocketAddr) -> Value {
    req("GET", &format!("http://{addr}/api/flows"), None).1
}

fn flow_names(flows: &Value) -> Vec<String> {
    flows["flows"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|f| f["name"].as_str().map(str::to_string))
        .collect()
}

fn flow_state(flows: &Value, name: &str) -> Option<String> {
    flows["flows"]
        .as_array()?
        .iter()
        .find(|f| f["name"] == name)?
        .get("state")?
        .as_str()
        .map(str::to_string)
}

/// Poll `cond` until it holds or ~5s passes.
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

fn post_push(addr: &SocketAddr, body: &[u8], secret: &str) -> u16 {
    let sig = signature_header(secret, body);
    let resp = client()
        .post(format!("http://{addr}/hook"))
        .header("x-github-event", "push")
        .header("x-hub-signature-256", &sig)
        .send(body)
        .unwrap();
    resp.status().as_u16()
}

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

fn config(logs: &RunLog, managed: Option<PathBuf>) -> ServeConfig {
    ServeConfig {
        listen: "127.0.0.1:0".into(),
        path: "/hook".into(),
        history_dir: None,
        queue_store_dir: None,
        managed_dir: managed,
        on_run: Some(on_run(logs)),
        observer: None,
        secrets: BTreeMap::from([(
            "PEREGRIN_FLOWS_TEST_WH_SECRET".to_string(),
            "s3cr3t".to_string(),
        )]),
        labels: Vec::new(),
    }
}

/// Boot source on disk so `reload` has a real file to re-read.
fn boot_hosted(name: &str, dir: &Path) -> HostedCampaign {
    let path = dir.join(format!("{name}.yaml"));
    std::fs::write(&path, worker_yaml(name)).unwrap();
    HostedCampaign {
        name: name.to_string(),
        campaign: load(&worker_yaml(name)),
        source: FlowSource::Path(path),
    }
}

#[test]
fn flows_api_full_lifecycle() {
    let tmp = tempfile::tempdir().unwrap();
    let logs = capture();
    let managed = tmp.path().join("serve");
    let served = serve::serve_campaigns(
        vec![boot_hosted("base", tmp.path())],
        &config(&logs, Some(managed.clone())),
    )
    .unwrap();
    let addr = served.listener;

    // GET lists the boot flow, active.
    let flows = get_flows(addr);
    assert_eq!(flow_names(&flows), vec!["base".to_string()]);
    assert_eq!(flow_state(&flows, "base").as_deref(), Some("active"));

    // Validation failures.
    let (s, _) = req(
        "POST",
        &format!("http://{addr}/api/flows"),
        Some(&json!({"name": "bad/name", "yaml": "x"})),
    );
    assert_eq!(s, 400);
    let (s, _) = req(
        "POST",
        &format!("http://{addr}/api/flows"),
        Some(&json!({"name": "ok"})),
    );
    assert_eq!(s, 400);
    let (s, _) = req(
        "POST",
        &format!("http://{addr}/api/flows"),
        Some(&json!({"name": "ok", "yaml": "nodes: [unclosed"})),
    );
    assert_eq!(s, 400);

    // POST a live flow — 201, persisted into the managed dir.
    let yaml = worker_yaml("api-flow");
    let (s, body) = req(
        "POST",
        &format!("http://{addr}/api/flows"),
        Some(&json!({"name": "api-flow", "yaml": yaml})),
    );
    assert_eq!(s, 201, "{body}");
    assert_eq!(body["state"], "active");
    assert!(managed.join("api-flow.yaml").is_file());

    // Duplicate → 409; the flow shows in GET.
    let (s, _) = req(
        "POST",
        &format!("http://{addr}/api/flows"),
        Some(&json!({"name": "api-flow", "yaml": worker_yaml("api-flow")})),
    );
    assert_eq!(s, 409);
    assert!(wait_for(|| {
        flow_names(&get_flows(addr)).contains(&"api-flow".to_string())
    }));

    // DELETE → 202 draining, then the entry disappears.
    let (s, body) = req("DELETE", &format!("http://{addr}/api/flows/api-flow"), None);
    assert_eq!(s, 202, "{body}");
    assert_eq!(body["draining"], true);
    assert!(wait_for(|| {
        !flow_names(&get_flows(addr)).contains(&"api-flow".to_string())
    }));
    // Second delete → 404.
    let (s, _) = req("DELETE", &format!("http://{addr}/api/flows/api-flow"), None);
    assert_eq!(s, 404);

    // Reload a disk-sourced flow → 202, back to active.
    let (s, body) = req(
        "POST",
        &format!("http://{addr}/api/flows/base/reload"),
        None,
    );
    assert_eq!(s, 202, "{body}");
    assert!(wait_for(|| {
        let f = get_flows(addr);
        flow_state(&f, "base").as_deref() == Some("active") && flow_names(&f).len() == 1
    }));
    // Reload/delete unknown → 404.
    let (s, _) = req(
        "POST",
        &format!("http://{addr}/api/flows/ghost/reload"),
        None,
    );
    assert_eq!(s, 404);
    let (s, _) = req("DELETE", &format!("http://{addr}/api/flows/ghost"), None);
    assert_eq!(s, 404);
    // GET one flow.
    let (s, body) = req("GET", &format!("http://{addr}/api/flows/base"), None);
    assert_eq!(s, 200);
    assert_eq!(body["name"], "base");
    let (s, _) = req("GET", &format!("http://{addr}/api/flows/ghost"), None);
    assert_eq!(s, 404);
    // Unsupported method → 404.
    let (s, _) = req("PUT", &format!("http://{addr}/api/flows"), None);
    assert_eq!(s, 404);

    served.join().unwrap();
}

#[test]
fn flows_api_webhook_flow_registers_and_unregisters_routes() {
    let tmp = tempfile::tempdir().unwrap();
    let logs = capture();
    // Boot fleet has NO webhook flow — the listener still binds and
    // the API-added flow's deliveries must reach the demux.
    let served =
        serve::serve_campaigns(vec![boot_hosted("base", tmp.path())], &config(&logs, None))
            .unwrap();
    let addr = served.listener;
    let demux = served.demux.clone();

    // POST a webhook flow — it registers its repo route live.
    let (s, body) = req(
        "POST",
        &format!("http://{addr}/api/flows"),
        Some(&json!({"name": "api-wh", "yaml": webhook_yaml("api-wh", "acme/widgets")})),
    );
    assert_eq!(s, 201, "{body}");
    assert!(wait_for(|| demux.route_count() == 1));
    assert_eq!(demux.repos_for_owner("api-wh"), vec!["acme/widgets"]);

    // A verified push runs the API-added flow.
    let body_bytes = json!({
        "ref": "refs/heads/main",
        "after": "2222222222222222222222222222222222222222",
        "repository": { "full_name": "acme/widgets" },
    })
    .to_string()
    .into_bytes();
    assert_eq!(post_push(&addr, &body_bytes, "s3cr3t"), 202);
    assert!(wait_for(|| {
        logs.lock()
            .unwrap()
            .iter()
            .any(|(n, k)| n == "api-wh" && k.contains(&"workflow.done".to_string()))
    }));

    // DELETE unregisters the route BEFORE the drain — a push now
    // 404s and `repos_for_owner` is empty while the flow drains.
    let (s, _) = req("DELETE", &format!("http://{addr}/api/flows/api-wh"), None);
    assert_eq!(s, 202);
    assert_eq!(demux.route_count(), 0);
    assert!(demux.repos_for_owner("api-wh").is_empty());
    assert_eq!(post_push(&addr, &body_bytes, "s3cr3t"), 404);
    assert!(wait_for(|| {
        !flow_names(&get_flows(addr)).contains(&"api-wh".to_string())
    }));

    served.join().unwrap();
}

#[test]
fn flows_router_enforces_the_bearer() {
    // Token gate without a full daemon — an empty FlowHost behind
    // `flows::router` + the shared listener helper.
    let host = Arc::new(FlowHost::new(
        BTreeMap::new(),
        None,
        None,
        Vec::new(),
        None,
        Arc::new(SharedWebhook::new()),
        Arc::new(RwLock::new(Vec::<(String, Injector)>::new())),
        None,
        None,
        Arc::new(AtomicBool::new(false)),
    ));
    let shutdown = Arc::new(AtomicBool::new(false));
    let (addr, thread) = inject::serve_listener(
        "127.0.0.1:0",
        flows::router(host, Some("sekret".to_string())),
        Arc::clone(&shutdown),
    )
    .unwrap();

    // No bearer → 401 on every route.
    let (s, _) = req("GET", &format!("http://{addr}/api/flows"), None);
    assert_eq!(s, 401);
    // Wrong bearer → 401.
    let res = client()
        .get(format!("http://{addr}/api/flows"))
        .header("authorization", "Bearer wrong")
        .call()
        .unwrap();
    assert_eq!(res.status().as_u16(), 401);
    // Right bearer → 200.
    let res = client()
        .get(format!("http://{addr}/api/flows"))
        .header("authorization", "Bearer sekret")
        .call()
        .unwrap();
    assert_eq!(res.status().as_u16(), 200);

    shutdown.store(true, std::sync::atomic::Ordering::Relaxed);
    let _ = thread.join();
}
