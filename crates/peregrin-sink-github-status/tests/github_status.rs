#![allow(clippy::unwrap_used)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::assert_is_empty)]

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use peregrin_driver::{Driver, ExecutionContext};
use peregrin_sink_github_status::GithubStatusSink;
use serde_json::{Map, Value, json};

// ---------------------------------------------------------------------------
// Fixture helpers — the one-shot HTTP stub from peregrin-driver-http's
// tests: accept a connection, read the whole request, hand it back over
// a channel, write `response`.
// ---------------------------------------------------------------------------

fn node(params: Value) -> peregrin_core::Node {
    peregrin_core::Node {
        id: "n".into(),
        driver: "github-status".into(),
        params,
        ..Default::default()
    }
}

fn trigger(kind: &str, payload: Value) -> peregrin_core::Event {
    peregrin_core::Event::new(kind, payload, "build")
}

/// A ctx carrying the `secrets` capability grant and a populated
/// secrets record — the minimum for `resolve_token` to pass.
fn ctx_with_token() -> ExecutionContext {
    ctx_with(Value::Bool(true), &[("GITHUB_TOKEN", "tok123")])
}

fn ctx_with(secrets_cap: Value, secrets: &[(&str, &str)]) -> ExecutionContext {
    ExecutionContext {
        capabilities: std::rc::Rc::new(Map::from_iter([("secrets".to_string(), secrets_cap)])),
        secrets: std::rc::Rc::new(
            secrets
                .iter()
                .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                .collect::<BTreeMap<_, _>>(),
        ),
        ..Default::default()
    }
}

struct Stub {
    url: String,
    request: mpsc::Receiver<String>,
    join: thread::JoinHandle<()>,
}

fn request_complete(buf: &[u8]) -> bool {
    let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") else {
        return false;
    };
    let head = String::from_utf8_lossy(&buf[..pos]);
    let len = head
        .lines()
        .find_map(|line| {
            line.to_ascii_lowercase()
                .strip_prefix("content-length:")
                .map(str::trim)
                .and_then(|v| v.parse::<usize>().ok())
        })
        .unwrap_or(0);
    buf.len() >= pos + 4 + len
}

fn serve(response: &'static str) -> Stub {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let (tx, rx) = mpsc::channel();
    let join = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut buf = Vec::new();
        let mut tmp = [0_u8; 8192];
        loop {
            match stream.read(&mut tmp) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    buf.extend_from_slice(&tmp[..n]);
                    if request_complete(&buf) {
                        break;
                    }
                }
            }
        }
        let _ = tx.send(String::from_utf8_lossy(&buf).into_owned());
        let _ = stream.write_all(response.as_bytes());
        let _ = stream.flush();
    });
    Stub {
        url,
        request: rx,
        join,
    }
}

impl Stub {
    fn captured(self) -> String {
        let request = self.request.recv_timeout(Duration::from_secs(5)).unwrap();
        let _ = self.join.join();
        request
    }
}

/// params with `api_base` pointed at `stub` — every live-post test
/// goes through it (the real api.github.com is never touched).
fn base_params(stub_url: &str) -> Value {
    json!({
        "repo": "o/r",
        "sha": "abc123",
        "api_base": stub_url,
    })
}

fn invoke(
    node: &peregrin_core::Node,
    triggers: &[peregrin_core::Event],
    ctx: &ExecutionContext,
) -> Vec<peregrin_core::Event> {
    GithubStatusSink::new().invoke(node, triggers, ctx).unwrap()
}

const CREATED: &str = "HTTP/1.1 201 Created\r\ncontent-type: application/json\r\ncontent-length: 9\r\n\r\n{\"id\":42}";

// ---------------------------------------------------------------------------
// Naming / contracts / param validation
// ---------------------------------------------------------------------------

#[test]
fn name_and_schemas() {
    let sink = GithubStatusSink::new();
    let _ = GithubStatusSink::default();
    assert_eq!(sink.name(), "github-status");
    let input = sink.input().unwrap();
    assert_eq!(input.fields["repo"], "string");
    assert_eq!(input.fields["sha"], "string");
    assert_eq!(input.fields["state"], "string?");
    assert_eq!(sink.output()[0].kind, "node.failed");

    // The effects seam — a sink over explicit SystemEffects.
    let custom =
        GithubStatusSink::with_effects(std::sync::Arc::new(peregrin_template::SystemEffects));
    let err = custom
        .invoke(&node(json!({})), &[], &ExecutionContext::default())
        .unwrap_err();
    assert!(err.reason.contains("missing 'params.repo'"));
}

#[test]
fn missing_params_and_bad_state() {
    let ctx = ctx_with_token();

    let err = GithubStatusSink::new()
        .invoke(&node(json!({})), &[], &ctx)
        .unwrap_err();
    assert_eq!(err.reason, "github-status node 'n' missing 'params.repo'");

    // Empty-string repo counts as missing.
    let err = GithubStatusSink::new()
        .invoke(&node(json!({"repo": ""})), &[], &ctx)
        .unwrap_err();
    assert!(err.reason.contains("missing 'params.repo'"));

    let err = GithubStatusSink::new()
        .invoke(&node(json!({"repo": "o/r"})), &[], &ctx)
        .unwrap_err();
    assert_eq!(err.reason, "github-status node 'n' missing 'params.sha'");

    let err = GithubStatusSink::new()
        .invoke(
            &node(json!({"repo": "o/r", "sha": "a", "state": "bogus"})),
            &[],
            &ctx,
        )
        .unwrap_err();
    assert!(err.reason.contains("'params.state' must be one of"));

    // A non-object `params:` resolves to the empty map — same miss.
    let bare = peregrin_core::Node {
        id: "n".into(),
        driver: "github-status".into(),
        params: Value::Null,
        ..Default::default()
    };
    let err = GithubStatusSink::new()
        .invoke(&bare, &[], &ctx)
        .unwrap_err();
    assert!(err.reason.contains("missing 'params.repo'"));

    // Strict mode: an unresolvable placeholder fails the param
    // resolve pass itself (DriverError, not an event).
    let mut strict = ctx_with_token();
    strict.strict = true;
    let err = GithubStatusSink::new()
        .invoke(
            &node(json!({"repo": "${event.payload.missing}", "sha": "a"})),
            &[trigger("t", json!({}))],
            &strict,
        )
        .unwrap_err();
    assert!(!err.reason.is_empty());
}

// ---------------------------------------------------------------------------
// The happy path — request shape + posted confirmation event
// ---------------------------------------------------------------------------

#[test]
fn posts_status_and_emits_posted() {
    let stub = serve(CREATED);
    let ctx = ctx_with_token();
    let out = invoke(
        &node(json!({
            "repo": "o/r",
            "sha": "abc123",
            "api_base": stub.url,
            "timeout": 10,
        })),
        &[trigger("build.done", json!({}))],
        &ctx,
    );
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].kind, "n.status-posted");
    assert_eq!(out[0].payload["status"], 201);
    assert_eq!(out[0].payload["state"], "success");
    assert_eq!(out[0].payload["context"], "ci/n");
    assert_eq!(out[0].payload["repo"], "o/r");
    assert_eq!(out[0].payload["sha"], "abc123");
    assert_eq!(out[0].payload["body"]["id"], 42);
    assert_eq!(out[0].source, "n");

    let request = stub.captured();
    assert!(request.starts_with("POST /repos/o/r/statuses/abc123 HTTP/1.1"));
    assert!(request.contains("authorization: Bearer tok123"));
    assert!(request.contains("accept: application/vnd.github+json"));
    assert!(request.contains("x-github-api-version: 2022-11-28"));
    assert!(request.contains("content-type: application/json"));
    assert!(request.contains("\"state\":\"success\""));
    assert!(request.contains("\"context\":\"ci/n\""));
    // Default description = the trigger event type.
    assert!(request.contains("\"description\":\"build.done\""));
    // No target_url param → key absent from the body.
    assert!(!request.contains("target_url"));
}

// ---------------------------------------------------------------------------
// State mapping — trigger type suffix, explicit override
// ---------------------------------------------------------------------------

#[test]
fn state_auto_map_matrix() {
    let ctx = ctx_with_token();
    for (kind, state) in [
        ("build.done", "success"),
        ("lint.done", "success"),
        ("build.failed", "failure"),
        ("node.failed", "failure"),
        ("deploy.pending", "pending"),
        ("workflow.cancelled", "error"),
        ("weird", "error"),
    ] {
        let stub = serve(CREATED);
        let out = invoke(
            &node(base_params(&stub.url)),
            &[trigger(kind, json!({}))],
            &ctx,
        );
        assert_eq!(out[0].payload["state"], state, "trigger {kind}");
        stub.captured();
    }

    // No trigger at all → error (and the description falls back to it).
    let stub = serve(CREATED);
    let out = invoke(&node(base_params(&stub.url)), &[], &ctx);
    assert_eq!(out[0].payload["state"], "error");
    let request = stub.captured();
    assert!(request.contains("\"description\":\"error\""));

    // Every explicit state value wins over the mapping.
    for state in ["error", "failure", "pending", "success"] {
        let stub = serve(CREATED);
        let mut params = base_params(&stub.url);
        params["state"] = json!(state);
        let out = invoke(&node(params), &[trigger("build.done", json!({}))], &ctx);
        assert_eq!(out[0].payload["state"], state);
        stub.captured();
    }
}

// ---------------------------------------------------------------------------
// Context defaulting
// ---------------------------------------------------------------------------

#[test]
fn context_default_and_override() {
    let ctx = ctx_with_token();

    // Default: ci/<node.id>.
    let stub = serve(CREATED);
    let out = invoke(&node(base_params(&stub.url)), &[], &ctx);
    assert_eq!(out[0].payload["context"], "ci/n");
    stub.captured();

    // config.name mirrored into config → ci/<campaign>.
    let mut named = ctx_with_token();
    named.config = std::rc::Rc::new(Map::from_iter([("name".to_string(), json!("peregrin"))]));
    let stub = serve(CREATED);
    let out = invoke(&node(base_params(&stub.url)), &[], &named);
    assert_eq!(out[0].payload["context"], "ci/peregrin");
    stub.captured();

    // params.context wins outright; an empty one falls back.
    let stub = serve(CREATED);
    let mut params = base_params(&stub.url);
    params["context"] = json!("ci/custom");
    let out = invoke(&node(params), &[], &named);
    assert_eq!(out[0].payload["context"], "ci/custom");
    stub.captured();

    let stub = serve(CREATED);
    let mut params = base_params(&stub.url);
    params["context"] = json!("");
    let out = invoke(&node(params), &[], &named);
    assert_eq!(out[0].payload["context"], "ci/peregrin");
    stub.captured();
}

// ---------------------------------------------------------------------------
// Event-payload templating + status_repo redirect
// ---------------------------------------------------------------------------

#[test]
fn repo_sha_template_from_trigger() {
    let stub = serve(CREATED);
    let ctx = ctx_with_token();
    let out = invoke(
        &node(json!({
            "repo": "${event.payload.repo}",
            "sha": "${event.payload.sha}",
            "api_base": stub.url,
            "description": "sha ${event.payload.sha}",
            "target_url": "${event.payload.run_url}",
            "context": "ci/${event.payload.campaign}",
        })),
        &[trigger(
            "build.failed",
            json!({
                "repo": "9aia/peregrin",
                "sha": "deadbeef",
                "run_url": "https://ci.example/run/7",
                "campaign": "gate",
            }),
        )],
        &ctx,
    );
    assert_eq!(out[0].payload["state"], "failure");
    assert_eq!(out[0].payload["context"], "ci/gate");
    let request = stub.captured();
    assert!(request.starts_with("POST /repos/9aia/peregrin/statuses/deadbeef "));
    assert!(request.contains("\"description\":\"sha deadbeef\""));
    assert!(request.contains("\"target_url\":\"https://ci.example/run/7\""));
}

#[test]
fn status_repo_redirect_wins() {
    let stub = serve(CREATED);
    let ctx = ctx_with_token();
    let out = invoke(
        &node(json!({
            "repo": "o/r",
            "status_repo": "mirror/fork",
            "sha": "abc",
            "api_base": stub.url,
        })),
        &[],
        &ctx,
    );
    assert_eq!(out[0].payload["repo"], "mirror/fork");
    let request = stub.captured();
    assert!(request.starts_with("POST /repos/mirror/fork/statuses/abc "));
}

// ---------------------------------------------------------------------------
// Token lookup — capability gate + secrets record
// ---------------------------------------------------------------------------

#[test]
fn token_gate_and_lookup_miss() {
    let post = base_params("http://127.0.0.1:1"); // never reached on deny

    // No `secrets` capability at all → denied before any lookup.
    let err = GithubStatusSink::new()
        .invoke(&node(post.clone()), &[], &ExecutionContext::default())
        .unwrap_err();
    assert_eq!(
        err.reason,
        "github-status node 'n': capability 'secrets' not granted for 'GITHUB_TOKEN'"
    );

    // Explicit `secrets: false` denies.
    let err = GithubStatusSink::new()
        .invoke(&node(post.clone()), &[], &ctx_with(json!(false), &[]))
        .unwrap_err();
    assert!(err.reason.contains("capability 'secrets' not granted"));

    // Capability granted but the secret isn't in the record.
    let err = GithubStatusSink::new()
        .invoke(&node(post.clone()), &[], &ctx_with(json!(true), &[]))
        .unwrap_err();
    assert_eq!(
        err.reason,
        "github-status node 'n': secret 'GITHUB_TOKEN' unresolved"
    );
}

#[test]
fn token_env_custom_name_and_strict_grants() {
    // `token_env` picks the secret key; the same name feeds the
    // strict grant check.
    let stub = serve(CREATED);
    let ctx = ctx_with(json!(true), &[("GH_PAT", "pat-token")]);
    let mut params = base_params(&stub.url);
    params["token_env"] = json!("GH_PAT");
    let out = invoke(&node(params), &[], &ctx);
    assert_eq!(out[0].kind, "n.status-posted");
    assert!(stub.captured().contains("authorization: Bearer pat-token"));

    // Strict + a glob allow-list that matches.
    let stub = serve(CREATED);
    let mut strict = ctx_with(json!(["GH_*"]), &[("GH_PAT", "x")]);
    strict.strict = true;
    let mut params = base_params(&stub.url);
    params["token_env"] = json!("GH_PAT");
    let out = invoke(&node(params), &[], &strict);
    assert_eq!(out[0].kind, "n.status-posted");
    stub.captured();

    // Strict + grant that doesn't match the name → denied.
    let mut strict = ctx_with(json!(["OTHER_*"]), &[("GH_PAT", "x")]);
    strict.strict = true;
    let err = GithubStatusSink::new()
        .invoke(
            &node(json!({"repo": "o/r", "sha": "a", "token_env": "GH_PAT"})),
            &[],
            &strict,
        )
        .unwrap_err();
    assert!(
        err.reason
            .contains("capability 'secrets' not granted for 'GH_PAT'")
    );

    // Strict + {allow, deny} object — deny wins over allow.
    let mut strict = ctx_with(
        json!({"allow": ["*"], "deny": ["GH_*"]}),
        &[("GH_PAT", "x")],
    );
    strict.strict = true;
    let err = GithubStatusSink::new()
        .invoke(
            &node(json!({"repo": "o/r", "sha": "a", "token_env": "GH_PAT"})),
            &[],
            &strict,
        )
        .unwrap_err();
    assert!(err.reason.contains("capability 'secrets' not granted"));

    // Strict + a non-bool/list/object grant denies.
    let mut strict = ctx_with(json!(5), &[("GITHUB_TOKEN", "x")]);
    strict.strict = true;
    let err = GithubStatusSink::new()
        .invoke(&node(json!({"repo": "o/r", "sha": "a"})), &[], &strict)
        .unwrap_err();
    assert!(err.reason.contains("capability 'secrets' not granted"));

    // Non-strict: a present-but-nonmatching grant still resolves —
    // matching is a strict-only check, like `${secret.*}`.
    let stub = serve(CREATED);
    let ctx = ctx_with(json!(["OTHER_*"]), &[("GITHUB_TOKEN", "x")]);
    let out = invoke(&node(base_params(&stub.url)), &[], &ctx);
    assert_eq!(out[0].kind, "n.status-posted");
    stub.captured();
}

// ---------------------------------------------------------------------------
// Confirmation event naming — params.emit / emits[0] fallbacks
// ---------------------------------------------------------------------------

#[test]
fn emit_overrides() {
    let ctx = ctx_with_token();

    // emits[0] when nothing else names it.
    let stub = serve(CREATED);
    let mut n = node(base_params(&stub.url));
    n.emits = vec!["gate.posted".into()];
    let out = invoke(&n, &[], &ctx);
    assert_eq!(out[0].kind, "gate.posted");
    stub.captured();

    // params.emit as a bare string.
    let stub = serve(CREATED);
    let mut params = base_params(&stub.url);
    params["emit"] = json!("ci.verified");
    let out = invoke(&node(params), &[], &ctx);
    assert_eq!(out[0].kind, "ci.verified");
    stub.captured();

    // params.emit map — `posted` key, then `default`.
    let stub = serve(CREATED);
    let mut params = base_params(&stub.url);
    params["emit"] = json!({"posted": "ci.posted", "default": "ci.other"});
    let out = invoke(&node(params), &[], &ctx);
    assert_eq!(out[0].kind, "ci.posted");
    stub.captured();

    let stub = serve(CREATED);
    let mut params = base_params(&stub.url);
    params["emit"] = json!({"default": "ci.fallback"});
    let out = invoke(&node(params), &[], &ctx);
    assert_eq!(out[0].kind, "ci.fallback");
    stub.captured();
}

// ---------------------------------------------------------------------------
// Failure arms — non-2xx API error, transport failure, dry run
// ---------------------------------------------------------------------------

#[test]
fn api_error_is_node_failed() {
    let stub = serve(
        "HTTP/1.1 422 Unprocessable Entity\r\ncontent-type: application/json\r\ncontent-length: 52\r\n\r\n{\"message\":\"Validation Failed\",\"errors\":[\"bad sha\"]}",
    );
    let ctx = ctx_with_token();
    let out = invoke(
        &node(json!({
            "repo": "o/r",
            "sha": "nope",
            "api_base": stub.url,
            "context": "ci/gate",
        })),
        &[],
        &ctx,
    );
    assert_eq!(out[0].kind, "node.failed");
    assert_eq!(out[0].payload["status"], 422);
    // The GitHub `message` surfaces as `error`; the full body rides too.
    assert_eq!(out[0].payload["error"], "Validation Failed");
    assert_eq!(out[0].payload["body"]["errors"][0], "bad sha");
    assert_eq!(out[0].payload["context"], "ci/gate");
    assert_eq!(
        out[0].payload["url"],
        format!("{}/repos/o/r/statuses/nope", stub.url)
    );
    stub.captured();
}

#[test]
fn api_error_non_json_body() {
    // A non-JSON error body surfaces raw under `error`.
    let stub = serve(
        "HTTP/1.1 502 Bad Gateway\r\ncontent-type: text/plain\r\ncontent-length: 5\r\n\r\nproxy",
    );
    let ctx = ctx_with_token();
    let out = invoke(&node(base_params(&stub.url)), &[], &ctx);
    assert_eq!(out[0].kind, "node.failed");
    assert_eq!(out[0].payload["status"], 502);
    assert_eq!(out[0].payload["error"], "proxy");
    stub.captured();
}

#[test]
fn transport_failure_is_node_failed_status_zero() {
    let closed = {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);
        addr
    };
    let ctx = ctx_with_token();
    let out = invoke(
        &node(json!({
            "repo": "o/r",
            "sha": "a",
            "api_base": format!("http://{closed}"),
        })),
        &[],
        &ctx,
    );
    assert_eq!(out[0].kind, "node.failed");
    assert_eq!(out[0].payload["status"], 0);
    assert!(out[0].payload["error"].is_string());
    assert!(!out[0].payload["error"].as_str().unwrap().is_empty());
}

#[test]
fn dry_run_posts_nothing() {
    // No stub, no token — dry_run returns the would-be post without
    // touching the network or the secrets record.
    let ctx = ExecutionContext {
        dry_run: true,
        ..Default::default()
    };
    let out = invoke(
        &node(json!({
            "repo": "o/r",
            "sha": "abc",
            "target_url": "https://ci/run/1",
            "emit": "ci.planned",
        })),
        &[trigger("build.done", json!({}))],
        &ctx,
    );
    assert_eq!(out[0].kind, "ci.planned");
    assert_eq!(out[0].payload["dry_run"], true);
    assert_eq!(out[0].payload["state"], "success");
    assert_eq!(out[0].payload["context"], "ci/n");
    assert_eq!(out[0].payload["description"], "build.done");
    assert_eq!(out[0].payload["target_url"], "https://ci/run/1");
}

#[test]
fn api_base_trailing_slash_and_text_body() {
    // Trailing slash on api_base is trimmed; a text 2xx body rides
    // the posted payload as a string.
    let stub = serve("HTTP/1.1 200 OK\r\ncontent-type: text/plain\r\ncontent-length: 2\r\n\r\nok");
    let ctx = ctx_with_token();
    let out = invoke(
        &node(json!({
            "repo": "o/r",
            "sha": "a",
            "api_base": format!("{}/", stub.url),
        })),
        &[],
        &ctx,
    );
    assert_eq!(out[0].payload["status"], 200);
    assert_eq!(out[0].payload["body"], "ok");
    let request = stub.captured();
    assert!(request.starts_with("POST /repos/o/r/statuses/a "));
}
