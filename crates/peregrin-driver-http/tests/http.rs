#![allow(clippy::unwrap_used)]
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use peregrin_driver::{Driver, ExecutionContext};
use peregrin_driver_http::HttpDriver;
use serde_json::{Value, json};

fn node(params: Value) -> peregrin_core::Node {
    peregrin_core::Node {
        id: "n".into(),
        driver: "http".into(),
        params,
        ..Default::default()
    }
}

/// A one-shot HTTP stub: accepts a single connection, reads the whole
/// request (headers + `content-length` body), hands it back over a
/// channel, and writes `response`.
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
    serve_with(move |_| response.to_string())
}

fn serve_with(response: impl FnOnce(&str) -> String + Send + 'static) -> Stub {
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
        let request = String::from_utf8_lossy(&buf).into_owned();
        let reply = response(&request);
        let _ = tx.send(request);
        let _ = stream.write_all(reply.as_bytes());
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

#[test]
fn name_and_missing_url() {
    let driver = HttpDriver::new();
    let _ = HttpDriver::default();
    assert_eq!(driver.name(), "http");
    let ctx = ExecutionContext::default();
    let err = driver.invoke(&node(json!({})), &[], &ctx).unwrap_err();
    assert_eq!(err.reason, "http node 'n' missing 'params.url'");

    // The effects seam — a driver over explicit SystemEffects.
    let custom = HttpDriver::with_effects(std::sync::Arc::new(peregrin_template::SystemEffects));
    let err = custom.invoke(&node(json!({})), &[], &ctx).unwrap_err();
    assert!(err.reason.contains("missing 'params.url'"));
}

#[test]
fn get_done_with_text_body() {
    let stub = serve(
        "HTTP/1.1 200 OK\r\ncontent-type: text/plain\r\ncontent-length: 5\r\nx-flag: on\r\n\r\nhello",
    );
    let ctx = ExecutionContext::default();
    let out = driver_call(&node(json!({"url": stub.url})), &ctx);
    assert_eq!(out[0].kind, "http.done");
    assert_eq!(out[0].payload["status"], 200);
    assert_eq!(out[0].payload["body"], "hello");
    assert_eq!(out[0].payload["url"], stub.url);
    assert_eq!(out[0].payload["method"], "GET");
    assert_eq!(out[0].payload["headers"]["x-flag"], "on");
    let request = stub.captured();
    assert!(request.starts_with("GET / HTTP/1.1"));
}

fn driver_call(node: &peregrin_core::Node, ctx: &ExecutionContext) -> Vec<peregrin_core::Event> {
    HttpDriver::new().invoke(node, &[], ctx).unwrap()
}

#[test]
fn json_body_parsed_and_fallback() {
    // Valid JSON parses into a value.
    let stub = serve(
        "HTTP/1.1 200 OK\r\ncontent-type: application/json; charset=utf-8\r\ncontent-length: 7\r\n\r\n{\"a\":1}",
    );
    let ctx = ExecutionContext::default();
    let out = driver_call(&node(json!({"url": stub.url})), &ctx);
    assert_eq!(out[0].payload["body"]["a"], 1);
    stub.captured();

    // Broken JSON under a json content-type falls back to raw text.
    let stub = serve(
        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: 5\r\n\r\n{oops",
    );
    let out = driver_call(&node(json!({"url": stub.url})), &ctx);
    assert_eq!(out[0].payload["body"], "{oops");
    stub.captured();
}

#[test]
fn post_object_body_and_headers() {
    let stub = serve("HTTP/1.1 200 OK\r\ncontent-length: 0\r\n\r\n");
    let ctx = ExecutionContext::default();
    let out = driver_call(
        &node(json!({
            "url": stub.url,
            "method": "post",
            "headers": {"x-token": "t1", "x-num": 5},
            "body": {"k": "v"},
        })),
        &ctx,
    );
    assert_eq!(out[0].kind, "http.done");
    let request = stub.captured();
    assert!(request.starts_with("POST / HTTP/1.1"));
    assert!(request.contains("x-token: t1"));
    assert!(request.contains("x-num: 5"));
    assert!(request.contains("content-type: application/json"));
    assert!(request.ends_with("{\"k\":\"v\"}"));
}

#[test]
fn body_variants() {
    let ctx = ExecutionContext::default();

    // String body passes through raw, no forced content-type.
    let stub = serve("HTTP/1.1 200 OK\r\ncontent-length: 0\r\n\r\n");
    driver_call(
        &node(json!({"url": stub.url, "method": "POST", "body": "raw"})),
        &ctx,
    );
    let request = stub.captured();
    assert!(request.ends_with("raw"));
    assert!(!request.contains("application/json"));

    // Number body stringifies.
    let stub = serve("HTTP/1.1 200 OK\r\ncontent-length: 0\r\n\r\n");
    driver_call(
        &node(json!({"url": stub.url, "method": "POST", "body": 5})),
        &ctx,
    );
    assert!(stub.captured().ends_with('5'));

    // null body sends nothing.
    let stub = serve("HTTP/1.1 200 OK\r\ncontent-length: 0\r\n\r\n");
    driver_call(&node(json!({"url": stub.url, "body": null})), &ctx);
    let request = stub.captured();
    assert!(!request.contains("content-length"));
}

#[test]
fn non_2xx_and_transport_failure() {
    let ctx = ExecutionContext::default();

    let stub = serve("HTTP/1.1 404 Not Found\r\ncontent-length: 4\r\n\r\nnope");
    let out = driver_call(&node(json!({"url": stub.url})), &ctx);
    assert_eq!(out[0].kind, "http.failed");
    assert_eq!(out[0].payload["status"], 404);
    assert_eq!(out[0].payload["body"], "nope");
    stub.captured();

    // Connection refused — the fetch rejection port: status 0 + error.
    let closed = {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);
        addr
    };
    let out = driver_call(&node(json!({"url": format!("http://{closed}")})), &ctx);
    assert_eq!(out[0].kind, "http.failed");
    assert_eq!(out[0].payload["status"], 0);
    assert!(out[0].payload["error"].is_string());
}

#[test]
fn timeout_fails_the_request() {
    // Accepts but never responds.
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let join = thread::spawn(move || {
        let (_stream, _) = listener.accept().unwrap();
        thread::sleep(Duration::from_secs(2));
    });
    let ctx = ExecutionContext::default();
    let out = driver_call(&node(json!({"url": url, "timeout": 1})), &ctx);
    assert_eq!(out[0].kind, "http.failed");
    assert_eq!(out[0].payload["status"], 0);
    join.join().unwrap();
}

#[test]
fn truncated_body_is_a_driver_error() {
    // content-length promises 100 bytes; the stub closes after 5 —
    // `response.text()` rejecting is a DriverError, not an event.
    let stub = serve("HTTP/1.1 200 OK\r\ncontent-length: 100\r\n\r\nshort");
    let ctx = ExecutionContext::default();
    let err = HttpDriver::new()
        .invoke(&node(json!({"url": stub.url})), &[], &ctx)
        .unwrap_err();
    assert!(!err.reason.is_empty());
    let _ = stub.join.join();
}

#[test]
fn notify_posts_to_http_target() {
    let notify_stub = serve_with(|request| {
        assert!(request.contains("\"body\":\"the response\""));
        assert!(request.contains("\"title\":\"heads up\""));
        "HTTP/1.1 200 OK\r\ncontent-length: 0\r\n\r\n".to_string()
    });
    let body = "the response";
    let stub = serve_with(move |_| {
        format!(
            "HTTP/1.1 200 OK\r\ncontent-type: text/plain\r\ncontent-length: {}\r\n\r\n{body}",
            body.len()
        )
    });
    let ctx = ExecutionContext::default();
    let out = driver_call(
        &node(json!({
            "url": stub.url,
            "notify": true,
            "apprise": notify_stub.url,
            "title": "heads up",
        })),
        &ctx,
    );
    assert_eq!(out[0].kind, "http.done");
    assert_eq!(out[0].payload["notified"], true);
    stub.captured();
    notify_stub.captured();
}

#[test]
fn notify_message_override_and_json_scheme() {
    let notify_stub = serve("HTTP/1.1 200 OK\r\ncontent-length: 0\r\n\r\n");
    let stub = serve("HTTP/1.1 500 ERR\r\ncontent-length: 0\r\n\r\n");
    let apprise = notify_stub.url.replace("http://", "json://");
    let ctx = ExecutionContext::default();
    let out = driver_call(
        &node(json!({
            "url": stub.url,
            "notify": true,
            "apprise": apprise,
            "message": "custom note",
        })),
        &ctx,
    );
    // Non-2xx still notifies, event stays http.failed with `notified`.
    assert_eq!(out[0].kind, "http.failed");
    assert_eq!(out[0].payload["notified"], true);
    stub.captured();
    let request = notify_stub.captured();
    assert!(request.contains("\"body\":\"custom note\""));
}

#[test]
fn notify_failures() {
    let ctx = ExecutionContext::default();

    // tgram → telegram: a non-http scheme — ahha/nodemailer unported.
    let stub = serve("HTTP/1.1 200 OK\r\ncontent-length: 0\r\n\r\n");
    let err = HttpDriver::new()
        .invoke(
            &node(json!({
                "url": stub.url,
                "notify": true,
                "apprise": "tgram://bottoken/chatid",
            })),
            &[],
            &ctx,
        )
        .unwrap_err();
    assert!(err.reason.contains("scheme unsupported"));
    stub.captured();

    // jsons → https: rewrite covered, then the transport fails.
    let stub = serve("HTTP/1.1 200 OK\r\ncontent-length: 0\r\n\r\n");
    let err = HttpDriver::new()
        .invoke(
            &node(json!({
                "url": stub.url,
                "notify": true,
                "apprise": "jsons://127.0.0.1:1/x",
            })),
            &[],
            &ctx,
        )
        .unwrap_err();
    assert!(err.reason.contains("apprise notify failed"));
    stub.captured();

    // The apprise endpoint itself answers non-2xx.
    let notify_stub = serve("HTTP/1.1 500 ERR\r\ncontent-length: 0\r\n\r\n");
    let stub = serve("HTTP/1.1 200 OK\r\ncontent-length: 0\r\n\r\n");
    let err = HttpDriver::new()
        .invoke(
            &node(json!({
                "url": stub.url,
                "notify": true,
                "apprise": notify_stub.url,
            })),
            &[],
            &ctx,
        )
        .unwrap_err();
    assert_eq!(err.reason, "apprise notify failed with status 500");
    stub.captured();
    notify_stub.captured();

    // notify: true with no usable apprise target → plain done.
    let stub = serve("HTTP/1.1 200 OK\r\ncontent-length: 0\r\n\r\n");
    let out = driver_call(
        &node(json!({"url": stub.url, "notify": true, "apprise": ""})),
        &ctx,
    );
    assert_eq!(out[0].kind, "http.done");
    assert!(out[0].payload.get("notified").is_none());
    stub.captured();
}
