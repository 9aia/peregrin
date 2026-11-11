//! peregrin-driver-http — `driver: http`, port of `ac-driver-http`.
//!
//! `params.url` (required) is fetched with `params.method` (default
//! `GET`), `params.headers`, an optional `params.body` (string sent
//! raw; object/array JSON-encoded with a `Content-Type:
//! application/json` default; number/boolean via `String(v)`), and
//! `params.timeout` in seconds (default 300 — ADR-0005). A 2xx
//! response emits `http.done`, any other status `http.failed`, both
//! carrying `{status, body, url, method, headers, attempts}`; a JSON
//! `content-type` parses the body, falling back to text. A transport
//! failure emits `http.failed` with `status: 0` (the TS converts the
//! fetch rejection, not `Effect.fail`).
//!
//! `params.retries` (default 0, capped at 10) re-sends the request
//! after a transport failure or a retryable status (429/5xx) with a
//! linear 250 ms×attempt backoff — opt-in because retrying is only
//! safe for idempotent endpoints (the ci-ingress `POST
//! /api/entities` dedups on the deterministic entity id).
//!
//! `params.notify` + `params.apprise` sends a notification through the
//! TS `handl-plugin-apprise` `send` path: `tgram:`/`jsons:`/`json:`
//! scheme rewrites then a `{body, title, format}` JSON POST for
//! `http(s)://` targets. The TS `ahha`/`nodemailer` transports behind
//! other schemes are not ported — they fail with a `DriverError`.

use std::sync::Arc;
use std::time::Duration;

use serde_json::{Map, Value};

use peregrin_core::{Event, Node, js_string};
use peregrin_driver::{Driver, DriverError, ExecutionContext};
use peregrin_template::{
    Context, Effects, EventScope, NodeScope, Options, ParamsScope, Scope, SystemEffects,
    resolve_value_opts,
};

/// `params.timeout` default — 300 s (ADR-0005).
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(300);
/// `notifyOne`'s default timeout — 60 s.
const NOTIFY_TIMEOUT: Duration = Duration::from_secs(60);

/// `driver: http`. `effects` backs the driver-side param re-resolve
/// the TS performs inside `invoke`.
pub struct HttpDriver {
    effects: Arc<dyn Effects>,
}

impl Default for HttpDriver {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpDriver {
    /// A driver over [`SystemEffects`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            effects: Arc::new(SystemEffects),
        }
    }

    /// A driver over custom effects (tests).
    #[must_use]
    pub fn with_effects(effects: Arc<dyn Effects>) -> Self {
        Self { effects }
    }

    /// Re-resolve `node.params` the way the TS driver does inside
    /// `invoke` (the runner resolves too — the driver-level pass keeps
    /// direct invocations working). `ctx` lends its strict flag,
    /// capability map, and shared missing-secret ledger.
    fn resolve_params(
        &self,
        node: &Node,
        triggers: &[Event],
        ctx: &ExecutionContext,
    ) -> Result<Map<String, Value>, DriverError> {
        let tctx = Context {
            strict: ctx.strict,
            missing_secrets: ctx.missing_secrets.clone(),
            capabilities: (*ctx.capabilities).clone(),
            effects: &*self.effects,
        };
        let state_view = |key: &str| ctx.state.borrow().get(key);
        let secret_view = |name: &str| ctx.resolve_secret(name);
        let resolved = resolve_value_opts(
            &node.params,
            &tctx,
            &Options {
                node: Some(node),
                functions: &[],
                scope: Scope {
                    event: triggers.first().map(|event| EventScope {
                        event,
                        patterns: &node.on,
                    }),
                    config: Some(&ctx.config),
                    params: ParamsScope::Resolved(&ctx.params),
                    state: Some(&state_view),
                    node: Some(NodeScope {
                        id: &node.id,
                        fires: ctx.fires,
                    }),
                    secrets: Some(&secret_view),
                },
            },
        )
        .map_err(|error| DriverError::new(error.to_string()))?;
        Ok(resolved.as_object().cloned().unwrap_or_default())
    }
}

/// `makeBody` — string bodies pass through; objects/arrays JSON-encode
/// and default `Content-Type` to `application/json` (the TS checks the
/// two casings only); numbers/booleans stringify; `null` and
/// unsupported types send no body.
fn make_body(body: Option<&Value>, headers: &mut Map<String, Value>) -> Option<String> {
    let body = body?;
    match body {
        Value::String(text) => Some(text.clone()),
        Value::Array(_) | Value::Object(_) => {
            if !headers.contains_key("Content-Type") && !headers.contains_key("content-type") {
                headers.insert(
                    "Content-Type".to_string(),
                    Value::String("application/json".to_string()),
                );
            }
            // `JSON.stringify` — `Display` on a Value is compact JSON.
            Some(body.to_string())
        }
        Value::Number(_) | Value::Bool(_) => Some(js_string(body)),
        Value::Null => None,
    }
}

/// One ureq agent per invoke — `timeout_global` is end-to-end (DNS →
/// body read), the fetch `AbortSignal` equivalent.
fn agent_with(timeout: Duration) -> ureq::Agent {
    ureq::Agent::new_with_config(
        ureq::Agent::config_builder()
            .http_status_as_error(false)
            .timeout_global(Some(timeout))
            .build(),
    )
}

/// `fetch(url, {method, headers, body, signal})` — any failure builds
/// as the fetch rejection (`http.failed` with `status: 0`), including
/// malformed methods/headers/uris which the `http` builder defers to
/// `.body()`.
fn send_request(
    agent: &ureq::Agent,
    method: &str,
    url: &str,
    headers: &Map<String, Value>,
    body: Option<String>,
) -> Result<ureq::http::Response<ureq::Body>, String> {
    let mut builder = ureq::http::Request::builder().method(method).uri(url);
    for (key, value) in headers {
        builder = builder.header(key.as_str(), js_string(value));
    }
    match body {
        Some(text) => agent.run(builder.body(text).map_err(|error| error.to_string())?),
        None => agent.run(
            builder
                .body(ureq::SendBody::none())
                .map_err(|error| error.to_string())?,
        ),
    }
    .map_err(|error| error.to_string())
}

/// Linear backoff between `params.retries` attempts — 250 ms × the
/// attempt number that just failed (attempt 1 → 250 ms, 2 → 500 ms…).
fn retry_delay(attempt: u64) -> Duration {
    Duration::from_millis(250 * attempt)
}

/// `readBody` — `content-type` containing `application/json` parses,
/// unparseable JSON falls back to the raw text (`orElseSucceed(text)`);
/// a read failure is a `DriverError`.
fn read_body(response: &mut ureq::http::Response<ureq::Body>) -> Result<Value, DriverError> {
    let text = response
        .body_mut()
        .read_to_string()
        .map_err(|error| DriverError::new(error.to_string()))?;
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_string();
    if content_type.contains("application/json") {
        return Ok(serde_json::from_str(&text).unwrap_or(Value::String(text)));
    }
    Ok(Value::String(text))
}

/// `normalizeTarget` — the apprise scheme shorthands.
fn normalize_target(target: &str) -> String {
    if let Some(rest) = target.strip_prefix("tgram://") {
        return format!("telegram://{rest}");
    }
    if let Some(rest) = target.strip_prefix("jsons://") {
        return format!("https://{rest}");
    }
    if let Some(rest) = target.strip_prefix("json://") {
        return format!("http://{rest}");
    }
    target.to_string()
}

/// `apprise.send`/`notifyOne` — the http(s) arm posts
/// `{body, title, format}` JSON; `undefined` fields drop out of
/// `JSON.stringify` so absent `title`/`format` keys stay absent.
/// Non-http schemes went through the TS `ahha`/`nodemailer`
/// transports, which have no Rust counterpart — they fail.
fn notify_send(target: &str, body: &str, title: Option<&str>) -> Result<(), DriverError> {
    let normalized = normalize_target(target);
    if !(normalized.starts_with("http://") || normalized.starts_with("https://")) {
        return Err(DriverError::new(format!(
            "apprise notify failed for '{target}': scheme unsupported — \
             only http(s) targets are ported (ahha/nodemailer have no Rust equivalent)"
        )));
    }
    let mut payload = Map::from_iter([("body".to_string(), Value::String(body.to_string()))]);
    if let Some(title) = title {
        payload.insert("title".to_string(), Value::String(title.to_string()));
    }
    let mut headers = Map::new();
    headers.insert(
        "Content-Type".to_string(),
        Value::String("application/json".to_string()),
    );
    let response = send_request(
        &agent_with(NOTIFY_TIMEOUT),
        "POST",
        &normalized,
        &headers,
        Some(Value::Object(payload).to_string()),
    )
    .map_err(|error| DriverError::new(format!("apprise notify failed for '{target}': {error}")))?;
    if !response.status().is_success() {
        return Err(DriverError::new(format!(
            "apprise notify failed with status {}",
            response.status().as_u16()
        )));
    }
    Ok(())
}

impl Driver for HttpDriver {
    fn name(&self) -> &'static str {
        "http"
    }

    fn invoke(
        &self,
        node: &Node,
        triggers: &[Event],
        ctx: &ExecutionContext,
    ) -> Result<Vec<Event>, DriverError> {
        let params = self.resolve_params(node, triggers, ctx)?;
        let url = params
            .get("url")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if url.is_empty() {
            return Err(DriverError::new(format!(
                "http node '{}' missing 'params.url'",
                node.id
            )));
        }

        let method = params
            .get("method")
            .and_then(Value::as_str)
            .map_or_else(|| "GET".to_string(), str::to_uppercase);
        let mut headers = params
            .get("headers")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let timeout = params
            .get("timeout")
            .and_then(Value::as_f64)
            .map_or(DEFAULT_TIMEOUT, |seconds| {
                Duration::from_secs_f64(seconds.max(0.0))
            });
        let body = make_body(params.get("body"), &mut headers);
        // `params.retries` — extra attempts after the first, capped;
        // only transport failures and 429/5xx retry (a 4xx is a
        // deterministic rejection — replaying it changes nothing).
        let retries = params
            .get("retries")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            .min(10);

        let mut attempt = 0_u64;
        let mut response = loop {
            attempt += 1;
            match send_request(&agent_with(timeout), &method, url, &headers, body.clone()) {
                Err(error) => {
                    if attempt <= retries {
                        std::thread::sleep(retry_delay(attempt));
                        continue;
                    }
                    return Ok(vec![Event::new(
                        "http.failed",
                        serde_json::json!({
                            "status": 0,
                            "error": error,
                            "url": url,
                            "method": method,
                            "attempts": attempt,
                        }),
                        &node.id,
                    )]);
                }
                Ok(resp) => {
                    let status = resp.status().as_u16();
                    if attempt <= retries && (status == 429 || status >= 500) {
                        drop(resp);
                        std::thread::sleep(retry_delay(attempt));
                        continue;
                    }
                    break resp;
                }
            }
        };

        let response_body = read_body(&mut response)?;
        let ok = response.status().is_success();
        let mut response_headers = Map::new();
        for (name, value) in response.headers() {
            response_headers.insert(
                name.as_str().to_string(),
                Value::String(String::from_utf8_lossy(value.as_bytes()).into_owned()),
            );
        }

        let mut payload = Map::from_iter([
            (
                "status".to_string(),
                Value::Number(serde_json::Number::from(response.status().as_u16())),
            ),
            ("body".to_string(), response_body.clone()),
            ("url".to_string(), Value::String(url.to_string())),
            ("method".to_string(), Value::String(method.clone())),
            ("headers".to_string(), Value::Object(response_headers)),
            (
                "attempts".to_string(),
                Value::Number(serde_json::Number::from(attempt)),
            ),
        ]);

        let apprise = params.get("apprise").and_then(Value::as_str);
        if params.get("notify") == Some(&Value::Bool(true))
            && let Some(target) = apprise.filter(|target| !target.is_empty())
        {
            let notify_body = params
                .get("message")
                .and_then(Value::as_str)
                .filter(|message| !message.is_empty())
                .map_or_else(|| js_string(&response_body), str::to_string);
            let title = params
                .get("title")
                .and_then(Value::as_str)
                .filter(|title| !title.is_empty());
            notify_send(target, &notify_body, title)?;
            payload.insert("notified".to_string(), Value::Bool(true));
            return Ok(vec![Event::new(
                if ok { "http.done" } else { "http.failed" },
                Value::Object(payload),
                &node.id,
            )]);
        }

        Ok(vec![Event::new(
            if ok { "http.done" } else { "http.failed" },
            Value::Object(payload),
            &node.id,
        )])
    }
}
