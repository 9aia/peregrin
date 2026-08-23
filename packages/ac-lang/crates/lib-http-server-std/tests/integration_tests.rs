use std::thread;
use std::time::Duration;

use ac_lib_http_server::{HttpMethod, HttpServer, HttpServerRequest, HttpServerResponse};
use ac_lib_http_server_std::StdHttpServer;

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
