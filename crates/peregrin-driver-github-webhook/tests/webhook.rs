#![allow(clippy::unwrap_used)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::items_after_statements)]
//! `driver: github-webhook` — listener lifecycle, the bupha request
//! contract (ping/405/404/401/dedup/skip/malformed/queued), secret
//! gating, and the ingress channel end to end into a `Runner`.

use std::collections::BTreeMap;
use std::net::TcpListener;
use std::sync::Arc;
use std::time::{Duration, Instant};

use peregrin_core::{Event, Node};
use peregrin_driver::{Driver, DriverError, ExecutionContext, NoopDriver};
use peregrin_driver_github_webhook::{GithubWebhookDriver, signature_header};
use peregrin_engine::{Ingress, Registry, RunOptions, Runner, ingress};
use peregrin_loader::Loader;
use peregrin_loader_yaml::YamlLoader;
use serde_json::{Map, Value, json};

const SECRET: &str = "s3cr3t";
const REPO: &str = "acme/widgets";

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

fn node(params: Value) -> Node {
    Node {
        id: "hook".into(),
        driver: "github-webhook".into(),
        params,
        emits: vec!["hook.listening".into()],
        ..Default::default()
    }
}

fn default_params() -> Value {
    json!({
        "listen": "127.0.0.1:0",
        "repos": [{ "name": REPO, "secret_env": "WH_SECRET" }],
    })
}

fn ctx_with(secrets_cap: Value, secrets: &[(&str, &str)], strict: bool) -> ExecutionContext {
    let mut capabilities = Map::new();
    if !secrets_cap.is_null() {
        capabilities.insert("secrets".to_string(), secrets_cap);
    }
    ExecutionContext {
        capabilities: std::rc::Rc::new(capabilities),
        secrets: std::rc::Rc::new(
            secrets
                .iter()
                .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                .collect::<BTreeMap<_, _>>(),
        ),
        strict,
        ..Default::default()
    }
}

fn default_ctx() -> ExecutionContext {
    ctx_with(Value::Bool(true), &[("WH_SECRET", SECRET)], false)
}

/// Invoke the node and hand back `(driver, ingress, base_url)` — the
/// `base_url` is `http://<bound-addr>`; append the hook path to post.
fn serving(params: Value, ctx: &ExecutionContext) -> (Arc<GithubWebhookDriver>, Ingress, String) {
    let (injector, chan) = ingress();
    let driver = Arc::new(GithubWebhookDriver::new(injector));
    let events = driver.invoke(&node(params), &[], ctx).unwrap();
    assert_eq!(kinds(&events), vec!["hook.listening"]);
    let base = format!("http://{}", driver.bound_addr().unwrap());
    (driver, chan, base)
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

fn push_body() -> Vec<u8> {
    json!({
        "ref": "refs/heads/main",
        "before": "1111111111111111111111111111111111111111",
        "after": "2222222222222222222222222222222222222222",
        "pusher": { "name": "octocat" },
        "sender": { "login": "octocat" },
        "repository": {
            "full_name": REPO,
            "clone_url": "https://github.com/acme/widgets.git",
        },
    })
    .to_string()
    .into_bytes()
}

fn push_headers(secret: &str, body: &[u8]) -> Vec<(String, String)> {
    vec![
        ("x-github-event".to_string(), "push".to_string()),
        (
            "x-hub-signature-256".to_string(),
            signature_header(secret, body),
        ),
    ]
}

fn post_push(url: &str, secret: &str, body: &[u8]) -> (u16, String) {
    let headers = push_headers(secret, body);
    let refs: Vec<(&str, &str)> = headers
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    post(url, &refs, body)
}

fn kinds(events: &[Event]) -> Vec<&str> {
    events.iter().map(|e| e.kind.as_str()).collect()
}

// ---------------------------------------------------------------------------
// Signing (RFC 2104 over sha2 — verified against openssl vectors)
// ---------------------------------------------------------------------------

#[test]
fn hmac_sha256_matches_rfc4231_vectors() {
    // Test Case 2 — key inside the block size.
    assert_eq!(
        signature_header("Jefe", b"what do ya want for nothing?"),
        "sha256=5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
    );
    // Test Case 7 shape — key longer than the 64-byte block takes the
    // hash-then-pad arm (openssl-verified).
    let long_key = "a".repeat(80);
    assert_eq!(
        signature_header(
            &long_key,
            b"Test Using Larger Than Block-Size Key - Hash Key First"
        ),
        "sha256=7502d8b2069f64dcbca4d51628fdc86a17200b3fad268755483946baf3d99fa8"
    );
}

// ---------------------------------------------------------------------------
// Listener lifecycle + the happy push path
// ---------------------------------------------------------------------------

#[test]
fn listening_event_reports_bound_addr_and_verified_push_injects() {
    let (_driver, chan, base) = serving(default_params(), &default_ctx());
    let events_url = format!("{base}/hook");

    let (status, text) = post_push(&events_url, SECRET, &push_body());
    assert_eq!((status, text.as_str()), (202, "queued"));

    let mut drained = Vec::new();
    assert_eq!(chan.drain_into(&mut drained), 1);
    let event = &drained[0];
    assert_eq!(event.kind, "push");
    assert_eq!(event.source, "hook");
    assert_eq!(
        event.payload,
        json!({
            "repo": "acme/widgets",
            "ref": "refs/heads/main",
            "sha": "2222222222222222222222222222222222222222",
            "before": "1111111111111111111111111111111111111111",
            "actor": "octocat",
            "clone_url": "https://github.com/acme/widgets.git",
        })
    );
}

// ---------------------------------------------------------------------------
// pull_request deliveries
// ---------------------------------------------------------------------------

const HEAD_SHA: &str = "3333333333333333333333333333333333333333";

fn pr_body(action: &str) -> Vec<u8> {
    json!({
        "action": action,
        "number": 42,
        "pull_request": {
            "head": {
                "sha": HEAD_SHA,
                "ref": "feature/pr",
                "repo": { "clone_url": "https://github.com/octocat/widgets.git" },
            },
            "base": { "ref": "main" },
            "user": { "login": "octocat" },
        },
        "sender": { "login": "hubot" },
        "repository": {
            "full_name": REPO,
            "clone_url": "https://github.com/acme/widgets.git",
        },
    })
    .to_string()
    .into_bytes()
}

fn post_pr(url: &str, secret: &str, body: &[u8], delivery: Option<&str>) -> (u16, String) {
    let mut headers = vec![
        ("x-github-event".to_string(), "pull_request".to_string()),
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

#[test]
fn verified_pull_request_injects_head_payload() {
    let (_driver, chan, base) = serving(default_params(), &default_ctx());
    let url = format!("{base}/hook");
    let (status, text) = post_pr(&url, SECRET, &pr_body("opened"), None);
    assert_eq!((status, text.as_str()), (202, "queued"));

    let mut drained = Vec::new();
    assert_eq!(chan.drain_into(&mut drained), 1);
    let event = &drained[0];
    assert_eq!(event.kind, "pull_request");
    assert_eq!(event.source, "hook");
    assert_eq!(
        event.payload,
        json!({
            "repo": "acme/widgets",
            "number": 42,
            "action": "opened",
            "sha": HEAD_SHA,
            "ref": "feature/pr",
            "base_ref": "main",
            // sender.login wins over pull_request.user.login.
            "actor": "hubot",
            // The head repo's clone_url (fork-aware) wins over the
            // base repository's.
            "clone_url": "https://github.com/octocat/widgets.git",
        })
    );
}

#[test]
fn pull_request_actions_param_filters_per_route() {
    // Only `opened` accepted — `synchronize` verifies but is a no-op.
    let params = json!({
        "listen": "127.0.0.1:0",
        "pull_request_actions": "opened",
        "repos": [{ "name": REPO, "secret_env": "WH_SECRET" }],
    });
    let (_driver, chan, base) = serving(params, &default_ctx());
    let url = format!("{base}/hook");
    assert_eq!(
        post_pr(&url, SECRET, &pr_body("synchronize"), None),
        (200, "ignored: no-op event".to_string())
    );
    assert_eq!(post_pr(&url, SECRET, &pr_body("opened"), None).0, 202);
    let mut drained = Vec::new();
    assert_eq!(chan.drain_into(&mut drained), 1);
    assert_eq!(drained[0].kind, "pull_request");

    // List form + whitespace/empties tolerated; default set covers
    // synchronize/reopened when the param is absent.
    let params = json!({
        "listen": "127.0.0.1:0",
        "pull_request_actions": [" closed ", "", "labeled"],
        "repos": [{ "name": REPO, "secret_env": "WH_SECRET" }],
    });
    let (_driver2, chan2, base2) = serving(params, &default_ctx());
    let url2 = format!("{base2}/hook");
    assert_eq!(post_pr(&url2, SECRET, &pr_body("labeled"), None).0, 202);
    assert_eq!(
        post_pr(&url2, SECRET, &pr_body("opened"), None),
        (200, "ignored: no-op event".to_string())
    );
    let mut drained = Vec::new();
    assert_eq!(chan2.drain_into(&mut drained), 1);

    // Absent param → the default set: synchronize lands, closed doesn't.
    let (_driver3, chan3, base3) = serving(default_params(), &default_ctx());
    let url3 = format!("{base3}/hook");
    assert_eq!(post_pr(&url3, SECRET, &pr_body("synchronize"), None).0, 202);
    assert_eq!(
        post_pr(&url3, SECRET, &pr_body("closed"), None),
        (200, "ignored: no-op event".to_string())
    );
    let mut drained = Vec::new();
    assert_eq!(chan3.drain_into(&mut drained), 1);
}

#[test]
fn pull_request_verify_dedup_and_malformed() {
    let (_driver, chan, base) = serving(default_params(), &default_ctx());
    let url = format!("{base}/hook");
    let body = pr_body("opened");
    // Same HMAC gate as push.
    assert_eq!(post_pr(&url, "wrong-secret", &body, None).0, 401);
    // Delivery-id dedup spans event kinds.
    assert_eq!(
        post_pr(&url, SECRET, &body, Some("pr-delivery-1")),
        (202, "queued".to_string())
    );
    assert_eq!(
        post_pr(&url, SECRET, &body, Some("pr-delivery-1")),
        (200, "duplicate delivery".to_string())
    );
    // Malformed: missing action / number / head sha, unsafe sha.
    for (mut payload, needle) in [
        (
            json!({"number": 1, "pull_request": {"head": {"sha": HEAD_SHA}}}),
            "missing action",
        ),
        (
            json!({"action": "opened", "pull_request": {"head": {"sha": HEAD_SHA}}}),
            "missing number",
        ),
        (
            json!({"action": "opened", "number": 1, "pull_request": {"head": {"sha": "../x"}}}),
            "unsafe sha",
        ),
    ] {
        payload["repository"] = json!({"full_name": REPO});
        let raw = payload.to_string().into_bytes();
        let (status, text) = post_pr(&url, SECRET, &raw, None);
        assert_eq!(
            (status, text.as_str()),
            (400, "unparseable payload"),
            "{needle}"
        );
    }
    let mut drained = Vec::new();
    assert_eq!(chan.drain_into(&mut drained), 1, "only the first landed");
}

#[test]
fn pull_request_clone_url_falls_back_like_push() {
    // No head.repo.clone_url and no repository.clone_url → the route's
    // api_base web base derives it.
    let params = json!({
        "listen": "127.0.0.1:0",
        "api_base": "https://ghe.acme/api/v3",
        "repos": [{ "name": REPO, "secret_env": "WH_SECRET" }],
    });
    let (_driver, chan, base) = serving(params, &default_ctx());
    let body = json!({
        "action": "opened",
        "number": 7,
        "pull_request": { "head": { "sha": HEAD_SHA, "ref": "x" }, "base": { "ref": "main" } },
        "repository": { "full_name": REPO },
    })
    .to_string()
    .into_bytes();
    assert_eq!(post_pr(&format!("{base}/hook"), SECRET, &body, None).0, 202);
    let mut drained = Vec::new();
    assert_eq!(chan.drain_into(&mut drained), 1);
    assert_eq!(
        drained[0].payload["clone_url"],
        "https://ghe.acme/acme/widgets.git"
    );
    // actor falls back to pull_request.user.login when sender is absent.
    let body = json!({
        "action": "reopened",
        "number": 7,
        "pull_request": {
            "head": { "sha": HEAD_SHA, "ref": "x" },
            "base": { "ref": "main" },
            "user": { "login": "octocat" },
        },
        "repository": { "full_name": REPO },
    })
    .to_string()
    .into_bytes();
    assert_eq!(post_pr(&format!("{base}/hook"), SECRET, &body, None).0, 202);
    let mut drained = Vec::new();
    assert_eq!(chan.drain_into(&mut drained), 1);
    assert_eq!(drained[0].payload["actor"], "octocat");
}

#[test]
fn listening_payload_and_driver_metadata() {
    let (injector, _chan) = ingress();
    let driver = GithubWebhookDriver::new(injector);
    assert_eq!(driver.name(), "github-webhook");
    assert!(driver.input().is_none());
    assert_eq!(driver.output().len(), 1);
    assert_eq!(driver.output()[0].kind, "node.failed");
    assert!(driver.bound_addr().is_none());

    let params = json!({
        "listen": "127.0.0.1:0",
        "path": "/wh",
        "api_base": "https://ghe.acme/api/v3",
        "repos": [{ "name": REPO, "secret_env": "WH_SECRET" }],
    });
    let events = driver.invoke(&node(params), &[], &default_ctx()).unwrap();
    assert_eq!(kinds(&events), vec!["hook.listening"]);
    let payload = &events[0].payload;
    assert_eq!(payload["path"], "/wh");
    assert_eq!(payload["repos"], json!(["acme/widgets"]));
    assert_eq!(payload["api_base"], "https://ghe.acme/api/v3");
    // `listen` carries the resolved ephemeral port.
    assert_ne!(payload["listen"], "127.0.0.1:0");
    assert!(
        payload["listen"]
            .as_str()
            .unwrap()
            .starts_with("127.0.0.1:")
    );
}

#[test]
fn pushes_preserve_injection_order() {
    let (_driver, chan, base) = serving(default_params(), &default_ctx());
    let url = format!("{base}/hook");
    let body_a = json!({
        "ref": "refs/heads/main",
        "before": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "after": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        "repository": { "full_name": REPO },
    })
    .to_string()
    .into_bytes();
    let body_b = json!({
        "ref": "refs/heads/main",
        "before": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        "after": "cccccccccccccccccccccccccccccccccccccccc",
        "repository": { "full_name": REPO },
    })
    .to_string()
    .into_bytes();
    assert_eq!(post_push(&url, SECRET, &body_a).0, 202);
    assert_eq!(post_push(&url, SECRET, &body_b).0, 202);
    let mut drained = Vec::new();
    assert_eq!(chan.drain_into(&mut drained), 2);
    assert_eq!(
        drained[0].payload["sha"],
        "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
    );
    assert_eq!(
        drained[1].payload["sha"],
        "cccccccccccccccccccccccccccccccccccccccc"
    );
}

// ---------------------------------------------------------------------------
// The bupha request contract
// ---------------------------------------------------------------------------

#[test]
fn ping_is_acked_without_verifying_or_injecting() {
    let (_driver, chan, base) = serving(default_params(), &default_ctx());
    // Deliberately unsigned — bupha acks probes before verify.
    let (status, text) = post(
        &format!("{base}/hook"),
        &[("x-github-event", "ping")],
        b"{}",
    );
    assert_eq!((status, text.as_str()), (200, "pong"));
    let mut drained = Vec::new();
    assert_eq!(chan.drain_into(&mut drained), 0);
}

#[test]
fn non_push_events_and_missing_event_header_rejected() {
    let (_driver, _chan, base) = serving(default_params(), &default_ctx());
    let url = format!("{base}/hook");
    let (status, text) = post(&url, &[("x-github-event", "issues")], b"{}");
    assert_eq!((status, text.as_str()), (400, "unsupported event"));
    let (status, text) = post(&url, &[], b"{}");
    assert_eq!((status, text.as_str()), (400, "unsupported event"));
}

#[test]
fn unwatched_repo_and_unparseable_body_404() {
    let (_driver, _chan, base) = serving(default_params(), &default_ctx());
    let url = format!("{base}/hook");
    let foreign = json!({
        "ref": "refs/heads/main",
        "after": "2222222222222222222222222222222222222222",
        "repository": { "full_name": "other/repo" },
    })
    .to_string()
    .into_bytes();
    assert_eq!(
        post_push(&url, SECRET, &foreign),
        (404, "unwatched repo".to_string())
    );
    // Not JSON at all — the repo hint itself fails.
    let body = b"not json".to_vec();
    assert_eq!(
        post_push(&url, SECRET, &body),
        (404, "unwatched repo".to_string())
    );
    // JSON but no repository.full_name.
    let body = json!({"ref": "x"}).to_string().into_bytes();
    assert_eq!(
        post_push(&url, SECRET, &body),
        (404, "unwatched repo".to_string())
    );
}

#[test]
fn signature_failures_all_401() {
    let (_driver, _chan, base) = serving(default_params(), &default_ctx());
    let url = format!("{base}/hook");
    let body = push_body();
    // Wrong secret.
    assert_eq!(post_push(&url, "wrong", &body).0, 401);
    // Missing signature header.
    let (status, _) = post(&url, &[("x-github-event", "push")], &body);
    assert_eq!(status, 401);
    // Wrong algorithm prefix.
    let (status, _) = post(
        &url,
        &[
            ("x-github-event", "push"),
            ("x-hub-signature-256", "sha1=deadbeef"),
        ],
        &body,
    );
    assert_eq!(status, 401);
    // Bad hex.
    let (status, _) = post(
        &url,
        &[
            ("x-github-event", "push"),
            ("x-hub-signature-256", "sha256=zzzz"),
        ],
        &body,
    );
    assert_eq!(status, 401);
    // Well-formed hex, wrong length.
    let (status, _) = post(
        &url,
        &[
            ("x-github-event", "push"),
            ("x-hub-signature-256", "sha256=abcd"),
        ],
        &body,
    );
    assert_eq!(status, 401);
}

#[test]
fn duplicate_delivery_is_acked_without_reinjecting() {
    let (_driver, chan, base) = serving(default_params(), &default_ctx());
    let url = format!("{base}/hook");
    let body = push_body();
    let sig = signature_header(SECRET, &body);
    let headers = [
        ("x-github-event", "push"),
        ("x-hub-signature-256", sig.as_str()),
        ("x-github-delivery", "delivery-1"),
    ];
    assert_eq!(post(&url, &headers, &body).0, 202);
    assert_eq!(
        post(&url, &headers, &body),
        (200, "duplicate delivery".to_string())
    );
    let mut drained = Vec::new();
    assert_eq!(chan.drain_into(&mut drained), 1);
}

#[test]
fn deleted_and_zero_sha_pushes_skip() {
    let (_driver, chan, base) = serving(default_params(), &default_ctx());
    let url = format!("{base}/hook");
    let deleted = json!({
        "ref": "refs/heads/main",
        "after": "2222222222222222222222222222222222222222",
        "deleted": true,
        "repository": { "full_name": REPO },
    })
    .to_string()
    .into_bytes();
    assert_eq!(
        post_push(&url, SECRET, &deleted),
        (200, "ignored: no-op event".to_string())
    );
    let zero = json!({
        "ref": "refs/heads/main",
        "after": "0000000000000000000000000000000000000000",
        "deleted": false,
        "repository": { "full_name": REPO },
    })
    .to_string()
    .into_bytes();
    assert_eq!(
        post_push(&url, SECRET, &zero),
        (200, "ignored: no-op event".to_string())
    );
    let mut drained = Vec::new();
    assert_eq!(chan.drain_into(&mut drained), 0);
}

#[test]
fn malformed_signed_payloads_400() {
    let (_driver, _chan, base) = serving(default_params(), &default_ctx());
    let url = format!("{base}/hook");
    // Signed correctly but missing `ref`.
    let no_ref = json!({
        "after": "2222222222222222222222222222222222222222",
        "repository": { "full_name": REPO },
    })
    .to_string()
    .into_bytes();
    assert_eq!(
        post_push(&url, SECRET, &no_ref),
        (400, "unparseable payload".to_string())
    );
    // Missing `after`.
    let no_after = json!({
        "ref": "refs/heads/main",
        "repository": { "full_name": REPO },
    })
    .to_string()
    .into_bytes();
    assert_eq!(
        post_push(&url, SECRET, &no_after),
        (400, "unparseable payload".to_string())
    );
    // `after` present but not revision-safe — each unsafe shape is a
    // malformed arm, not a skipped push.
    for sha in [
        "abc",                                      // too short
        "-222222222222222222222222222222222222222", // leading dash
        "22..2222",                                 // range traversal
        "2222/222",                                 // path separator
        "22222222222222222222222222222222!22222",   // bad char
    ] {
        let body = json!({
            "ref": "refs/heads/main",
            "after": sha,
            "repository": { "full_name": REPO },
        })
        .to_string()
        .into_bytes();
        assert_eq!(post_push(&url, SECRET, &body).0, 400, "sha {sha}");
    }
}

#[test]
fn wrong_path_404_and_get_405_and_oversize_413() {
    let (_driver, _chan, base) = serving(default_params(), &default_ctx());
    // Anything but the hook path.
    let (status, _) = post(&format!("{base}/other"), &[], b"{}");
    assert_eq!(status, 404);
    // Right path, wrong method.
    let resp = client().get(format!("{base}/hook")).call().unwrap();
    assert_eq!(resp.status().as_u16(), 405);
    // Over the 2 MiB cap.
    let body = vec![b'x'; 2 * 1024 * 1024 + 1];
    let (status, _) = post(
        &format!("{base}/hook"),
        &[("x-github-event", "push")],
        &body,
    );
    assert_eq!(status, 413);
}

#[test]
fn inject_after_run_end_is_503() {
    let (injector, chan) = ingress();
    let driver = GithubWebhookDriver::new(injector);
    driver
        .invoke(&node(default_params()), &[], &default_ctx())
        .unwrap();
    let base = format!("http://{}", driver.bound_addr().unwrap());
    drop(chan); // the run's receiver is gone
    assert_eq!(
        post_push(&format!("{base}/hook"), SECRET, &push_body()).0,
        503
    );
}

#[test]
fn refire_replaces_the_listener() {
    let (injector, _chan) = ingress();
    let driver = GithubWebhookDriver::new(injector);
    driver
        .invoke(&node(default_params()), &[], &default_ctx())
        .unwrap();
    let first = driver.bound_addr().unwrap();
    driver
        .invoke(&node(default_params()), &[], &default_ctx())
        .unwrap();
    let second = driver.bound_addr().unwrap();
    assert_ne!(first, second);
    // The old listener is gone — the old port refuses.
    let resp = client().post(format!("http://{first}/hook")).send(b"{}");
    assert!(resp.is_err() || !resp.unwrap().status().is_success());
}

#[test]
fn custom_path_is_honored() {
    let params = json!({
        "listen": "127.0.0.1:0",
        "path": "/gh/hook",
        "repos": [{ "name": REPO, "secret_env": "WH_SECRET" }],
    });
    let (_driver, _chan, base) = serving(params, &default_ctx());
    assert_eq!(
        post_push(&format!("{base}/gh/hook"), SECRET, &push_body()).0,
        202
    );
    // The default path now 404s.
    assert_eq!(post(&format!("{base}/hook"), &[], b"{}").0, 404);
}

#[test]
fn clone_url_falls_back_to_api_base_web_root() {
    // Default api_base → github.com.
    let (_driver, chan, base) = serving(default_params(), &default_ctx());
    let bare = json!({
        "ref": "refs/heads/main",
        "after": "2222222222222222222222222222222222222222",
        "repository": { "full_name": REPO },
    })
    .to_string()
    .into_bytes();
    assert_eq!(post_push(&format!("{base}/hook"), SECRET, &bare).0, 202);
    let mut drained = Vec::new();
    assert_eq!(chan.drain_into(&mut drained), 1);
    assert_eq!(
        drained[0].payload["clone_url"],
        "https://github.com/acme/widgets.git"
    );
    assert_eq!(drained[0].payload["before"], "");
    assert_eq!(drained[0].payload["actor"], "");

    // GHE `/api/v3` suffix strips to the host root.
    let params = json!({
        "listen": "127.0.0.1:0",
        "api_base": "https://ghe.acme/api/v3/",
        "repos": [{ "name": REPO, "secret_env": "WH_SECRET" }],
    });
    let (_driver2, chan2, base2) = serving(params, &default_ctx());
    assert_eq!(post_push(&format!("{base2}/hook"), SECRET, &bare).0, 202);
    let mut drained = Vec::new();
    assert_eq!(chan2.drain_into(&mut drained), 1);
    assert_eq!(
        drained[0].payload["clone_url"],
        "https://ghe.acme/acme/widgets.git"
    );
}

#[test]
fn multi_repo_secrets_verify_independently() {
    let params = json!({
        "listen": "127.0.0.1:0",
        "repos": [
            { "name": "acme/widgets", "secret_env": "WH_A" },
            { "name": "acme/gadgets", "secret_env": "WH_B" },
        ],
    });
    let ctx = ctx_with(
        Value::Bool(true),
        &[("WH_A", "secret-a"), ("WH_B", "secret-b")],
        false,
    );
    let (_driver, _chan, base) = serving(params, &ctx);
    let url = format!("{base}/hook");
    let body = |repo: &str| {
        json!({
            "ref": "refs/heads/main",
            "after": "2222222222222222222222222222222222222222",
            "repository": { "full_name": repo },
        })
        .to_string()
        .into_bytes()
    };
    assert_eq!(post_push(&url, "secret-a", &body(REPO)).0, 202);
    assert_eq!(post_push(&url, "secret-b", &body("acme/gadgets")).0, 202);
    // Cross-signed — A's payload with B's secret.
    assert_eq!(post_push(&url, "secret-b", &body(REPO)).0, 401);
}

// ---------------------------------------------------------------------------
// Param validation + secret gating
// ---------------------------------------------------------------------------

fn invoke_err(params: Value, ctx: &ExecutionContext) -> DriverError {
    let (injector, _chan) = ingress();
    GithubWebhookDriver::new(injector)
        .invoke(&node(params), &[], ctx)
        .unwrap_err()
}

#[test]
fn params_validation_errors() {
    let ctx = default_ctx();
    for (params, needle) in [
        (json!({}), "missing 'params.repos'"),
        (json!({"repos": []}), "missing 'params.repos'"),
        (json!({"repos": "x"}), "missing 'params.repos'"),
        (json!({"repos": [{}]}), "non-empty 'name'"),
        (json!({"repos": [{"name": ""}]}), "non-empty 'name'"),
        (json!({"repos": [{"name": "o/r"}]}), "missing 'secret_env'"),
        (
            json!({"repos": [
                {"name": "o/r", "secret_env": "S"},
                {"name": "o/r", "secret_env": "S"},
            ]}),
            "duplicate repo 'o/r'",
        ),
        (
            json!({"path": "no-slash", "repos": [{"name": "o/r", "secret_env": "S"}]}),
            "params.path",
        ),
        (
            json!({"path": "/x/:id", "repos": [{"name": "o/r", "secret_env": "S"}]}),
            "params.path",
        ),
    ] {
        let err = invoke_err(params, &ctx);
        assert!(err.reason.contains(needle), "{} !~ {needle}", err.reason);
    }
}

#[test]
fn secret_gate_denials() {
    let params = default_params();
    // No `capabilities.secrets` at all.
    let ctx = ctx_with(Value::Null, &[("WH_SECRET", SECRET)], false);
    assert!(
        invoke_err(params.clone(), &ctx)
            .reason
            .contains("capability 'secrets'")
    );
    // Explicit false.
    let ctx = ctx_with(Value::Bool(false), &[("WH_SECRET", SECRET)], false);
    assert!(
        invoke_err(params.clone(), &ctx)
            .reason
            .contains("capability 'secrets'")
    );
    // Strict + grant the name doesn't match (list).
    let ctx = ctx_with(json!(["OTHER_*"]), &[("WH_SECRET", SECRET)], true);
    assert!(
        invoke_err(params.clone(), &ctx)
            .reason
            .contains("WH_SECRET")
    );
    // Strict + scalar grant.
    let ctx = ctx_with(json!(42), &[("WH_SECRET", SECRET)], true);
    assert!(
        invoke_err(params.clone(), &ctx)
            .reason
            .contains("capability 'secrets'")
    );
    // Strict + {allow, deny} with deny winning.
    let ctx = ctx_with(
        json!({"allow": ["WH_*"], "deny": ["WH_SECRET"]}),
        &[("WH_SECRET", SECRET)],
        true,
    );
    assert!(
        invoke_err(params.clone(), &ctx)
            .reason
            .contains("capability 'secrets'")
    );
    // Strict + allow map with an empty allow list.
    let ctx = ctx_with(json!({"allow": []}), &[("WH_SECRET", SECRET)], true);
    assert!(
        invoke_err(params.clone(), &ctx)
            .reason
            .contains("capability 'secrets'")
    );
    // Granted but the named secret is absent → strict-mode miss +
    // unresolved error.
    let ctx = ctx_with(Value::Bool(true), &[], true);
    let err = invoke_err(params, &ctx);
    assert!(err.reason.contains("secret 'WH_SECRET' unresolved"));
}

#[test]
fn secret_gate_grants() {
    // Strict + `secrets: true`.
    let ctx = ctx_with(Value::Bool(true), &[("WH_SECRET", SECRET)], true);
    let (_d, _c, base) = serving(default_params(), &ctx);
    assert_eq!(
        post_push(&format!("{base}/hook"), SECRET, &push_body()).0,
        202
    );
    // Strict + matching glob list.
    let ctx = ctx_with(json!(["WH_*"]), &[("WH_SECRET", SECRET)], true);
    let (_d, _c, base) = serving(default_params(), &ctx);
    assert_eq!(
        post_push(&format!("{base}/hook"), SECRET, &push_body()).0,
        202
    );
    // Strict + {allow} glob.
    let ctx = ctx_with(json!({"allow": ["WH_*"]}), &[("WH_SECRET", SECRET)], true);
    let (_d, _c, base) = serving(default_params(), &ctx);
    assert_eq!(
        post_push(&format!("{base}/hook"), SECRET, &push_body()).0,
        202
    );
    // Non-strict: presence of a non-false grant is enough even when
    // the name doesn't match the list (mirrors `${secret.*}`).
    let ctx = ctx_with(json!(["OTHER"]), &[("WH_SECRET", SECRET)], false);
    let (_d, _c, base) = serving(default_params(), &ctx);
    assert_eq!(
        post_push(&format!("{base}/hook"), SECRET, &push_body()).0,
        202
    );
}

#[test]
fn glob_match_edge_arms_via_the_gate() {
    // `?` does not match line terminators.
    let params = json!({
        "listen": "127.0.0.1:0",
        "repos": [{ "name": REPO, "secret_env": "WH\n" }],
    });
    let ctx = ctx_with(json!(["WH?"]), &[("WH\n", SECRET)], true);
    assert!(invoke_err(params, &ctx).reason.contains("capability"));
    // `*` does not cross line terminators on the backtrack arm.
    let params = json!({
        "listen": "127.0.0.1:0",
        "repos": [{ "name": REPO, "secret_env": "WH\nX" }],
    });
    let ctx = ctx_with(json!(["WH*X"]), &[("WH\nX", SECRET)], true);
    assert!(invoke_err(params, &ctx).reason.contains("capability"));
    // `?` matches ordinary chars; trailing `*` eats the rest.
    let params = json!({
        "listen": "127.0.0.1:0",
        "repos": [{ "name": REPO, "secret_env": "WH_A" }],
    });
    let ctx = ctx_with(json!(["WH_?"]), &[("WH_A", SECRET)], true);
    let (_d, _c, base) = serving(params, &ctx);
    assert_eq!(
        post_push(&format!("{base}/hook"), SECRET, &push_body()).0,
        202
    );
    // A trailing `*` matches the empty tail — `WH` against `WH*`
    // exercises the tail-skip loop (the star never resumes).
    let params = json!({
        "listen": "127.0.0.1:0",
        "repos": [{ "name": REPO, "secret_env": "WH" }],
    });
    let ctx = ctx_with(json!(["WH*"]), &[("WH", SECRET)], true);
    let (_d, _c, base) = serving(params, &ctx);
    assert_eq!(
        post_push(&format!("{base}/hook"), SECRET, &push_body()).0,
        202
    );
    // An exact pattern shorter than the name fails outright (no star
    // to backtrack through).
    let params = json!({
        "listen": "127.0.0.1:0",
        "repos": [{ "name": REPO, "secret_env": "WHAT" }],
    });
    let ctx = ctx_with(json!(["WH"]), &[("WHAT", SECRET)], true);
    assert!(invoke_err(params, &ctx).reason.contains("capability"));
}

// ---------------------------------------------------------------------------
// Bind/dry-run/param-resolve edges
// ---------------------------------------------------------------------------

#[test]
fn bind_failure_reports_node_failed() {
    // Hold the port so the driver's bind collides.
    let blocker = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = blocker.local_addr().unwrap();
    let params = json!({
        "listen": addr.to_string(),
        "repos": [{ "name": REPO, "secret_env": "WH_SECRET" }],
    });
    let (injector, _chan) = ingress();
    let driver = GithubWebhookDriver::new(injector);
    let events = driver.invoke(&node(params), &[], &default_ctx()).unwrap();
    assert_eq!(kinds(&events), vec!["node.failed"]);
    assert!(
        events[0].payload["error"]
            .as_str()
            .unwrap()
            .contains("failed to bind")
    );
    assert!(driver.bound_addr().is_none());
}

#[test]
fn dry_run_reports_listening_without_binding() {
    let (injector, _chan) = ingress();
    let driver = GithubWebhookDriver::new(injector);
    let mut ctx = default_ctx();
    ctx.dry_run = true;
    // No secrets populated — dry_run returns before the gate.
    let ctx = ExecutionContext {
        dry_run: true,
        ..ctx
    };
    let events = driver.invoke(&node(default_params()), &[], &ctx).unwrap();
    assert_eq!(kinds(&events), vec!["hook.listening"]);
    assert_eq!(events[0].payload["dry_run"], true);
    assert_eq!(events[0].payload["listen"], "127.0.0.1:0");
    assert_eq!(events[0].payload["repos"], json!(["acme/widgets"]));
    assert!(driver.bound_addr().is_none());
}

#[test]
fn strict_mode_unresolved_template_fails() {
    let ctx = ctx_with(Value::Bool(true), &[("WH_SECRET", SECRET)], true);
    let params = json!({
        "listen": "${config.nope}",
        "repos": [{ "name": REPO, "secret_env": "WH_SECRET" }],
    });
    assert_ne!(invoke_err(params, &ctx).reason, "");
}

#[test]
fn str_param_empty_and_nonstring_fall_back() {
    // `path: ""` and non-string fall back to the `/hook` default.
    let params = json!({
        "listen": "127.0.0.1:0",
        "path": "",
        "api_base": 42,
        "repos": [{ "name": REPO, "secret_env": "WH_SECRET" }],
    });
    let (_d, _c, base) = serving(params, &default_ctx());
    assert_eq!(
        post_push(&format!("{base}/hook"), SECRET, &push_body()).0,
        202
    );
}

#[test]
fn params_resolve_against_state_scope() {
    // `${state.*}` binds the node's state store via the state_view
    // closure — the same re-resolve surface the other impls carry.
    let ctx = default_ctx();
    ctx.state
        .borrow_mut()
        .set("addr", Value::String("127.0.0.1:0".to_string()));
    let params = json!({
        "listen": "${state.addr}",
        "repos": [{ "name": REPO, "secret_env": "WH_SECRET" }],
    });
    let (_d, _c, base) = serving(params, &ctx);
    assert_eq!(
        post_push(&format!("{base}/hook"), SECRET, &push_body()).0,
        202
    );
}

// ---------------------------------------------------------------------------
// End-to-end: injected push reaches a real Runner via with_ingress
// ---------------------------------------------------------------------------

/// A driver that sleeps — holds the run open so the POST lands
/// mid-run deterministically (the engine ingress tests' pattern).
struct SlowDriver;

impl Driver for SlowDriver {
    fn name(&self) -> &'static str {
        "slow"
    }

    fn invoke(
        &self,
        node: &Node,
        _triggers: &[Event],
        _ctx: &ExecutionContext,
    ) -> Result<Vec<Event>, DriverError> {
        std::thread::sleep(Duration::from_millis(600));
        Ok(vec![Event::new(
            node.emits
                .first()
                .cloned()
                .unwrap_or_else(|| format!("{}.done", node.id)),
            json!({}),
            &node.id,
        )])
    }
}

#[test]
fn push_reaches_the_runner_through_with_ingress() {
    let (injector, chan) = ingress();
    let driver = Arc::new(GithubWebhookDriver::new(injector.clone()));
    let registry = Registry::default()
        .with("github-webhook", driver.clone())
        .with("noop", Arc::new(NoopDriver))
        .with("slow", Arc::new(SlowDriver));
    let campaign = YamlLoader
        .load_str(
            "
            capabilities:
              secrets: true
            nodes:
              - id: hook
                driver: github-webhook
                on: [start]
                emits: [hook.listening]
                params:
                  listen: '127.0.0.1:0'
                  repos:
                    - { name: 'acme/widgets', secret_env: WH_SECRET }
              - id: slow
                driver: slow
                on: [hook.listening]
                emits: [slow.done]
              - id: build
                driver: noop
                on: [push]
                emits: [build.done]
            ",
            std::path::Path::new("."),
        )
        .unwrap();

    let mut runner = Runner::new(registry).with_ingress(chan);
    let poster = driver.clone();
    let log = std::thread::scope(|scope| {
        scope.spawn(move || {
            // The listener binds during `hook`'s invoke; `slow` (which
            // fires on hook.listening) holds the run open past the
            // POST so the drain picks the push up next iteration.
            let deadline = Instant::now() + Duration::from_secs(10);
            let addr = loop {
                if let Some(addr) = poster.bound_addr() {
                    break addr;
                }
                assert!(Instant::now() < deadline, "listener never bound");
                std::thread::sleep(Duration::from_millis(5));
            };
            let (status, text) = post_push(&format!("http://{addr}/hook"), SECRET, &push_body());
            assert_eq!((status, text.as_str()), (202, "queued"));
            // The campaign declares an ingress driver → the run serves;
            // cancel is the in-band exit. `slow` (600ms) holds the run
            // thread, so push+cancel must land in *separate* drains —
            // cancel inside `slow`'s sleep window would ride the same
            // batch and starve `build` before its ready step.
            std::thread::sleep(Duration::from_millis(750));
            injector.inject(Event::cancel("test done")).unwrap();
        });
        runner
            .run(
                &campaign,
                &RunOptions {
                    secrets: BTreeMap::from([("WH_SECRET".to_string(), SECRET.to_string())]),
                    ..Default::default()
                },
            )
            .unwrap()
    });

    let push = log.iter().find(|e| e.kind == "push").unwrap();
    assert_eq!(push.source, "hook");
    assert_eq!(
        push.payload["sha"],
        "2222222222222222222222222222222222222222"
    );
    assert_eq!(
        push.payload["clone_url"],
        "https://github.com/acme/widgets.git"
    );
    let kinds = kinds(&log);
    assert!(kinds.contains(&"hook.listening"));
    // The `build` node (noop) fired on the injected push — noop's
    // done event is literally `node.done`.
    assert!(kinds.contains(&"node.done"), "{}", kinds.join(", "));
    assert_eq!(log.last().unwrap().kind, "workflow.cancelled");
}
