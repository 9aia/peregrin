use ac_runtime_values::NonFunctionValue;
use std::collections::HashMap;
use std::sync::Arc;

use ac_runtime_values::{Error};
use ac_runtime::{Context, Function};
use ac_runtime_modules::Module;

use crate::error::HttpError;

// ── Types ───────────────────────────────────────────────────────────

/// HTTP method enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
}

impl HttpMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Patch => "PATCH",
            Self::Delete => "DELETE",
            Self::Head => "HEAD",
            Self::Options => "OPTIONS",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_uppercase().as_str() {
            "GET" => Some(Self::Get),
            "POST" => Some(Self::Post),
            "PUT" => Some(Self::Put),
            "PATCH" => Some(Self::Patch),
            "DELETE" => Some(Self::Delete),
            "HEAD" => Some(Self::Head),
            "OPTIONS" => Some(Self::Options),
            _ => None,
        }
    }
}

/// HTTP request descriptor.
#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: HttpMethod,
    pub url: String,
    pub body: Option<String>,
    pub headers: HashMap<String, String>,
    pub timeout_ms: Option<u64>,
}

impl HttpRequest {
    pub fn new(method: HttpMethod, url: impl Into<String>) -> Self {
        HttpRequest {
            method,
            url: url.into(),
            body: None,
            headers: HashMap::new(),
            timeout_ms: None,
        }
    }

    pub fn with_body(mut self, body: impl Into<String>) -> Self {
        self.body = Some(body.into());
        self
    }

    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(name.into(), value.into());
        self
    }

    pub fn with_timeout(mut self, ms: u64) -> Self {
        self.timeout_ms = Some(ms);
        self
    }
}

/// HTTP response.
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub status_text: String,
    pub body: String,
    pub headers: HashMap<String, String>,
    pub url: String,
}

impl HttpResponse {
    pub fn new(status: u16, body: impl Into<String>) -> Self {
        HttpResponse {
            status,
            status_text: String::new(),
            body: body.into(),
            headers: HashMap::new(),
            url: String::new(),
        }
    }

    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(name.into(), value.into());
        self
    }

    pub fn with_status_text(mut self, text: impl Into<String>) -> Self {
        self.status_text = text.into();
        self
    }

    pub fn with_url(mut self, url: impl Into<String>) -> Self {
        self.url = url.into();
        self
    }

    /// Returns `true` if the status code is in the 2xx range.
    pub fn ok(&self) -> bool {
        self.status >= 200 && self.status < 300
    }
}

// ── Trait ───────────────────────────────────────────────────────────

/// Abstract HTTP client service.
///
/// Inspired by the [Fetch API](https://developer.mozilla.org/en-US/docs/Web/API/Fetch_API),
/// this trait uses a single [`request`] method with an [`HttpRequest`] descriptor,
/// giving full control over method, headers, body, and timeout.
///
/// Implementations:
/// - `ac-lib-http-client-std` — backed by `ureq`
/// - [`http_client_stub::StubHttpClient`] — canned responses for testing
pub trait HttpClient: Send + Sync {
    fn request(&self, req: &HttpRequest) -> Result<HttpResponse, HttpError>;
}

// ── Module adapter ──────────────────────────────────────────────────

/// Module adapter for [`HttpClient`] — exposes `http.*` functions.
///
/// | Function | Args | Description |
/// |----------|------|-------------|
/// | `http.get(url)` | `(url)` or `(url, headers)` | GET request |
/// | `http.post(url, body)` | `(url, body)` or `(url, body, headers)` | POST request |
/// | `http.put(url, body)` | `(url, body)` or `(url, body, headers)` | PUT request |
/// | `http.patch(url, body)` | `(url, body)` or `(url, body, headers)` | PATCH request |
/// | `http.delete(url)` | `(url)` or `(url, headers)` | DELETE request |
/// | `http.head(url)` | `(url)` or `(url, headers)` | HEAD request |
/// | `http.request(method, url, body, headers)` | `(method, url, body, headers)` | Full control |
///
/// `headers` is a JSON object of header name to value.
/// `body` in `http.request` can be `null` for bodyless requests.
pub struct HttpClientModule {
    get: GetFn,
    post: PostFn,
    put: PutFn,
    patch: PatchFn,
    delete: DeleteFn,
    head: HeadFn,
    request: RequestFn,
}

impl HttpClientModule {
    pub fn new(client: impl HttpClient + 'static) -> Self {
        let client = Arc::new(client);
        HttpClientModule {
            get: GetFn {
                client: client.clone(),
            },
            post: PostFn {
                client: client.clone(),
            },
            put: PutFn {
                client: client.clone(),
            },
            patch: PatchFn {
                client: client.clone(),
            },
            delete: DeleteFn {
                client: client.clone(),
            },
            head: HeadFn {
                client: client.clone(),
            },
            request: RequestFn { client },
        }
    }
}

impl Module for HttpClientModule {
    fn name(&self) -> &str {
        "http"
    }

    fn function(&self, name: &str) -> Option<&dyn Function<NonFunctionValue>> {
        match name {
            "get" => Some(&self.get),
            "post" => Some(&self.post),
            "put" => Some(&self.put),
            "patch" => Some(&self.patch),
            "delete" => Some(&self.delete),
            "head" => Some(&self.head),
            "request" => Some(&self.request),
            _ => None,
        }
    }
}

// ── Helpers ─────────────────────────────────────────────────────────

fn require_string_at(args: &[NonFunctionValue], idx: usize) -> Result<&str, Error> {
    args.get(idx)
        .ok_or(Error::ArgumentCount {
            expected: idx + 1,
            got: args.len(),
        })?
        .as_str()
        .ok_or(Error::TypeMismatch {
            expected: "string",
            got: "non-string",
        })
}

fn extract_headers(
    args: &[NonFunctionValue],
    idx: usize,
) -> Result<HashMap<String, String>, Error> {
    match args.get(idx) {
        None => Ok(HashMap::new()),
        Some(v) if v.is_null() => Ok(HashMap::new()),
        Some(v) => {
            let obj = v.as_object().ok_or(Error::TypeMismatch {
                expected: "object",
                got: "non-object",
            })?;
            let mut headers = HashMap::new();
            for (key, val) in obj {
                let value = val.as_str().ok_or(Error::TypeMismatch {
                    expected: "string",
                    got: "non-string",
                })?;
                headers.insert(key.clone(), value.to_string());
            }
            Ok(headers)
        }
    }
}

fn response_to_value(resp: HttpResponse) -> NonFunctionValue {
    let ok = resp.ok();
    let headers: serde_json::Map<String, serde_json::Value> = resp
        .headers
        .into_iter()
        .map(|(k, v)| (k, serde_json::Value::String(v)))
        .collect();
    NonFunctionValue::new(serde_json::json!({
        "status": resp.status,
        "status_text": resp.status_text,
        "ok": ok,
        "body": resp.body,
        "headers": serde_json::Value::Object(headers),
        "url": resp.url}))
}

// ── Verb function structs ───────────────────────────────────────────

macro_rules! bodyless_fn {
    ($name:ident, $method:expr) => {
        struct $name {
            client: Arc<dyn HttpClient>,
        }

        impl Function<NonFunctionValue> for $name {
            fn call(
                &self,
                args: &[NonFunctionValue],
                _ctx: &mut dyn Context<NonFunctionValue>,
            ) -> Result<NonFunctionValue, Error> {
                let url = require_string_at(args, 0)?;
                let headers = extract_headers(args, 1)?;
                let req = HttpRequest {
                    method: $method,
                    url: url.to_string(),
                    body: None,
                    headers,
                    timeout_ms: None,
                };
                self.client
                    .request(&req)
                    .map(response_to_value)
                    .map_err(Into::into)
            }
        }
    };
}

macro_rules! body_fn {
    ($name:ident, $method:expr) => {
        struct $name {
            client: Arc<dyn HttpClient>,
        }

        impl Function<NonFunctionValue> for $name {
            fn call(
                &self,
                args: &[NonFunctionValue],
                _ctx: &mut dyn Context<NonFunctionValue>,
            ) -> Result<NonFunctionValue, Error> {
                let url = require_string_at(args, 0)?;
                let body = require_string_at(args, 1)?;
                let headers = extract_headers(args, 2)?;
                let req = HttpRequest {
                    method: $method,
                    url: url.to_string(),
                    body: Some(body.to_string()),
                    headers,
                    timeout_ms: None,
                };
                self.client
                    .request(&req)
                    .map(response_to_value)
                    .map_err(Into::into)
            }
        }
    };
}

bodyless_fn!(GetFn, HttpMethod::Get);
bodyless_fn!(DeleteFn, HttpMethod::Delete);
bodyless_fn!(HeadFn, HttpMethod::Head);

body_fn!(PostFn, HttpMethod::Post);
body_fn!(PutFn, HttpMethod::Put);
body_fn!(PatchFn, HttpMethod::Patch);

struct RequestFn {
    client: Arc<dyn HttpClient>,
}

impl Function<NonFunctionValue> for RequestFn {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let method_str = require_string_at(args, 0)?;
        let method = HttpMethod::parse(method_str)
            .ok_or_else(|| Error::PrimitiveError(format!("unknown HTTP method: {method_str}")))?;
        let url = require_string_at(args, 1)?;
        let body = match args.get(2) {
            None => None,
            Some(v) if v.is_null() => None,
            Some(v) => Some(
                v.as_str()
                    .ok_or(Error::TypeMismatch {
                        expected: "string",
                        got: "non-string",
                    })?
                    .to_string(),
            ),
        };
        let headers = extract_headers(args, 3)?;
        let req = HttpRequest {
            method,
            url: url.to_string(),
            body,
            headers,
            timeout_ms: None,
        };
        self.client
            .request(&req)
            .map(response_to_value)
            .map_err(Into::into)
    }
}