#![allow(clippy::unwrap_used)]
//! `SharedWebhook` — the `campaign serve` demux: repo→routes mapping,
//! all-match delivery, per-route secrets, dedup, and the
//! `SharedWebhookDriver` registration path end to end over HTTP.

use std::sync::Arc;
use std::time::Duration;

use peregrin_core::Node;
use peregrin_driver::{Driver, ExecutionContext};
use peregrin_driver_github_webhook::{Route, SharedWebhook, SharedWebhookDriver, signature_header};
use peregrin_engine::{Ingress, Injector, ingress};
use serde_json::{Map, Value, json};

const SECRET_A: &str = "secret-a";
const SECRET_B: &str = "secret-b";
const REPO_A: &str = "acme/widgets";
const REPO_B: &str = "acme/gadgets";

fn route(secret: &str, injector: &Injector, source: &str) -> Route {
    Route {
        secret: secret.to_string(),
        injector: injector.clone(),
        source: source.to_string(),
        web_base: "https://github.com".to_string(),
    }
}

fn client() -> ureq::Agent {
    ureq::Agent::new_with_config(
        ureq::Agent::config_builder()
            .http_status_as_error(false)
            .timeout_global(Some(Duration::from_secs(10)))
            .build(),
    )
}

fn post(url: &str, headers: &[(&str, &str)], body: &[u8]) -> (u16, String) {
    let mut req = client().post(url);
    for (name, value) in headers {
        req = req.header(*name, *value);
    }
    let mut resp = req.send(body).unwrap();
    let status = resp.status().as_u16();
    let text = resp.body_mut().read_to_string().unwrap();
    (status, text)
}

fn push_body(repo: &str) -> Vec<u8> {
    json!({
        "ref": "refs/heads/main",
        "before": "1111111111111111111111111111111111111111",
        "after": "2222222222222222222222222222222222222222",
        "pusher": { "name": "octocat" },
        "repository": { "full_name": repo },
    })
    .to_string()
    .into_bytes()
}

fn post_push(url: &str, secret: &str, body: &[u8], delivery: Option<&str>) -> (u16, String) {
    let mut headers = vec![
        ("x-github-event".to_string(), "push".to_string()),
        (
            "x-hub-signature-256".to_string(),
            signature_header(secret, body),
        ),
    ];
    if let Some(id) = delivery {
        headers.push(("x-github-delivery".to_string(), id.to_string()));
    }
    let refs: Vec<(&str, &str)> = headers
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    post(url, &refs, body)
}

fn drain(chan: &Ingress) -> Vec<peregrin_core::Event> {
    let mut out = Vec::new();
    chan.drain_into(&mut out);
    out
}

/// Serve `demux` on an ephemeral port, returning `(demux, base_url)`.
fn serving(demux: &Arc<SharedWebhook>) -> String {
    let addr = demux.serve("127.0.0.1:0", "/hook").unwrap();
    format!("http://{addr}")
}

#[test]
fn demux_routes_each_repo_to_its_campaign() {
    let demux = Arc::new(SharedWebhook::new());
    let (inj_a, chan_a) = ingress();
    let (inj_b, chan_b) = ingress();
    demux.register(REPO_A, route(SECRET_A, &inj_a, "hook-a"));
    demux.register(REPO_B, route(SECRET_B, &inj_b, "hook-b"));
    assert_eq!(demux.repos(), vec![REPO_B, REPO_A]); // sorted
    assert_eq!(demux.route_count(), 2);
    let base = serving(&demux);
    let url = format!("{base}/hook");

    // Repo A signed with A's secret lands only on A's channel, with
    // A's node id as source and A's web base for the clone_url
    // fallback (payload omits repository.clone_url).
    let body = push_body(REPO_A);
    assert_eq!(
        post_push(&url, SECRET_A, &body, None),
        (202, "queued".to_string())
    );
    let events = drain(&chan_a);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, "push");
    assert_eq!(events[0].source, "hook-a");
    assert_eq!(events[0].payload["repo"], REPO_A);
    assert_eq!(
        events[0].payload["clone_url"],
        "https://github.com/acme/widgets.git"
    );
    assert!(
        drain(&chan_b).is_empty(),
        "repo B must not see repo A's push"
    );

    // Repo B likewise — and cross-signed deliveries 401.
    let body = push_body(REPO_B);
    assert_eq!(
        post_push(&url, SECRET_B, &body, None),
        (202, "queued".to_string())
    );
    assert_eq!(drain(&chan_b).len(), 1);
    assert_eq!(drain(&chan_a).len(), 0);
    let body = push_body(REPO_A);
    assert_eq!(post_push(&url, SECRET_B, &body, None).0, 401);

    // Unwatched repo → 404 even correctly signed.
    let body = push_body("other/repo");
    assert_eq!(
        post_push(&url, SECRET_A, &body, None),
        (404, "unwatched repo".to_string())
    );
}

#[test]
fn same_repo_delivers_to_all_matching_routes() {
    let demux = Arc::new(SharedWebhook::new());
    let (inj_a, chan_a) = ingress();
    let (inj_b, chan_b) = ingress();
    // Two campaigns watch the same repo under the SAME secret —
    // the all-match rule: both get every verified push.
    demux.register(REPO_A, route(SECRET_A, &inj_a, "hook-a"));
    demux.register(REPO_A, route(SECRET_A, &inj_b, "hook-b"));
    let base = serving(&demux);
    let url = format!("{base}/hook");

    let body = push_body(REPO_A);
    assert_eq!(
        post_push(&url, SECRET_A, &body, None),
        (202, "queued".to_string())
    );
    let events_a = drain(&chan_a);
    let events_b = drain(&chan_b);
    assert_eq!(events_a.len(), 1);
    assert_eq!(events_b.len(), 1);
    // Sources stay distinct — each campaign sees its own node id.
    assert_eq!(events_a[0].source, "hook-a");
    assert_eq!(events_b[0].source, "hook-b");
}

#[test]
fn same_repo_different_secrets_only_the_signer_gets_it() {
    let demux = Arc::new(SharedWebhook::new());
    let (inj_a, chan_a) = ingress();
    let (inj_b, chan_b) = ingress();
    // Campaign A and B both watch REPO_A but resolved different
    // webhook secrets — GitHub signs with whichever hook secret is
    // configured; only that campaign's route validates.
    demux.register(REPO_A, route(SECRET_A, &inj_a, "hook-a"));
    demux.register(REPO_A, route(SECRET_B, &inj_b, "hook-b"));
    let base = serving(&demux);
    let url = format!("{base}/hook");

    let body = push_body(REPO_A);
    assert_eq!(
        post_push(&url, SECRET_A, &body, None),
        (202, "queued".to_string())
    );
    assert_eq!(drain(&chan_a).len(), 1, "signer's route delivers");
    assert!(drain(&chan_b).is_empty(), "other secret's route skips");
}

#[test]
fn shared_dedup_ring_spans_routes() {
    let demux = Arc::new(SharedWebhook::new());
    let (inj_a, chan_a) = ingress();
    let (inj_b, chan_b) = ingress();
    demux.register(REPO_A, route(SECRET_A, &inj_a, "hook-a"));
    demux.register(REPO_A, route(SECRET_A, &inj_b, "hook-b"));
    let base = serving(&demux);
    let url = format!("{base}/hook");
    let body = push_body(REPO_A);

    assert_eq!(
        post_push(&url, SECRET_A, &body, Some("delivery-1")),
        (202, "queued".to_string())
    );
    assert_eq!(
        post_push(&url, SECRET_A, &body, Some("delivery-1")),
        (200, "duplicate delivery".to_string())
    );
    assert_eq!(drain(&chan_a).len(), 1);
    assert_eq!(drain(&chan_b).len(), 1);
}

#[test]
fn dead_route_channel_still_counts_other_deliveries() {
    let demux = Arc::new(SharedWebhook::new());
    let (inj_live, chan_live) = ingress();
    let (inj_dead, chan_dead) = ingress();
    demux.register(REPO_A, route(SECRET_A, &inj_live, "hook-live"));
    demux.register(REPO_A, route(SECRET_A, &inj_dead, "hook-dead"));
    drop(chan_dead); // the dead campaign's supervisor is gone
    let base = serving(&demux);
    let url = format!("{base}/hook");

    let body = push_body(REPO_A);
    // One route delivered → 202 even though the other 503'd internally.
    assert_eq!(
        post_push(&url, SECRET_A, &body, None),
        (202, "queued".to_string())
    );
    assert_eq!(drain(&chan_live).len(), 1);
}

#[test]
fn all_routes_dead_is_503() {
    let demux = Arc::new(SharedWebhook::new());
    let (inj, chan) = ingress();
    demux.register(REPO_A, route(SECRET_A, &inj, "hook"));
    drop(chan);
    let base = serving(&demux);
    let url = format!("{base}/hook");
    let body = push_body(REPO_A);
    assert_eq!(
        post_push(&url, SECRET_A, &body, None),
        (503, "run ended".to_string())
    );
}

// ---------------------------------------------------------------------------
// SharedWebhookDriver — campaign-side registration
// ---------------------------------------------------------------------------

fn ctx_with(secrets: &[(&str, &str)]) -> ExecutionContext {
    ExecutionContext {
        capabilities: std::rc::Rc::new(Map::from_iter([(
            "secrets".to_string(),
            Value::Bool(true),
        )])),
        secrets: std::rc::Rc::new(
            secrets
                .iter()
                .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                .collect(),
        ),
        ..Default::default()
    }
}

fn webhook_node(params: Value) -> Node {
    Node {
        id: "hook".into(),
        driver: "github-webhook".into(),
        params,
        on: vec!["start".into()],
        emits: vec!["hook.listening".into(), "push".into()],
        ..Default::default()
    }
}

#[test]
fn shared_driver_registers_routes_without_binding() {
    let demux = Arc::new(SharedWebhook::new());
    let (trigger_inj, _trigger_chan) = ingress();
    let driver = SharedWebhookDriver::new(trigger_inj, Arc::clone(&demux));
    assert!(driver.is_ingress());
    assert_eq!(driver.name(), "github-webhook");

    let params = json!({
        // listen/path are ignored under the shared listener — the
        // daemon's flags govern the one socket.
        "listen": "127.0.0.1:29999",
        "path": "/ignored",
        "repos": [
            { "name": REPO_A, "secret_env": "WH_A" },
            { "name": REPO_B, "secret_env": "WH_B" },
        ],
    });
    let events = driver
        .invoke(
            &webhook_node(params),
            &[],
            &ctx_with(&[("WH_A", SECRET_A), ("WH_B", SECRET_B)]),
        )
        .unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, "hook.listening");
    assert_eq!(events[0].payload["shared"], true);
    assert_eq!(events[0].payload["repos"], json!([REPO_A, REPO_B]));

    // Routes registered — and NOT yet serving (the driver never binds).
    assert_eq!(demux.route_count(), 2);
    assert_eq!(demux.repos(), vec![REPO_B, REPO_A]); // sorted
    assert!(demux.bound_addr().is_none());
}

#[test]
fn shared_driver_end_to_end_over_http() {
    let demux = Arc::new(SharedWebhook::new());
    let (trigger_inj, trigger_chan) = ingress();
    let driver = SharedWebhookDriver::new(trigger_inj, Arc::clone(&demux));
    let params = json!({
        "repos": [{ "name": REPO_A, "secret_env": "WH_A" }],
    });
    driver
        .invoke(&webhook_node(params), &[], &ctx_with(&[("WH_A", SECRET_A)]))
        .unwrap();
    let base = serving(&demux);
    let url = format!("{base}/hook");

    let body = push_body(REPO_A);
    assert_eq!(
        post_push(&url, SECRET_A, &body, None),
        (202, "queued".to_string())
    );
    let events = drain(&trigger_chan);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, "push");
    assert_eq!(events[0].source, "hook");
}

#[test]
fn serve_twice_and_bad_path_are_errors() {
    let demux = Arc::new(SharedWebhook::new());
    demux.serve("127.0.0.1:0", "/hook").unwrap();
    assert!(demux.serve("127.0.0.1:0", "/hook").is_err());
    demux.stop();
    // After stop the slot cleared — a re-bind works.
    demux.serve("127.0.0.1:0", "/hook").unwrap();
    let bad = SharedWebhook::new();
    assert!(bad.serve("127.0.0.1:0", "no-slash").is_err());
    assert!(bad.serve("127.0.0.1:0", "/{capture}").is_err());
}
