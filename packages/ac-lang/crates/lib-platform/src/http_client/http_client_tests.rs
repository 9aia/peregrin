use ac_runtime::NonFunctionValue;
use ac_runtime_core::Module;
use ac_vm::{CanonicalContext, Context};

use super::http_client::{HttpClientModule, HttpResponse};
use super::http_client_stub::StubHttpClient;

fn call(module: &dyn Module, name: &str, args: &[NonFunctionValue]) -> NonFunctionValue {
    let func = module.function(name).expect("function exists");
    let mut ctx = CanonicalContext::new();
    func.call(args, &mut ctx).expect("call succeeds")
}

fn s(v: &str) -> NonFunctionValue {
    NonFunctionValue::new(v.to_string())
}

fn o(pairs: &[(&str, &str)]) -> NonFunctionValue {
    let mut map = serde_json::Map::new();
    for (k, v) in pairs {
        map.insert(k.to_string(), serde_json::Value::String(v.to_string()));
    }
    NonFunctionValue::new(serde_json::Value::Object(map))
}

fn null() -> NonFunctionValue {
    NonFunctionValue::new(serde_json::Value::Null)
}

#[test]
fn module_name() {
    let module = HttpClientModule::new(StubHttpClient::default());
    assert_eq!(module.name(), "http");
}

#[test]
fn get() {
    let client = StubHttpClient::with("https://example.com", HttpResponse::new(200, "ok"));
    let module = HttpClientModule::new(client);
    let result = call(&module, "get", &[s("https://example.com")]);
    let obj = result.inner();
    assert_eq!(obj["status"], 200);
    assert_eq!(obj["body"], "ok");
    assert_eq!(obj["ok"], true);
}

#[test]
fn get_with_headers() {
    let client = StubHttpClient::with("https://example.com", HttpResponse::new(200, "ok"));
    let module = HttpClientModule::new(client);
    let headers = o(&[("Authorization", "Bearer token123")]);
    let result = call(&module, "get", &[s("https://example.com"), headers]);
    let obj = result.inner();
    assert_eq!(obj["status"], 200);
}

#[test]
fn post() {
    let client = StubHttpClient::with("https://api.com", HttpResponse::new(201, "created"));
    let module = HttpClientModule::new(client);
    let result = call(&module, "post", &[s("https://api.com"), s("payload")]);
    let obj = result.inner();
    assert_eq!(obj["status"], 201);
    assert_eq!(obj["body"], "created");
    assert_eq!(obj["ok"], true);
}

#[test]
fn post_with_headers() {
    let client = StubHttpClient::with("https://api.com", HttpResponse::new(201, "created"));
    let module = HttpClientModule::new(client);
    let headers = o(&[("Content-Type", "application/json")]);
    let result = call(
        &module,
        "post",
        &[s("https://api.com"), s("payload"), headers],
    );
    let obj = result.inner();
    assert_eq!(obj["status"], 201);
}

#[test]
fn put() {
    let client = StubHttpClient::with("https://api.com/items/1", HttpResponse::new(200, "updated"));
    let module = HttpClientModule::new(client);
    let result = call(
        &module,
        "put",
        &[s("https://api.com/items/1"), s("payload")],
    );
    let obj = result.inner();
    assert_eq!(obj["status"], 200);
    assert_eq!(obj["body"], "updated");
}

#[test]
fn patch() {
    let client = StubHttpClient::with("https://api.com/items/1", HttpResponse::new(200, "patched"));
    let module = HttpClientModule::new(client);
    let result = call(
        &module,
        "patch",
        &[s("https://api.com/items/1"), s("payload")],
    );
    let obj = result.inner();
    assert_eq!(obj["status"], 200);
    assert_eq!(obj["body"], "patched");
}

#[test]
fn delete() {
    let client = StubHttpClient::with("https://api.com/items/1", HttpResponse::new(204, ""));
    let module = HttpClientModule::new(client);
    let result = call(&module, "delete", &[s("https://api.com/items/1")]);
    let obj = result.inner();
    assert_eq!(obj["status"], 204);
    assert_eq!(obj["body"], "");
    assert_eq!(obj["ok"], true);
}

#[test]
fn delete_with_headers() {
    let client = StubHttpClient::with("https://api.com/items/1", HttpResponse::new(204, ""));
    let module = HttpClientModule::new(client);
    let headers = o(&[("Authorization", "Bearer token")]);
    let result = call(&module, "delete", &[s("https://api.com/items/1"), headers]);
    let obj = result.inner();
    assert_eq!(obj["status"], 204);
}

#[test]
fn head() {
    let client = StubHttpClient::with("https://example.com", HttpResponse::new(200, ""));
    let module = HttpClientModule::new(client);
    let result = call(&module, "head", &[s("https://example.com")]);
    let obj = result.inner();
    assert_eq!(obj["status"], 200);
}

#[test]
fn request_get_with_null_body() {
    let client = StubHttpClient::with("https://example.com", HttpResponse::new(200, "ok"));
    let module = HttpClientModule::new(client);
    let result = call(
        &module,
        "request",
        &[s("GET"), s("https://example.com"), null(), null()],
    );
    let obj = result.inner();
    assert_eq!(obj["status"], 200);
    assert_eq!(obj["body"], "ok");
}

#[test]
fn request_post_with_body_and_headers() {
    let client = StubHttpClient::with("https://api.com", HttpResponse::new(201, "created"));
    let module = HttpClientModule::new(client);
    let headers = o(&[("Content-Type", "application/json")]);
    let result = call(
        &module,
        "request",
        &[s("POST"), s("https://api.com"), s("payload"), headers],
    );
    let obj = result.inner();
    assert_eq!(obj["status"], 201);
}

#[test]
fn request_case_insensitive_method() {
    let client = StubHttpClient::with("https://example.com", HttpResponse::new(200, "ok"));
    let module = HttpClientModule::new(client);
    let result = call(
        &module,
        "request",
        &[s("get"), s("https://example.com"), null(), null()],
    );
    let obj = result.inner();
    assert_eq!(obj["status"], 200);
}

#[test]
fn request_unknown_method() {
    let module = HttpClientModule::new(StubHttpClient::default());
    let func = module.function("request").unwrap();
    let mut ctx = CanonicalContext::new();
    let err = func
        .call(
            &[s("BOGUS"), s("https://example.com"), null(), null()],
            &mut ctx,
        )
        .unwrap_err();
    match err {
        ac_vm::Error::PrimitiveError(msg) => assert!(msg.contains("unknown HTTP method")),
        other => panic!("expected PrimitiveError, got {other:?}")}
}

#[test]
fn response_headers() {
    let client = StubHttpClient::with(
        "https://example.com",
        HttpResponse::new(200, "ok")
            .with_header("content-type", "application/json")
            .with_header("x-request-id", "abc-123"),
    );
    let module = HttpClientModule::new(client);
    let result = call(&module, "get", &[s("https://example.com")]);
    let obj = result.inner();
    assert_eq!(obj["headers"]["content-type"], "application/json");
    assert_eq!(obj["headers"]["x-request-id"], "abc-123");
}

#[test]
fn response_ok_false_for_4xx() {
    let client = StubHttpClient::with("https://example.com", HttpResponse::new(404, "not found"));
    let module = HttpClientModule::new(client);
    let result = call(&module, "get", &[s("https://example.com")]);
    let obj = result.inner();
    assert_eq!(obj["status"], 404);
    assert_eq!(obj["ok"], false);
}

#[test]
fn response_status_text_and_url() {
    let client = StubHttpClient::with(
        "https://example.com",
        HttpResponse::new(200, "ok")
            .with_status_text("OK")
            .with_url("https://example.com"),
    );
    let module = HttpClientModule::new(client);
    let result = call(&module, "get", &[s("https://example.com")]);
    let obj = result.inner();
    assert_eq!(obj["status_text"], "OK");
    assert_eq!(obj["url"], "https://example.com");
}

#[test]
fn get_not_found() {
    let module = HttpClientModule::new(StubHttpClient::default());
    let func = module.function("get").unwrap();
    let mut ctx = CanonicalContext::new();
    let err = func.call(&[s("https://nope.com")], &mut ctx).unwrap_err();
    match err {
        ac_vm::Error::PrimitiveError(msg) => assert!(msg.contains("no canned response")),
        other => panic!("expected PrimitiveError, got {other:?}")}
}

#[test]
fn unknown_function() {
    let module = HttpClientModule::new(StubHttpClient::default());
    assert!(module.function("nope").is_none());
}
