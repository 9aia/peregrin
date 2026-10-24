//! Hub → node HTTP bridges.
//!
//! - [`sse_events`] / [`sse_session_stream`] — long-lived SSE forwards.
//!   ureq is a blocking client, so one `std::thread` per open stream
//!   drains the upstream `BufRead` into an mpsc that an axum `Sse`
//!   response drains. `data:`/`event:` frames pass through verbatim;
//!   comment pings are dropped (axum adds its own keep-alive).
//! - [`passthrough`] — the generic `/api/*` forward, run under
//!   `spawn_blocking`. Bounded bodies only.

use std::convert::Infallible;
use std::io::BufReader;

use axum::body::Body;
use axum::extract::{Path, Request, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use futures::stream;

use crate::HubState;

/// Cap on a proxied request body — sits just above the node's own
/// `MAX_JSON_BODY` (32MiB, the `/api/sessions/import` transcript
/// ceiling) so an oversized body hits the *node's* 413, whose error
/// shape the client already understands.
pub(crate) const MAX_PROXY_BODY: usize = 40 * 1024 * 1024;

fn upstream_url(state: &HubState, path_and_query: &str) -> String {
    format!("{}{}", state.node_url, path_and_query)
}

fn auth_header(state: &HubState, headers: &HeaderMap) -> Option<String> {
    if let Some(token) = &state.node_token {
        return Some(format!("Bearer {token}"));
    }
    headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string)
}

/// `GET /api/events` — the node feed (`event:`-named frames).
pub async fn sse_events(State(state): State<HubState>, req: Request<Body>) -> Response {
    let path = path_and_query(&req, "/api/events");
    sse_bridge(&state, &upstream_url(&state, &path), req.headers(), None)
}

/// `GET /api/sessions/{id}/stream` — `SessionEvent` frames, routed to
/// the session's owning node in multi-node setups.
pub async fn sse_session_stream(
    State(state): State<HubState>,
    Path(id): Path<String>,
    req: Request<Body>,
) -> Response {
    let (base, token) = state.session_upstream(&id);
    let path = path_and_query(&req, &format!("/api/sessions/{id}/stream"));
    let url = format!("{base}{path}");
    sse_bridge(&state, &url, req.headers(), token.as_deref())
}

fn path_and_query(req: &Request<Body>, path: &str) -> String {
    match req.uri().query() {
        Some(q) => format!("{path}?{q}"),
        None => path.to_string(),
    }
}

/// Open the upstream SSE connection and copy frames to the client.
fn sse_bridge(
    state: &HubState,
    url: &str,
    headers: &HeaderMap,
    token_override: Option<&str>,
) -> Response {
    let auth = token_override
        .map(|t| format!("Bearer {t}"))
        .or_else(|| auth_header(state, headers));
    let mut req = state.sse_agent().get(url);
    if let Some(auth) = auth {
        req = req.header(header::AUTHORIZATION.as_str(), auth);
    }
    // W3C trace-context propagation — carries this request's
    // `http_request` span downstream so hub→node hops join one trace.
    // Empty when export is off (and under coverage, where the global
    // propagator is the noop — the loop is unreachable there).
    #[cfg(not(coverage))]
    for (k, v) in peregrin_telemetry::trace_headers() {
        req = req.header(&k, v);
    }
    let upstream = match req.call() {
        Ok(res) => res,
        Err(e) => {
            return (StatusCode::BAD_GATEWAY, format!("node unreachable: {e}")).into_response();
        }
    };
    if !upstream.status().is_success() {
        return upstream_error(upstream).into_response();
    }

    let (tx, rx) = tokio::sync::mpsc::channel::<Event>(256);
    // Blocking reader — one thread per open stream. The thread exits
    // when the client drops (blocking_send fails) or upstream closes.
    std::thread::spawn(move || pump_sse(upstream, &tx));
    let stream = stream::unfold(rx, |mut rx| async move {
        rx.recv().await.map(|e| (Ok::<_, Infallible>(e), rx))
    });
    let mut res = Sse::new(stream)
        .keep_alive(
            KeepAlive::new()
                .interval(std::time::Duration::from_secs(15))
                .text("ping"),
        )
        .into_response();
    res.headers_mut()
        .insert(header::CONNECTION, HeaderValue::from_static("keep-alive"));
    res
}

/// Surface a non-2xx upstream as-is (status + `{error}` body).
fn upstream_error(mut res: ureq::http::Response<ureq::Body>) -> (StatusCode, String) {
    let status = res.status().as_u16();
    let body = res
        .body_mut()
        .read_to_string()
        .unwrap_or_else(|_| String::new());
    (
        StatusCode::from_u16(status).unwrap_or(StatusCode::BAD_GATEWAY),
        body,
    )
}

/// ureq maps transport timeouts to `Error::Timeout` wrapped in an
/// `io::Error` of kind `Other` once it crosses the `Read` boundary.
fn is_timeout(e: &std::io::Error) -> bool {
    if matches!(
        e.kind(),
        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
    ) {
        return true;
    }
    e.get_ref()
        .and_then(|inner| inner.downcast_ref::<ureq::Error>())
        .is_some_and(|e| matches!(e, ureq::Error::Timeout(_)))
}

/// Parse `data:`/`event:` line pairs off the blocking upstream reader
/// and forward them until the stream or the client goes away. The
/// agent carries a 30s body-recv timeout — a stall becomes a
/// keep-alive probe: if the client is gone the `blocking_send` fails
/// and the thread ends.
fn pump_sse(res: ureq::http::Response<ureq::Body>, tx: &tokio::sync::mpsc::Sender<Event>) {
    let mut reader = BufReader::new(res.into_body().into_reader());
    pump_lines(&mut reader, tx);
}

/// The reader half of [`pump_sse`] — split out so tests can drive the
/// timeout/liveness/error arms without a real 30-second stall.
fn pump_lines(reader: &mut impl std::io::BufRead, tx: &tokio::sync::mpsc::Sender<Event>) {
    let mut event_name: Option<String> = None;
    let mut data = String::new();
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => break, // EOF
            Ok(_) => {}
            Err(e) if is_timeout(&e) => {
                // Liveness probe — drops the connection (and this
                // thread) the moment the client is gone.
                if tx.blocking_send(Event::default().comment("ping")).is_err() {
                    break;
                }
                continue;
            }
            Err(_) => break,
        }
        let line = line.trim_end_matches(['\r', '\n']);
        if line.is_empty() {
            // Frame boundary — emit if we collected data.
            if !data.is_empty() {
                let mut event = Event::default().data(data.trim_end_matches('\n'));
                if let Some(name) = event_name.take() {
                    event = event.event(name);
                }
                if tx.blocking_send(event).is_err() {
                    break;
                }
            }
            data.clear();
            event_name = None;
        } else if let Some(rest) = line.strip_prefix("data:") {
            data.push_str(rest.strip_prefix(' ').unwrap_or(rest));
            data.push('\n');
        } else if let Some(rest) = line.strip_prefix("event:") {
            event_name = Some(rest.trim().to_string());
        }
        // `:` comments (upstream keep-alives) and unknown fields drop.
    }
}

/// Generic `/{*rest}` under `/api/` — forward method, body, and the
/// auth/content-type headers; return status + body verbatim.
pub async fn passthrough(State(state): State<HubState>, req: Request<Body>) -> Response {
    let path = req.uri().path().to_string();
    let query = req
        .uri()
        .query()
        .map(|q| format!("?{q}"))
        .unwrap_or_default();
    let url = upstream_url(&state, &format!("{path}{query}"));
    let method = req.method().clone();
    let auth = auth_header(&state, req.headers());
    let content_type = req
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    let Ok(body) = axum::body::to_bytes(req.into_body(), MAX_PROXY_BODY).await else {
        return (StatusCode::PAYLOAD_TOO_LARGE, "request body too large").into_response();
    };

    let agent = state.json_agent();
    // Inject before `spawn_blocking` — the current span's context is
    // only visible on this task (blocking workers don't inherit it).
    #[cfg(not(coverage))]
    let trace = peregrin_telemetry::trace_headers();
    let result = tokio::task::spawn_blocking(move || {
        let mut builder = ureq::http::Request::builder()
            .method(method.as_str())
            .uri(&url);
        if let Some(auth) = &auth {
            builder = builder.header(header::AUTHORIZATION.as_str(), auth.as_str());
        }
        #[cfg(not(coverage))]
        for (k, v) in &trace {
            builder = builder.header(k.as_str(), v.as_str());
        }
        if let Some(ct) = &content_type {
            builder = builder.header(header::CONTENT_TYPE.as_str(), ct.as_str());
        }
        let request = builder.body(body.to_vec()).map_err(|e| e.to_string())?;
        agent.run(request).map_err(|e| e.to_string())
    })
    .await;

    proxy_result(result)
}

/// `spawn_blocking` result → the proxy's response — extracted so tests
/// can drive every arm (including the JoinError arm, unreachable
/// without a panicking worker).
fn proxy_result(
    result: Result<Result<ureq::http::Response<ureq::Body>, String>, tokio::task::JoinError>,
) -> Response {
    match result {
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        Ok(Err(e)) => (StatusCode::BAD_GATEWAY, format!("node unreachable: {e}")).into_response(),
        Ok(Ok(mut res)) => {
            let status =
                StatusCode::from_u16(res.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
            let ct = res
                .headers()
                .get(header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .map(str::to_string);
            let body = res.body_mut().read_to_vec().unwrap_or_default();
            let mut response = (status, body).into_response();
            if let Some(ct) = ct {
                if let Ok(v) = HeaderValue::from_str(&ct) {
                    response.headers_mut().insert(header::CONTENT_TYPE, v);
                }
            }
            response
        }
    }
}

#[cfg(test)]
mod cov_tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    /// A canned HTTP server speaking raw bytes — malformed headers and
    /// stalls included.
    fn raw_server(reply: &'static [u8]) -> (String, std::thread::JoinHandle<()>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let h = std::thread::spawn(move || {
            for stream in listener.incoming().take(1) {
                let mut s = stream.unwrap();
                // Read the request head, then reply verbatim.
                let mut buf = [0u8; 8192];
                let _ = std::io::Read::read(&mut s, &mut buf);
                let _ = std::io::Write::write_all(&mut s, reply);
            }
        });
        (url, h)
    }

    #[tokio::test]
    async fn proxy_result_arms() {
        // JoinError — an aborted blocking task.
        let h = tokio::spawn(async {
            std::future::pending::<Result<ureq::http::Response<ureq::Body>, String>>().await
        });
        h.abort();
        let res = proxy_result(h.await);
        assert_eq!(res.status(), StatusCode::INTERNAL_SERVER_ERROR);

        // Ok(Err) — a fabricated transport error.
        let res2 = proxy_result(Ok(Err("dead".into())));
        assert_eq!(res2.status(), StatusCode::BAD_GATEWAY);

        // Ok(Ok) — a real response read off the raw server.
        let (url, _t) = raw_server(
            b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\r\n{}",
        );
        let res3 = tokio::task::spawn_blocking(move || {
            proxy_result(Ok(ureq::get(&url).call().map_err(|e| e.to_string())))
        })
        .await
        .unwrap();
        assert_eq!(res3.status(), StatusCode::OK);
    }

    /// A scripted BufRead — yields lines, then the queued error/EOF.
    struct ScriptReader {
        lines: Vec<std::io::Result<String>>,
    }
    impl std::io::Read for ScriptReader {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            match self.lines.first_mut() {
                None => Ok(0),
                Some(Ok(s)) => {
                    let n = s.len().min(buf.len());
                    buf[..n].copy_from_slice(&s.as_bytes()[..n]);
                    if n == s.len() {
                        let _ = self.lines.remove(0);
                    } else {
                        *s = s[n..].to_string();
                    }
                    Ok(n)
                }
                Some(Err(_)) => Err(self.lines.remove(0).unwrap_err()),
            }
        }
    }
    impl std::io::BufRead for ScriptReader {
        fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
            if matches!(self.lines.first(), Some(Err(_))) {
                return Err(self.lines.remove(0).unwrap_err());
            }
            match self.lines.first() {
                None => Ok(&[]),
                Some(Ok(s)) => Ok(s.as_bytes()),
                Some(Err(_)) => Err(std::io::Error::other("x")),
            }
        }
        fn consume(&mut self, n: usize) {
            if let Some(Ok(s)) = self.lines.first_mut() {
                if n >= s.len() {
                    let _ = self.lines.remove(0);
                } else {
                    *s = s[n..].to_string();
                }
            }
        }
    }

    // `blocking_send` panics on a runtime worker — `pump_lines`
    // runs on dedicated threads in production, so the test drives
    // it on `spawn_blocking` the same way.
    async fn pump(lines: Vec<std::io::Result<String>>, tx: &tokio::sync::mpsc::Sender<Event>) {
        let mut reader = ScriptReader { lines };
        let tx = tx.clone();
        tokio::task::spawn_blocking(move || pump_lines(&mut reader, &tx))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn sse_pump_arms() {
        use std::io::{Error, ErrorKind};
        let io = |e| Err::<String, Error>(e);
        // Frames: event+data, data-only, then EOF.
        let (tx, mut rx) = tokio::sync::mpsc::channel::<Event>(8);
        pump(
            vec![
                Ok("event: m\n".to_string()),
                Ok("data: {\"a\":1}\n".to_string()),
                Ok("\n".to_string()),
                Ok("data: second\n".to_string()),
                Ok("\n".to_string()),
            ],
            &tx,
        )
        .await;
        assert!(rx.recv().await.is_some());
        assert!(rx.recv().await.is_some());

        // Timeout → keep-alive ping; dropped client → break.
        let (tx2, mut rx2) = tokio::sync::mpsc::channel::<Event>(8);
        pump(
            vec![
                io(Error::from(ErrorKind::TimedOut)),
                Ok("data: x\n\n".to_string()),
            ],
            &tx2,
        )
        .await;
        assert!(rx2.recv().await.is_some());
        // Client gone before the ping → `blocking_send` fails → break.
        let (tx3, rx3) = tokio::sync::mpsc::channel::<Event>(1);
        drop(rx3);
        pump(vec![io(Error::from(ErrorKind::TimedOut))], &tx3).await;
        // Non-timeout error → break.
        let (tx4, _rx4) = tokio::sync::mpsc::channel::<Event>(1);
        pump(vec![io(Error::other("reset"))], &tx4).await;
        // is_timeout arms: WouldBlock + a wrapped ureq::Error::Timeout.
        assert!(is_timeout(&Error::from(ErrorKind::TimedOut)));
        assert!(is_timeout(&Error::from(ErrorKind::WouldBlock)));
        assert!(is_timeout(&Error::other(ureq::Error::Timeout(
            ureq::Timeout::RecvBody
        ))));
        assert!(!is_timeout(&Error::other("x")));
    }

    fn test_state(node_url: &str) -> HubState {
        HubState::new(
            leptos::config::LeptosOptions::builder()
                .output_name("peregrin_web")
                .site_root("target/site")
                .build(),
            peregrin_web::testing::FakeNodeApi::seeded().shared()
                as std::sync::Arc<dyn peregrin_web::api::NodeApi>,
            node_url,
            None,
        )
    }

    #[tokio::test]
    async fn upstream_error_and_sse_bridge_arms() {
        // Non-2xx upstream → `upstream_error` status + body verbatim.
        let (url, _t) =
            raw_server(b"HTTP/1.1 500 Internal Server Error\r\nContent-Length: 3\r\n\r\nerr");
        let state = test_state(&url);
        let res = state.sse_agent().get(&url).call().unwrap();
        let (status, body) = upstream_error(res);
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body, "err");

        // `sse_bridge` non-2xx → same surface through the handler.
        let (url_500, _t_500) =
            raw_server(b"HTTP/1.1 500 Internal Server Error\r\nContent-Length: 3\r\n\r\nerr");
        let state_500 = test_state(&url_500);
        let res = sse_bridge(&state_500, &url_500, &HeaderMap::new(), None);
        assert_eq!(res.status(), StatusCode::INTERNAL_SERVER_ERROR);

        // Auth from the request headers (no token override), a
        // success upstream → SSE response, pump thread spawned.
        let (url2, _t2) = raw_server(
            b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: 0\r\n\r\n",
        );
        let mut headers = HeaderMap::new();
        headers.insert(header::AUTHORIZATION, HeaderValue::from_static("Bearer t"));
        let state2 = test_state(&url2);
        let res = sse_bridge(&state2, &url2, &headers, None);
        assert_eq!(res.status(), StatusCode::OK);

        // `raw_server` takes one connection — each call needs its own.
        let sse_body: &[u8] =
            b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: 0\r\n\r\n";
        // A `node_token` on the state takes precedence over headers.
        let (url3, _t3) = raw_server(sse_body);
        let mut state3 = test_state(&url3);
        state3.node_token = Some("tok".into());
        let res = sse_bridge(&state3, &url3, &HeaderMap::new(), Some("over"));
        assert_eq!(res.status(), StatusCode::OK);
        // No override → `auth_header` returns the state token.
        let (url4, _t4) = raw_server(sse_body);
        let mut state4 = test_state(&url4);
        state4.node_token = Some("tok".into());
        let res = sse_bridge(&state4, &url4, &HeaderMap::new(), None);
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn passthrough_content_type_and_auth_arms() {
        // A JSON POST carrying content-type + auth → both builder
        // arms; the 200 + content-type response re-inserts the header.
        let (url, _t) = raw_server(
            b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\r\n{}",
        );
        let state = test_state(&url);
        let req = Request::post("/api/sessions")
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::AUTHORIZATION, "Bearer t")
            .body(Body::from("{}"))
            .unwrap();
        let res = passthrough(State(state), req).await;
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(
            res.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/json"
        );
    }
}
