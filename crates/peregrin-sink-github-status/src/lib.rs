//! peregrin-sink-github-status — `driver: github-status`, the GitHub
//! commit-status egress sink (ADR-0080 sinks-as-egress-drivers,
//! ADR-0082 §4 unified registry; ADR-0081 bupha `ci/<campaign>`
//! parity). Under the unified-driver ruling a sink is just a driver
//! whose node fires on terminal campaign events and publishes the
//! verdict out; the only emitted event is the `.status-posted`
//! confirmation (or `node.failed`).
//!
//! `POST {api_base}/repos/{repo}/statuses/{sha}` with
//! `{state, context, description?, target_url?}`.
//!
//! Params (all template-resolved against the trigger event — e.g.
//! `repo: "${event.payload.repo}"`, `sha: "${event.payload.sha}"`):
//!
//! - `repo` (required) — `owner/name` the status lands on.
//! - `status_repo` — bupha's redirect: when set it wins over `repo`,
//!   posting the verdict onto a different repository.
//! - `sha` (required) — the commit the status describes.
//! - `state` — `pending|success|failure|error`; absent → auto-mapped
//!   from the trigger event type suffix: `.done` → `success`,
//!   `.failed` → `failure`, `.pending` → `pending`, anything else →
//!   `error`.
//! - `context` — the status context. Default is deterministic:
//!   `ci/{config.name}` when the campaign mirrors its `name:` into
//!   `config:` (the driver port sees `config:`, not `campaign.name`),
//!   else `ci/{node.id}`. Campaigns wanting `ci/<campaign>` either set
//!   `config.name` or pass `context: "ci/<campaign>"` outright.
//! - `description` — defaults to the trigger event type (`build.done`);
//!   GitHub truncates at 140 chars.
//! - `target_url` — optional link back to the run.
//! - `api_base` — default `https://api.github.com`; override for GHE
//!   or a test stub.
//! - `token_env` — the **name** of the secret holding the GitHub
//!   token (default `GITHUB_TOKEN`), resolved through the
//!   `capabilities.secrets` gate + `ctx.get_secret` — never a literal
//!   secret in params. The gate mirrors `${secret.*}`: absent or
//!   `false` denies, and under `strict` the name must match the grant
//!   (`true`, glob list, or `{allow, deny}`).
//! - `timeout` — seconds, default 30, capped at 300.
//! - `emit` — a string, or `{posted, default}` map, overriding the
//!   confirmation event type; falls back to `emits[0]` then
//!   `{node.id}.status-posted`.
//!
//! Failure convention (the `http` driver's): non-2xx emits
//! `node.failed` with `{status, error, body, context, url}` — the API
//! error payload surfaced verbatim; a transport failure emits the
//! same shape with `status: 0`. Missing params/secrets and capability
//! denials are [`DriverError`]s.

use std::sync::Arc;
use std::time::Duration;

use serde_json::{Map, Value};

use peregrin_core::{Event, EventSchema, Node, js_string};
use peregrin_driver::{Driver, DriverError, ExecutionContext};
use peregrin_template::{
    Context, Effects, EventScope, NodeScope, Options, ParamsScope, Scope, SystemEffects,
    resolve_value_opts,
};

/// `params.api_base` default — the public GitHub REST endpoint.
const DEFAULT_API_BASE: &str = "https://api.github.com";
/// `params.token_env` default — conventional name for the GH token.
const DEFAULT_TOKEN_ENV: &str = "GITHUB_TOKEN";
/// `params.timeout` default — a status post is a single short call.
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
/// Ceiling on `params.timeout`.
const MAX_TIMEOUT: Duration = Duration::from_secs(300);
/// The `User-Agent` GitHub requires on every API call.
const USER_AGENT: &str = "peregrin-sink-github-status";
/// States the commit-status API accepts (`state` in the POST body).
const VALID_STATES: &[&str] = &["error", "failure", "pending", "success"];

/// `driver: github-status`. `effects` backs the driver-side param
/// re-resolve the other impls perform inside `invoke`.
pub struct GithubStatusSink {
    effects: Arc<dyn Effects>,
    output: Vec<EventSchema>,
}

impl Default for GithubStatusSink {
    fn default() -> Self {
        Self::new()
    }
}

impl GithubStatusSink {
    /// A sink over [`SystemEffects`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            effects: Arc::new(SystemEffects),
            output: output_schemas(),
        }
    }

    /// A sink over custom effects (tests).
    #[must_use]
    pub fn with_effects(effects: Arc<dyn Effects>) -> Self {
        Self {
            effects,
            output: output_schemas(),
        }
    }

    /// Re-resolve `node.params` the way `driver: http`/`shell` do —
    /// `event.*` binds the first trigger so `repo`/`sha` template off
    /// the terminal event's payload.
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
                    secrets: Some(&ctx.secrets),
                },
            },
        )
        .map_err(|error| DriverError::new(error.to_string()))?;
        Ok(resolved.as_object().cloned().unwrap_or_default())
    }
}

/// `params.state` explicit value → GitHub state; absent → auto-map
/// the trigger event type suffix (`*.done`/`*.failed`/`*.pending`).
fn resolve_state(params: &Map<String, Value>, triggers: &[Event]) -> Result<String, DriverError> {
    if let Some(state) = params.get("state").and_then(Value::as_str)
        && !state.is_empty()
    {
        if VALID_STATES.contains(&state) {
            return Ok(state.to_string());
        }
        return Err(DriverError::new(format!(
            "github-status 'params.state' must be one of {}",
            VALID_STATES.join("|")
        )));
    }
    let kind = triggers.first().map_or("", |event| event.kind.as_str());
    for (suffix, state) in [
        (".done", "success"),
        (".failed", "failure"),
        (".pending", "pending"),
    ] {
        if kind.ends_with(suffix) {
            return Ok(state.to_string());
        }
    }
    Ok("error".to_string())
}

/// The deterministic context default — `ci/{config.name}` when the
/// campaign mirrors `name:` into `config:`, else `ci/{node.id}`.
fn resolve_context(params: &Map<String, Value>, node: &Node, ctx: &ExecutionContext) -> String {
    if let Some(context) = params.get("context").and_then(Value::as_str)
        && !context.is_empty()
    {
        return context.to_string();
    }
    let name = ctx
        .config
        .get("name")
        .and_then(Value::as_str)
        .filter(|name| !name.is_empty());
    format!("ci/{}", name.unwrap_or(&node.id))
}

/// The token read — `params.token_env` names a **secret key**, gated
/// the way `${secret.*}` is: `capabilities.secrets` absent or `false`
/// denies outright; under `strict` the name must match the grant
/// (`true`, a glob list, or `{allow, deny}`). The lookup itself goes
/// through `ctx.get_secret` so strict-mode misses are audited on the
/// shared ledger.
fn resolve_token(
    params: &Map<String, Value>,
    node: &Node,
    ctx: &ExecutionContext,
) -> Result<String, DriverError> {
    let token_env = params
        .get("token_env")
        .and_then(Value::as_str)
        .filter(|name| !name.is_empty())
        .unwrap_or(DEFAULT_TOKEN_ENV);
    let deny =
        |reason: &str| DriverError::new(format!("github-status node '{}': {reason}", node.id));
    let Some(grant) = ctx.capabilities.get("secrets") else {
        return Err(deny(&format!(
            "capability 'secrets' not granted for '{token_env}'"
        )));
    };
    if matches!(grant, Value::Bool(false)) || (ctx.strict && !capability_matches(token_env, grant))
    {
        return Err(deny(&format!(
            "capability 'secrets' not granted for '{token_env}'"
        )));
    }
    let token = ctx.get_secret(token_env);
    if token.is_empty() {
        return Err(deny(&format!("secret '{token_env}' unresolved")));
    }
    Ok(token)
}

/// `capability_matches` — the `capabilities.<name>` grant check from
/// `peregrin-template`'s capability gate (kept crate-local: the gate
/// itself is `pub(crate)` there). `true` allows all; arrays glob
/// alternatives; `{allow, deny}` applies deny first.
fn capability_matches(value: &str, caps: &Value) -> bool {
    match caps {
        Value::Bool(b) => *b,
        Value::Array(patterns) => patterns
            .iter()
            .any(|pattern| glob_match(&js_string(pattern), value)),
        Value::Object(map) => {
            let list = |key: &str| {
                map.get(key)
                    .and_then(Value::as_array)
                    .map_or_else(Vec::new, |items| items.iter().map(js_string).collect())
            };
            if list("deny").iter().any(|p| glob_match(p, value)) {
                return false;
            }
            let allow = list("allow");
            !allow.is_empty() && allow.iter().any(|p| glob_match(p, value))
        }
        _ => false,
    }
}

/// `globMatch` — `*` any run (no line-terminator crossing), `?` one
/// char, everything else literal. Iterative star-backtracking port of
/// the TS matcher.
fn glob_match(pattern: &str, value: &str) -> bool {
    let pattern: Vec<char> = pattern.chars().collect();
    let value: Vec<char> = value.chars().collect();
    let (mut p, mut v) = (0, 0);
    let mut star: Option<(usize, usize)> = None;
    while v < value.len() {
        if p < pattern.len() && pattern[p] == '*' {
            star = Some((p, v));
            p += 1;
        } else if p < pattern.len()
            && (pattern[p] == value[v] || (pattern[p] == '?' && !is_line_terminator(value[v])))
        {
            p += 1;
            v += 1;
        } else if let Some((star_p, star_v)) = star {
            if is_line_terminator(value[star_v]) {
                return false;
            }
            star = Some((star_p, star_v + 1));
            p = star_p + 1;
            v = star_v + 1;
        } else {
            return false;
        }
    }
    while p < pattern.len() && pattern[p] == '*' {
        p += 1;
    }
    p == pattern.len()
}

/// JS regex `.` does not match these.
fn is_line_terminator(ch: char) -> bool {
    matches!(ch, '\n' | '\r' | '\u{2028}' | '\u{2029}')
}

/// The confirmation event type — `params.emit` string or
/// `{posted, default}` map, then `emits[0]`, then
/// `{node.id}.status-posted`.
fn posted_kind(params: &Map<String, Value>, node: &Node) -> String {
    let emit = params.get("emit");
    if let Some(kind) = emit.and_then(Value::as_str).filter(|k| !k.is_empty()) {
        return kind.to_string();
    }
    if let Some(map) = emit.and_then(Value::as_object)
        && let Some(kind) = map
            .get("posted")
            .or_else(|| map.get("default"))
            .and_then(Value::as_str)
            .filter(|k| !k.is_empty())
    {
        return kind.to_string();
    }
    node.emits
        .first()
        .cloned()
        .unwrap_or_else(|| format!("{}.status-posted", node.id))
}

/// `input` contract — `repo`/`sha` required, the rest optional.
fn input_schema() -> EventSchema {
    let field = |name: &str, ty: &str| (name.to_string(), Value::String(ty.to_string()));
    EventSchema {
        kind: String::new(),
        fields: Map::from_iter([
            field("repo", "string"),
            field("sha", "string"),
            field("status_repo", "string?"),
            field("state", "string?"),
            field("context", "string?"),
            field("description", "string?"),
            field("target_url", "string?"),
            field("api_base", "string?"),
            field("token_env", "string?"),
            field("timeout", "integer?"),
            field("emit", "object?"),
        ]),
        schema: Map::new(),
        annotations: Map::new(),
    }
}

/// `output` contracts — only `node.failed` is statically nameable;
/// the posted confirmation's type is per-node (`emits[0]`/
/// `{id}.status-posted`) so its contract rides the node's `output:`.
fn output_schemas() -> Vec<EventSchema> {
    vec![EventSchema {
        kind: "node.failed".to_string(),
        fields: Map::from_iter([
            ("error".to_string(), Value::String("string".to_string())),
            ("status".to_string(), Value::String("integer".to_string())),
            ("context".to_string(), Value::String("string?".to_string())),
        ]),
        schema: Map::new(),
        annotations: Map::new(),
    }]
}

/// One agent per invoke — `timeout_global` end-to-end, statuses not
/// raised as errors (the non-2xx arm reads the body itself).
fn agent_with(timeout: Duration) -> ureq::Agent {
    ureq::Agent::new_with_config(
        ureq::Agent::config_builder()
            .http_status_as_error(false)
            .timeout_global(Some(timeout))
            .build(),
    )
}

/// The commit-status POST. `Err` is the transport-rejection string
/// (the `status: 0` arm), not a `DriverError`.
fn post_status(
    agent: &ureq::Agent,
    url: &str,
    token: &str,
    body: &Map<String, Value>,
) -> Result<ureq::http::Response<ureq::Body>, String> {
    let request = ureq::http::Request::builder()
        .method("POST")
        .uri(url)
        .header("authorization", format!("Bearer {token}"))
        .header("accept", "application/vnd.github+json")
        .header("x-github-api-version", "2022-11-28")
        .header("user-agent", USER_AGENT)
        .header("content-type", "application/json")
        .body(serde_json::to_vec(body).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    agent.run(request).map_err(|e| e.to_string())
}

/// `readBody` — JSON content-types parse, everything else stays text
/// (unparseable JSON falls back to the raw text, like `driver: http`).
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

/// `node.failed` payload — `error` surfaces the GitHub message when
/// the body is a JSON object with one, else the raw body.
fn failure_payload(status: u16, body: &Value, context: &str, url: &str) -> Value {
    let error = body
        .get("message")
        .and_then(Value::as_str)
        .map_or_else(|| js_string(body), str::to_string);
    serde_json::json!({
        "status": status,
        "error": error,
        "body": body,
        "context": context,
        "url": url,
    })
}

impl Driver for GithubStatusSink {
    fn name(&self) -> &'static str {
        "github-status"
    }

    fn input(&self) -> Option<EventSchema> {
        Some(input_schema())
    }

    fn output(&self) -> &[EventSchema] {
        &self.output
    }

    fn invoke(
        &self,
        node: &Node,
        triggers: &[Event],
        ctx: &ExecutionContext,
    ) -> Result<Vec<Event>, DriverError> {
        let params = self.resolve_params(node, triggers, ctx)?;

        // `status_repo` — the bupha redirect — wins over `repo`. An
        // empty `status_repo` (the usual `"${config.status_repo}"`
        // wiring when the operator doesn't set it) is a non-override,
        // not a missing-repo error: filter it *before* the fallback
        // so `Some("")` doesn't shadow a populated `repo`.
        let repo = params
            .get("status_repo")
            .and_then(Value::as_str)
            .filter(|repo| !repo.is_empty())
            .or_else(|| {
                params
                    .get("repo")
                    .and_then(Value::as_str)
                    .filter(|repo| !repo.is_empty())
            })
            .map(str::to_string);
        let Some(repo) = repo else {
            return Err(DriverError::new(format!(
                "github-status node '{}' missing 'params.repo'",
                node.id
            )));
        };
        let sha = params
            .get("sha")
            .and_then(Value::as_str)
            .filter(|sha| !sha.is_empty())
            .map(str::to_string);
        let Some(sha) = sha else {
            return Err(DriverError::new(format!(
                "github-status node '{}' missing 'params.sha'",
                node.id
            )));
        };
        let state = resolve_state(&params, triggers)?;
        let context = resolve_context(&params, node, ctx);
        let description = params
            .get("description")
            .and_then(Value::as_str)
            .filter(|d| !d.is_empty())
            .map_or_else(
                || {
                    triggers
                        .first()
                        .map_or_else(|| state.clone(), |e| e.kind.clone())
                },
                str::to_string,
            );
        let target_url = params
            .get("target_url")
            .and_then(Value::as_str)
            .filter(|u| !u.is_empty())
            .map(str::to_string);
        let kind = posted_kind(&params, node);

        if ctx.dry_run {
            return Ok(vec![Event::new(
                kind,
                serde_json::json!({
                    "dry_run": true,
                    "repo": repo,
                    "sha": sha,
                    "state": state,
                    "context": context,
                    "description": description,
                    "target_url": target_url,
                }),
                &node.id,
            )]);
        }

        let token = resolve_token(&params, node, ctx)?;
        let api_base = params
            .get("api_base")
            .and_then(Value::as_str)
            .filter(|b| !b.is_empty())
            .unwrap_or(DEFAULT_API_BASE)
            .trim_end_matches('/');
        let url = format!("{api_base}/repos/{repo}/statuses/{sha}");
        let timeout = params
            .get("timeout")
            .and_then(Value::as_f64)
            .filter(|s| s.is_finite() && *s > 0.0)
            .map_or(DEFAULT_TIMEOUT, |s| {
                Duration::from_secs_f64(s).min(MAX_TIMEOUT)
            });

        let mut body = Map::from_iter([
            ("state".to_string(), Value::String(state.clone())),
            ("context".to_string(), Value::String(context.clone())),
            (
                "description".to_string(),
                Value::String(description.clone()),
            ),
        ]);
        if let Some(target) = &target_url {
            body.insert("target_url".to_string(), Value::String(target.clone()));
        }

        let mut response = match post_status(&agent_with(timeout), &url, &token, &body) {
            Err(error) => {
                return Ok(vec![Event::new(
                    "node.failed",
                    failure_payload(0, &Value::String(error), &context, &url),
                    &node.id,
                )]);
            }
            Ok(response) => response,
        };
        let response_body = read_body(&mut response)?;
        let status = response.status().as_u16();
        if !response.status().is_success() {
            return Ok(vec![Event::new(
                "node.failed",
                failure_payload(status, &response_body, &context, &url),
                &node.id,
            )]);
        }

        Ok(vec![Event::new(
            kind,
            serde_json::json!({
                "status": status,
                "state": state,
                "context": context,
                "repo": repo,
                "sha": sha,
                "url": url,
                "body": response_body,
            }),
            &node.id,
        )])
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    //! Grant-matcher internals — the line-terminator arms of
    //! [`glob_match`] need names that can't come through a normal
    //! `token_env` param, so they live next to the matcher.
    use super::*;
    use serde_json::json;

    #[test]
    fn glob_edges() {
        assert!(glob_match("GITHUB_*", "GITHUB_TOKEN"));
        assert!(glob_match("GITHUB_TOKEN*", "GITHUB_TOKEN")); // trailing star trim
        assert!(glob_match("*TOKEN", "GITHUB_TOKEN")); // leading star backtracks
        assert!(glob_match("GH?_TOKEN", "GHA_TOKEN")); // `?` one char
        assert!(!glob_match("GH?_TOKEN", "GH\n_TOKEN")); // `?` not a line break
        assert!(!glob_match("GH*_TOKEN", "GH\n_TOKEN")); // `*` can't cross \n
        assert!(!glob_match("GH", "GITHUB_TOKEN")); // value runs past pattern
        assert!(!glob_match("GITHUb", "GITHUB_TOKEN")); // case-sensitive
        assert!(!glob_match("A*Z", "AB")); // backtrack still misses
        assert!(!glob_match("", "x"));
        assert!(glob_match("", ""));
    }

    #[test]
    fn grant_shapes() {
        assert!(capability_matches("x", &json!(true)));
        assert!(!capability_matches("x", &json!(false)));
        assert!(capability_matches("GH_PAT", &json!(["GH_*"])));
        assert!(!capability_matches("GH_PAT", &json!(["OTHER"])));
        assert!(!capability_matches("x", &json!({"allow": []})));
        assert!(!capability_matches("x", &Value::Null));
        assert!(!capability_matches("x", &json!("secrets")));
    }
}
