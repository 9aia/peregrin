use std::fs;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use ac_lib_http_server::{HttpMethod, HttpServer, HttpServerRequest, HttpServerResponse};
use ac_lib_http_server_std::{ServerConfig, StdHttpServer};
use ac_runtime_values::FunctionValue;
use bytes::Bytes;
use http_body_util::{BodyExt, Empty};
use hyper::Request;
use hyper_util::rt::{TokioExecutor, TokioIo};
use serde_json::Value as JsonValue;

#[test]
fn get_route_returns_body() {
    let server = StdHttpServer::new();
    server
        .route(
            HttpMethod::Get,
            "/hello",
            Box::new(|_req: HttpServerRequest| HttpServerResponse::new(200, "Hello!")),
        )
        .unwrap();

    server.start("127.0.0.1:0").unwrap();
    let addr = server.local_addr().expect("server bound");
    thread::sleep(Duration::from_millis(50));

    let url = format!("http://{addr}/hello");
    let resp = ureq::get(&url).call().unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.into_string().unwrap(), "Hello!");

    server.stop().unwrap();
    server.join().unwrap();
}

#[test]
fn unmatched_route_returns_404() {
    let server = StdHttpServer::new();
    server.start("127.0.0.1:0").unwrap();
    let addr = server.local_addr().expect("server bound");
    thread::sleep(Duration::from_millis(50));

    let url = format!("http://{addr}/missing");
    match ureq::get(&url).call() {
        Ok(_) => panic!("expected 404"),
        Err(ureq::Error::Status(code, _)) => assert_eq!(code, 404),
        Err(other) => panic!("unexpected error: {other:?}"),
    }

    server.stop().unwrap();
    server.join().unwrap();
}

#[test]
fn default_content_type_for_handler() {
    let server = StdHttpServer::new();
    server
        .route(
            HttpMethod::Get,
            "/hello",
            Box::new(|_req: HttpServerRequest| HttpServerResponse::new(200, "Hello!")),
        )
        .unwrap();

    server.start("127.0.0.1:0").unwrap();
    let addr = server.local_addr().expect("server bound");
    thread::sleep(Duration::from_millis(50));

    let url = format!("http://{addr}/hello");
    let resp = ureq::get(&url).call().unwrap();
    assert_eq!(
        resp.header("Content-Type"),
        Some("text/plain; charset=utf-8")
    );

    server.stop().unwrap();
    server.join().unwrap();
}

#[test]
fn custom_content_type_preserved() {
    let server = StdHttpServer::new();
    server
        .route(
            HttpMethod::Get,
            "/json",
            Box::new(|_req: HttpServerRequest| {
                HttpServerResponse::new(200, "{}").with_header("Content-Type", "application/json")
            }),
        )
        .unwrap();

    server.start("127.0.0.1:0").unwrap();
    let addr = server.local_addr().expect("server bound");
    thread::sleep(Duration::from_millis(50));

    let url = format!("http://{addr}/json");
    let resp = ureq::get(&url).call().unwrap();
    assert_eq!(resp.header("Content-Type"), Some("application/json"));

    server.stop().unwrap();
    server.join().unwrap();
}

#[test]
fn payload_too_large_returns_413() {
    let server = StdHttpServer::with_config(ServerConfig {
        body_limit: 4,
        ..ServerConfig::default()
    });
    server
        .route(
            HttpMethod::Post,
            "/echo",
            Box::new(|req: HttpServerRequest| HttpServerResponse::new(200, req.body.clone())),
        )
        .unwrap();

    server.start("127.0.0.1:0").unwrap();
    let addr = server.local_addr().expect("server bound");
    thread::sleep(Duration::from_millis(50));

    let url = format!("http://{addr}/echo");
    match ureq::post(&url).send_string("hello") {
        Ok(_) => panic!("expected 413"),
        Err(ureq::Error::Status(code, _)) => assert_eq!(code, 413),
        Err(other) => panic!("unexpected error: {other:?}"),
    }

    server.stop().unwrap();
    server.join().unwrap();
}

#[test]
fn static_file_guesses_mime_type() {
    let dir = PathBuf::from(std::env::temp_dir()).join(format!("ac-test-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("icon.svg"), "<svg></svg>").unwrap();

    let server = StdHttpServer::new();
    server
        .serve_static("/public", dir.to_str().unwrap())
        .unwrap();

    server.start("127.0.0.1:0").unwrap();
    let addr = server.local_addr().expect("server bound");
    thread::sleep(Duration::from_millis(50));

    let url = format!("http://{addr}/public/icon.svg");
    let resp = ureq::get(&url).call().unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.header("Content-Type"), Some("image/svg+xml"));

    server.stop().unwrap();
    server.join().unwrap();

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn http2_request_succeeds() {
    let server = StdHttpServer::new();
    server
        .route(
            HttpMethod::Get,
            "/hello",
            Box::new(|_req: HttpServerRequest| HttpServerResponse::new(200, "Hello!")),
        )
        .unwrap();

    server.start("127.0.0.1:0").unwrap();
    let addr = server.local_addr().expect("server bound");
    thread::sleep(Duration::from_millis(50));

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async {
        let stream = tokio::net::TcpStream::connect(addr).await.expect("connect");
        let (mut request_sender, connection) =
            hyper::client::conn::http2::handshake(TokioExecutor::new(), TokioIo::new(stream))
                .await
                .expect("handshake");
        let drive = tokio::spawn(connection);

        let req = Request::builder()
            .uri(format!("http://{addr}/hello"))
            .body(Empty::<Bytes>::new())
            .unwrap();
        let res = request_sender
            .send_request(req)
            .await
            .expect("send request");
        assert_eq!(res.status(), 200);

        let body = res
            .into_body()
            .collect()
            .await
            .expect("collect body")
            .to_bytes();
        assert_eq!(body, "Hello!");

        drop(request_sender);
        let _ = drive.await;
    });

    server.stop().unwrap();
    server.join().unwrap();
}

#[test]
fn path_parameter_route() {
    let server = StdHttpServer::new();
    server
        .route(
            HttpMethod::Get,
            "/users/{id}",
            Box::new(|req: HttpServerRequest| {
                let id = req.params.get("id").cloned().unwrap_or_default();
                HttpServerResponse::new(200, format!("User {id}"))
            }),
        )
        .unwrap();
    server
        .route(
            HttpMethod::Get,
            "/posts/{post_id}/comments/{comment_id}",
            Box::new(|req: HttpServerRequest| {
                let post = req.params.get("post_id").cloned().unwrap_or_default();
                let comment = req.params.get("comment_id").cloned().unwrap_or_default();
                HttpServerResponse::new(200, format!("{post}-{comment}"))
            }),
        )
        .unwrap();

    server.start("127.0.0.1:0").unwrap();
    let addr = server.local_addr().expect("server bound");
    thread::sleep(Duration::from_millis(50));

    let url = format!("http://{addr}/users/42");
    let resp = ureq::get(&url).call().unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.into_string().unwrap(), "User 42");

    let url = format!("http://{addr}/posts/7/comments/3");
    let resp = ureq::get(&url).call().unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.into_string().unwrap(), "7-3");

    server.stop().unwrap();
    server.join().unwrap();
}

#[test]
fn function_value_handler_returns_string() {
    let function = FunctionValue {
        params: Vec::new(),
        return_schema: None,
        body: "\"hello\"".to_string(),
        patterns: Vec::new(),
    };
    let server = StdHttpServer::new();
    server
        .route_function(HttpMethod::Get, "/hello", function)
        .unwrap();

    server.start("127.0.0.1:0").unwrap();
    let addr = server.local_addr().expect("server bound");
    thread::sleep(Duration::from_millis(50));

    let url = format!("http://{addr}/hello");
    let resp = ureq::get(&url).call().unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.into_string().unwrap(), "hello");

    server.stop().unwrap();
    server.join().unwrap();
}

#[test]
fn function_value_handler_uses_path_param() {
    let function = FunctionValue {
        params: vec![("name".to_string(), JsonValue::Null)],
        return_schema: None,
        body: "name".to_string(),
        patterns: Vec::new(),
    };
    let server = StdHttpServer::new();
    server
        .route_function(HttpMethod::Get, "/hello/{name}", function)
        .unwrap();

    server.start("127.0.0.1:0").unwrap();
    let addr = server.local_addr().expect("server bound");
    thread::sleep(Duration::from_millis(50));

    let url = format!("http://{addr}/hello/world");
    let resp = ureq::get(&url).call().unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.into_string().unwrap(), "world");

    server.stop().unwrap();
    server.join().unwrap();
}
