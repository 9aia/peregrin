//! Safety helpers — port of `ac-safety`'s `capabilities.ts`,
//! `schemaValidator.ts`, and the `ResourceGate` slice of
//! `resourceGate.ts` the runner uses.
//!
//! - [`CapabilityChecker`] merges workflow + node capability maps and
//!   checks resolved driver params against allow/deny glob policy.
//! - [`SchemaValidator`]/`validate_object`/`validate_record` run the
//!   shallow JSON-schema checks events and node params go through.
//! - [`ResourceGate`] bounds concurrent node weight; the sequential
//!   runner can never contend, so `acquire` always admits — the cap
//!   arithmetic is kept for parity.

use serde_json::{Map, Value};

use peregrin_core::{Event, EventSchema, js_string};

const LIST_FIELDS: &[&str] = &["allow", "deny", "ports", "hosts", "paths"];
const DICT_LIST_KEYS: &[&str] = &["shell", "http", "python", "webhook"];
const TOP_LIST_KEYS: &[&str] = &["secrets", "agents", "urls"];

/// `ac-safety`'s synthetic event types — exempt from payload-schema
/// validation.
const SYNTHETIC_TYPES: &[&str] = &[
    "start",
    "workflow.done",
    "workflow.cancel",
    "workflow.cancelled",
    "workflow.failed",
    "workflow.cycle_detected",
];

// ---------------------------------------------------------------------------
// JS coercions (ac-safety's asList/asString/asNumber)
// ---------------------------------------------------------------------------

/// `asList` — non-arrays yield `[]`; items stringify via `asString`.
fn as_list(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| item.as_str().unwrap_or_default().to_string())
            .collect(),
        _ => Vec::new(),
    }
}

/// `asString(val, fallback)` — strings pass, everything else falls back.
fn as_string(value: Option<&Value>, fallback: &str) -> String {
    value
        .and_then(Value::as_str)
        .unwrap_or(fallback)
        .to_string()
}

/// `asNumber(val, fallback)` — numbers pass, non-empty strings parse.
fn as_number(value: Option<&Value>, fallback: f64) -> f64 {
    match value {
        Some(Value::Number(n)) => n.as_f64().unwrap_or(fallback),
        Some(Value::String(s)) if !s.trim().is_empty() => s.parse::<f64>().unwrap_or(fallback),
        _ => fallback,
    }
}

/// `deepCopy` — JSON is already a value type in Rust.
fn deep_copy(value: &Value) -> Value {
    value.clone()
}

/// `globMatch` — `^…$`-anchored glob where `*` → `.*` and `?` → `.`
/// (no line-terminator crossing), every other metachar literal.
/// Delegates to [`peregrin_core::glob_match`] — the one matcher every
/// capability consumer shares.
fn glob_match(pattern: &str, value: &str) -> bool {
    peregrin_core::glob_match(pattern, value)
}

// ---------------------------------------------------------------------------
// CapabilityChecker
// ---------------------------------------------------------------------------

/// Workflow+node capability policy evaluation (TS `CapabilityChecker`).
/// `default_policy` reads from `capabilities.default_policy` —
/// `"deny"` under strict, `"allow"` otherwise.
pub struct CapabilityChecker<'a> {
    wf_caps: &'a Map<String, Value>,
    default_policy: String,
}

impl<'a> CapabilityChecker<'a> {
    /// `new CapabilityChecker(workflowCaps, strict)`.
    pub fn new(workflow_caps: &'a Map<String, Value>, strict: bool) -> Self {
        let fallback = if strict { "deny" } else { "allow" };
        Self {
            wf_caps: workflow_caps,
            default_policy: as_string(workflow_caps.get("default_policy"), fallback),
        }
    }

    /// `merge(nodeCaps)` — node keys override workflow keys, except
    /// dict-list keys (`shell`/`http`/`python`/`webhook`) merge their
    /// list fields and top list keys (`secrets`/`agents`/`urls`)
    /// replace — or union when `node.merge === true`.
    #[must_use]
    pub fn merge(&self, node_caps: &Map<String, Value>) -> Map<String, Value> {
        let merge_lists = node_caps.get("merge") == Some(&Value::Bool(true));
        let mut result = Map::new();
        for (key, val) in self.wf_caps {
            result.insert(key.clone(), deep_copy(val));
        }
        for (key, val) in node_caps {
            if key == "merge" {
                continue;
            }
            if DICT_LIST_KEYS.contains(&key.as_str()) {
                let wf = result
                    .get(key)
                    .and_then(Value::as_object)
                    .cloned()
                    .unwrap_or_default();
                let nv = val.as_object().cloned().unwrap_or_default();
                result.insert(
                    key.clone(),
                    Value::Object(Self::merge_dict_list(&wf, &nv, merge_lists)),
                );
            } else if TOP_LIST_KEYS.contains(&key.as_str()) {
                result.insert(
                    key.clone(),
                    Value::Array(
                        Self::merge_list(
                            &as_list(result.get(key)),
                            &as_list(Some(val)),
                            merge_lists,
                        )
                        .into_iter()
                        .map(Value::String)
                        .collect(),
                    ),
                );
            } else {
                result.insert(key.clone(), val.clone());
            }
        }
        result
    }

    /// `check(driver, resolvedParams)` — violations against the merged
    /// policy; empty under `default_policy: allow`.
    #[must_use]
    pub fn check(&self, driver: &str, resolved_params: &Map<String, Value>) -> Vec<String> {
        if self.default_policy == "allow" {
            return Vec::new();
        }
        match driver {
            "shell" => self.check_glob_field(&as_string(resolved_params.get("cmd"), ""), "shell"),
            "http" => self.check_glob_field(&as_string(resolved_params.get("url"), ""), "http"),
            "agent" => self.check_agent(&as_string(resolved_params.get("runner"), "")),
            "python" => self.check_python(resolved_params),
            "webhook" => self.check_webhook(resolved_params),
            _ => Vec::new(),
        }
    }

    /// `filterSecrets` — under deny policy, keep only secrets on the
    /// `secrets` allow list.
    #[must_use]
    pub fn filter_secrets(
        &self,
        secrets: &std::collections::BTreeMap<String, String>,
    ) -> std::collections::BTreeMap<String, String> {
        if self.default_policy == "allow" {
            return secrets.clone();
        }
        let allowed = as_list(self.wf_caps.get("secrets"));
        if allowed.is_empty() {
            return std::collections::BTreeMap::new();
        }
        secrets
            .iter()
            .filter(|(key, _)| allowed.contains(*key))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    fn merge_list(wf_list: &[String], node_list: &[String], merge: bool) -> Vec<String> {
        if !merge {
            return node_list.to_vec();
        }
        let mut combined = wf_list.to_vec();
        for item in node_list {
            if !combined.contains(item) {
                combined.push(item.clone());
            }
        }
        combined
    }

    fn merge_dict_list(
        wf_dict: &Map<String, Value>,
        node_dict: &Map<String, Value>,
        merge: bool,
    ) -> Map<String, Value> {
        let mut result = Map::new();
        for (key, val) in wf_dict {
            result.insert(key.clone(), val.clone());
        }
        for (key, val) in node_dict {
            if LIST_FIELDS.contains(&key.as_str()) && val.is_array() {
                let merged =
                    Self::merge_list(&as_list(result.get(key)), &as_list(Some(val)), merge);
                result.insert(
                    key.clone(),
                    Value::Array(merged.into_iter().map(Value::String).collect()),
                );
            } else {
                result.insert(key.clone(), val.clone());
            }
        }
        result
    }

    fn check_python(&self, resolved_params: &Map<String, Value>) -> Vec<String> {
        let handler = as_string(resolved_params.get("handler"), "");
        if handler.is_empty() {
            return vec!["python node missing handler name".to_string()];
        }
        self.check_glob_field(&handler, "python")
    }

    #[allow(clippy::float_cmp)] // ports compare exact `Number(p) === port`
    fn check_webhook(&self, resolved_params: &Map<String, Value>) -> Vec<String> {
        let caps = self
            .wf_caps
            .get("webhook")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        if caps.is_empty() {
            return vec!["webhook blocked (deny-all, no webhook capabilities)".to_string()];
        }
        let mut violations = Vec::new();
        let port = as_number(resolved_params.get("port"), 8080.0);
        let host = as_string(resolved_params.get("host"), "127.0.0.1");
        let path = as_string(resolved_params.get("path"), "/events");
        let auth = resolved_params.get("auth").and_then(Value::as_object);

        let allowed_ports = as_list(caps.get("ports"));
        if allowed_ports.is_empty() {
            violations.push(format!(
                "port {} blocked (deny-all, no ports list)",
                js_num_str(port)
            ));
        } else if !allowed_ports
            .iter()
            .any(|p| p.parse::<f64>().is_ok_and(|n| n == port))
        {
            violations.push(format!(
                "port {} not in allowed ports {}",
                js_num_str(port),
                Value::Array(
                    allowed_ports
                        .iter()
                        .map(|p| Value::String(p.clone()))
                        .collect::<Vec<_>>()
                )
            ));
        }

        let allowed_hosts = as_list(caps.get("hosts"));
        if allowed_hosts.is_empty() {
            violations.push(format!("host '{host}' blocked (deny-all, no hosts list)"));
        } else if !allowed_hosts.contains(&host) {
            violations.push(format!(
                "host '{host}' not in allowed hosts {}",
                Value::Array(
                    allowed_hosts
                        .iter()
                        .map(|h| Value::String(h.clone()))
                        .collect::<Vec<_>>()
                )
            ));
        }

        let allowed_paths = as_list(caps.get("paths"));
        if allowed_paths.is_empty() {
            violations.push(format!("path '{path}' blocked (deny-all, no paths list)"));
        } else if !allowed_paths.iter().any(|p| glob_match(p, &path)) {
            violations.push(format!(
                "path '{path}' not in allowed paths {}",
                Value::Array(
                    allowed_paths
                        .iter()
                        .map(|p| Value::String(p.clone()))
                        .collect::<Vec<_>>()
                )
            ));
        }

        if caps.get("require_auth") == Some(&Value::Bool(true))
            && auth.is_none_or(|a| a.get("type").is_none() || a.get("secret").is_none())
        {
            violations.push(
                "webhook requires auth config (type + secret) but none or incomplete provided"
                    .to_string(),
            );
        }

        violations
    }

    fn check_glob_field(&self, value: &str, caps_key: &str) -> Vec<String> {
        let caps = self
            .wf_caps
            .get(caps_key)
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let deny = as_list(caps.get("deny"));
        let allow = as_list(caps.get("allow"));
        for pattern in &deny {
            if glob_match(pattern, value) {
                return vec![format!("'{value}' denied by pattern '{pattern}'")];
            }
        }
        if allow.is_empty() {
            return vec![format!("'{value}' blocked (deny-all, no allow list)")];
        }
        if !allow.iter().any(|p| glob_match(p, value)) {
            return vec![format!(
                "'{value}' not in allow list {}",
                Value::Array(
                    allow
                        .iter()
                        .map(|p| Value::String(p.clone()))
                        .collect::<Vec<_>>()
                )
            )];
        }
        Vec::new()
    }

    fn check_agent(&self, runner_name: &str) -> Vec<String> {
        let allowed = as_list(self.wf_caps.get("agents"));
        if allowed.is_empty() {
            return vec![format!(
                "agent '{runner_name}' blocked (deny-all, no agents list)"
            )];
        }
        if !allowed.contains(&runner_name.to_string()) {
            return vec![format!(
                "agent '{runner_name}' not in allowed agents {}",
                Value::Array(
                    allowed
                        .iter()
                        .map(|a| Value::String(a.clone()))
                        .collect::<Vec<_>>()
                )
            )];
        }
        Vec::new()
    }
}

/// `JSON.stringify(list)` of a string list — used in violation messages.
fn js_num_str(value: f64) -> String {
    if value.fract() == 0.0 && value.is_finite() && value.abs() < 1e21 {
        return format!("{}", value as i64);
    }
    value.to_string()
}

// ---------------------------------------------------------------------------
// Schema validation
// ---------------------------------------------------------------------------

/// `typeMatches` — the six JSON primitive names.
fn type_matches(value: &Value, json_type: &str) -> bool {
    match json_type {
        "string" => value.is_string(),
        "integer" => match value {
            Value::Number(n) => {
                n.is_i64()
                    || n.is_u64()
                    || n.as_f64()
                        .is_some_and(|f| f.is_finite() && f.fract() == 0.0)
            }
            _ => false,
        },
        "number" => value.is_number(),
        "boolean" => value.is_boolean(),
        "array" => value.is_array(),
        "object" => value.is_object(),
        "null" => value.is_null(),
        _ => true,
    }
}

fn format_error(path: &str, message: &str) -> String {
    format!("{path}: {message}")
}

/// `validateRecord` — required-property and per-property type checks
/// against a compiled schema dict.
pub fn validate_record(schema: &Map<String, Value>, payload: &Map<String, Value>) -> Vec<String> {
    let mut errors = Vec::new();
    if let Some(Value::Array(required)) = schema.get("required") {
        for name in required {
            let name = js_string(name);
            if !payload.contains_key(&name) {
                errors.push(format_error(&name, "missing required property"));
            }
        }
    }
    if let Some(Value::Object(properties)) = schema.get("properties") {
        for (name, spec) in properties {
            let Some(value) = payload.get(name) else {
                continue;
            };
            if let Some(expected) = spec.get("type").and_then(Value::as_str) {
                if !type_matches(value, expected) {
                    errors.push(format_error(name, &format!("expected type {expected}")));
                }
            }
        }
    }
    errors
}

/// `validateObject` — validate a record against an [`EventSchema`]'s
/// `compiled_schema`; `None` schema means nothing to check.
pub fn validate_object(schema: &EventSchema, value: &Map<String, Value>) -> Vec<String> {
    let Some(Value::Object(compiled)) = schema.compiled_schema() else {
        return Vec::new();
    };
    validate_record(&compiled, value)
}

/// `SchemaValidator` — per-event-type compiled schemas; synthetic
/// runner events and undeclared types pass.
pub struct SchemaValidator {
    validators: Map<String, Value>,
}

impl SchemaValidator {
    /// `hasSchema`.
    #[must_use]
    pub fn has_schema(&self, kind: &str) -> bool {
        self.validators.contains_key(kind)
    }

    /// `validate(event)` — synthetic types skip; a non-object payload
    /// fails `(root)`; otherwise `validateRecord`.
    #[must_use]
    pub fn validate(&self, event: &Event) -> Vec<String> {
        if SYNTHETIC_TYPES.contains(&event.kind.as_str()) {
            return Vec::new();
        }
        let Some(schema) = self.validators.get(&event.kind) else {
            return Vec::new();
        };
        let Some(payload) = event.payload.as_object() else {
            return vec![format_error("(root)", "payload must be an object")];
        };
        match schema.as_object() {
            Some(schema_map) => validate_record(schema_map, payload),
            None => Vec::new(),
        }
    }
}

/// `buildValidator` — `None` when no declared event schema compiles.
#[must_use]
pub fn build_validator(schemas: &[EventSchema]) -> Option<SchemaValidator> {
    let mut validators = Map::new();
    let mut has_any = false;
    for schema in schemas {
        if let Some(compiled) = schema.compiled_schema() {
            has_any = true;
            validators.insert(schema.kind.clone(), compiled);
        }
    }
    if !has_any {
        return None;
    }
    Some(SchemaValidator { validators })
}

// ---------------------------------------------------------------------------
// ResourceGate
// ---------------------------------------------------------------------------

/// `ResourceGate` — bounds concurrent node `resources.weight`. The
/// sequential runner holds at most one reservation at a time, so
/// `acquire` can never block; the TS's load-breaker/OS-probe halves
/// (`maxCpu`/`maxMem`/`minFreeMem`) are inert under the default probe
/// there too and are not ported.
pub struct ResourceGate {
    /// `maxConcurrency` — the TS default is 50.
    pub max_concurrency: u64,
    in_use: u64,
}

impl Default for ResourceGate {
    fn default() -> Self {
        Self {
            max_concurrency: 50,
            in_use: 0,
        }
    }
}

impl ResourceGate {
    /// `acquire(weight)` — cap to `[1, max_concurrency]` and reserve.
    /// Returns the reserved (capped) weight.
    pub fn acquire(&mut self, weight: u64) -> u64 {
        let capped = weight.clamp(1, self.max_concurrency.max(1));
        // The TS waits here until `_inUse + weight <= maxConcurrency`;
        // a sequential runner always has `_inUse == 0` at this point.
        self.in_use += capped;
        capped
    }

    /// `release(weight)` — free a reservation (clamped at 0 like the TS).
    pub fn release(&mut self, weight: u64) {
        let capped = weight.clamp(1, self.max_concurrency.max(1));
        self.in_use = self.in_use.saturating_sub(capped);
    }

    /// `isOverloaded` — always `false` without an OS probe (the TS
    /// default probe reports zero load anyway).
    #[must_use]
    pub fn is_overloaded(&self) -> bool {
        false
    }
}
