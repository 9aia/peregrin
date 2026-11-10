//! The campaign domain model — port of `ac-core`'s `model.ts`.
//!
//! `Workflow` → [`Campaign`], `Node`, `EventSchema`, `FunctionDefinition`,
//! `PythonImport`. The `from_dict`/`to_dict` methods mirror the TS
//! `fromDict`/`toDict` pair: `from_dict` applies the load-time
//! normalization the TS model applies (poll-node tick triggers,
//! self-trigger `max_fires` defaults, scalar coercions), `to_dict`
//! round-trips the canonical YAML dict shape.
//!
//! Fidelity note: TS keeps non-dict scalars in `Record`-typed fields and
//! non-scalar entries in `string[]` fields (type-level casts only); the
//! Rust model drops them — engine behaviour is identical since the
//! engine only reads dict fields/string entries.

use serde_json::{Map, Value};

// ---------------------------------------------------------------------------
// JS coercion helpers shared with the loader expansion passes.
// ---------------------------------------------------------------------------

/// JavaScript `Number.prototype.toString` for a JSON number: integral
/// floats print without a fraction (`300` not `300.0`).
fn js_number(number: &serde_json::Number) -> String {
    if let Some(f) = number.as_f64() {
        if f.is_finite() && f.fract() == 0.0 && f.abs() < 1e21 {
            return format!("{}", f as i64);
        }
    }
    number.to_string()
}

/// JavaScript `String(v)` — how the TS pipeline stringifies interpolated
/// `${include(...)}`/`${snippet(...)}` values and non-string map keys.
/// Objects become `"[object Object]"`, arrays join with `,`.
pub fn js_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => js_number(n),
        Value::String(s) => s.clone(),
        Value::Array(items) => items.iter().map(js_string).collect::<Vec<_>>().join(","),
        Value::Object(_) => "[object Object]".to_string(),
    }
}

/// JavaScript truthiness — used for `Boolean(raw.strict)`, `if (poll)`.
pub fn js_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0 && !f.is_nan()),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

/// Scalar → `String` coercion; non-scalars return `None`. `null` maps to
/// `None` so callers supply the TS fallback (`""` or `[]`).
fn scalar_string(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(js_number(n)),
        Value::Bool(b) => Some(b.to_string()),
        Value::Null | Value::Array(_) | Value::Object(_) => None,
    }
}

/// TS `coerceToString(v)` — string/number/boolean → `String`, else `""`.
fn coerce_to_string(value: Option<&Value>) -> String {
    value.and_then(scalar_string).unwrap_or_default()
}

/// TS `Array.from<string>(v ?? default)` — including the quirk that a
/// scalar *string* value iterates into chars (`on: start` →
/// `["s","t","a","r","t"]`, faithful to the TS runtime behaviour).
/// Non-scalar elements are dropped (TS keeps them but they can never
/// match an event type).
fn string_list(value: Option<&Value>, default: &[&str]) -> Vec<String> {
    match value {
        None | Some(Value::Null) => default.iter().map(ToString::to_string).collect(),
        Some(Value::Array(items)) => items.iter().filter_map(scalar_string).collect(),
        Some(Value::String(s)) => s.chars().map(String::from).collect(),
        Some(_) => Vec::new(),
    }
}

/// `(raw.f as Record) ?? {}` — dicts pass through, anything else is `{}`.
fn map_or_empty(value: Option<&Value>) -> Map<String, Value> {
    value
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default()
}

/// `(raw.f as Record) ?? null` — dicts pass through, anything else `None`.
fn opt_map(value: Option<&Value>) -> Option<Map<String, Value>> {
    value.and_then(Value::as_object).cloned()
}

// ---------------------------------------------------------------------------
// Trigger mode / max fires
// ---------------------------------------------------------------------------

/// `TriggerMode` — `"any"` fires on the first matching trigger, `"all"`
/// waits for every trigger type.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TriggerMode {
    /// Fire when any trigger event arrives.
    #[default]
    Any,
    /// Fire only once every `on` trigger type has arrived.
    All,
}

/// `max_fires` — a positive count or `"unlimited"`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaxFires {
    /// Retire the node after this many fires.
    Fires(u64),
    /// Never retire on fire count.
    Unlimited,
}

impl Default for MaxFires {
    fn default() -> Self {
        Self::Fires(1)
    }
}

impl serde::Serialize for MaxFires {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Fires(n) => serializer.serialize_u64(*n),
            Self::Unlimited => serializer.serialize_str("unlimited"),
        }
    }
}

impl<'de> serde::Deserialize<'de> for MaxFires {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = Value::deserialize(deserializer)?;
        Ok(parse_max_fires(Some(&value), Self::default()))
    }
}

/// TS `parseMaxFires` — `"unlimited"` or a positive integer (float
/// integers count, per `Number.isInteger`), else `default`.
fn parse_max_fires(value: Option<&Value>, default: MaxFires) -> MaxFires {
    match value {
        Some(Value::String(s)) if s == "unlimited" => MaxFires::Unlimited,
        Some(Value::Number(n)) => {
            let positive_integer = n.as_u64().filter(|u| *u > 0).or_else(|| {
                n.as_f64().and_then(|f| {
                    // `f as u64` saturates — JS `max_fires: 1e30` is a
                    // valid (huge) integer; saturation is close enough.
                    if f.fract() == 0.0 && f > 0.0 {
                        Some(f as u64)
                    } else {
                        None
                    }
                })
            });
            positive_integer.map_or(default, MaxFires::Fires)
        }
        _ => default,
    }
}

// ---------------------------------------------------------------------------
// Event
// ---------------------------------------------------------------------------

/// The TS `Event` default payload — `payload ?? {}` on decode.
fn default_payload() -> Value {
    Value::Object(Map::new())
}

/// Epoch milliseconds — `Date.now()` in the TS engine.
fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}

/// A domain event — the unit the bus carries and nodes react to. Port of
/// the TS `Event`: `type` (here `kind`), `payload`, `source`, `id`, `ts`.
/// `source` is the emitting node id (`__runner__` for engine-synthetic
/// events); `id`/`ts` are observational and default on decode.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Event {
    /// Event type tag — routing key for subscribers and node triggers.
    /// Serialized as `type` (the TS field name); `kind` reads too.
    #[serde(rename = "type", alias = "kind", default)]
    pub kind: String,
    /// JSON payload — validated by producers/consumers, opaque here.
    #[serde(default = "default_payload")]
    pub payload: serde_json::Value,
    /// Emitting node id, or `__runner__` for engine-synthetic events.
    #[serde(default)]
    pub source: String,
    /// Event id — `crypto.randomUUID().slice(0, 12)` in the TS.
    #[serde(default)]
    pub id: String,
    /// Emission time — `Date.now()` epoch milliseconds.
    #[serde(default)]
    pub ts: i64,
}

impl Event {
    /// `new Event(type, payload, source)` — fresh `id` and `ts`.
    #[allow(clippy::needless_pass_by_value)] // payload is stored as-is
    pub fn new(
        kind: impl Into<String>,
        payload: serde_json::Value,
        source: impl Into<String>,
    ) -> Self {
        Self {
            kind: kind.into(),
            payload,
            source: source.into(),
            id: uuid::Uuid::new_v4().to_string()[..12].to_string(),
            ts: now_millis(),
        }
    }

    /// `Event.start()` — the synthetic run trigger.
    #[must_use]
    pub fn start() -> Self {
        Self::new("start", default_payload(), "__runner__")
    }

    /// `Event.done()` — the successful-run terminator.
    #[must_use]
    pub fn done() -> Self {
        Self::new("workflow.done", default_payload(), "__runner__")
    }

    /// `Event.cancel(reason)` — a cancellation request event.
    #[must_use]
    pub fn cancel(reason: &str) -> Self {
        Self::new(
            "workflow.cancel",
            serde_json::json!({ "reason": reason }),
            "__runner__",
        )
    }

    /// `Event.nodeFailed(nodeId, error)`.
    pub fn node_failed(node_id: &str, error: &str) -> Self {
        Self::new(
            "node.failed",
            serde_json::json!({ "error": error, "node": node_id }),
            node_id,
        )
    }

    /// `Event.nodeExhausted(nodeId, maxFires)` — the `max_fires` cap
    /// retire signal (ADR-0005).
    pub fn node_exhausted(node_id: &str, max_fires: MaxFires) -> Self {
        Self::new(
            "node.exhausted",
            serde_json::json!({ "node": node_id, "maxFires": max_fires }),
            node_id,
        )
    }

    /// `Event.workflowFailed(nodeId, error)` — runner-sourced.
    pub fn workflow_failed(node_id: &str, error: &str) -> Self {
        Self::new(
            "workflow.failed",
            serde_json::json!({ "node": node_id, "error": error }),
            "__runner__",
        )
    }

    /// `Event.eventInvalid(eventType, source, errors, payload)`.
    #[allow(clippy::needless_pass_by_value)] // signature mirrors the TS
    pub fn event_invalid(
        kind: &str,
        source: &str,
        errors: &[String],
        payload: serde_json::Value,
    ) -> Self {
        Self::new(
            "event.invalid",
            serde_json::json!({
                "eventType": kind,
                "errors": errors,
                "payload": payload,
            }),
            source,
        )
    }

    /// `Event.capabilityViolation(nodeId, violations)`.
    pub fn capability_violation(node_id: &str, violations: &[String]) -> Self {
        Self::new(
            "capability.violation",
            serde_json::json!({ "node": node_id, "violations": violations }),
            node_id,
        )
    }
}

// ---------------------------------------------------------------------------
// EventSchema
// ---------------------------------------------------------------------------

/// `EventSchema` — a declared event type plus its field/schema contract.
/// Serialized under the `type` key (TS field name).
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EventSchema {
    /// Event type tag (`type` in YAML dicts).
    #[serde(rename = "type")]
    pub kind: String,
    /// Shorthand field contract: `name: <json-type>[?]`.
    #[serde(default)]
    pub fields: Map<String, Value>,
    /// Full JSON-schema override (wins over `fields` when non-empty).
    #[serde(default)]
    pub schema: Map<String, Value>,
    /// Free-form metadata.
    #[serde(default)]
    pub annotations: Map<String, Value>,
}

impl EventSchema {
    /// TS `EventSchema.fromDict`.
    #[must_use]
    pub fn from_dict(raw: &Value) -> Self {
        let empty = Map::new();
        let map = raw.as_object().unwrap_or(&empty);
        Self::from_map(map, &coerce_to_string(map.get("type")))
    }

    /// Build from a mapping with an explicit type tag — used by the node
    /// `input`/`output` forms where the type comes from the dict key.
    fn from_map(map: &Map<String, Value>, kind: &str) -> Self {
        Self {
            kind: kind.to_string(),
            fields: map_or_empty(map.get("fields")),
            schema: map_or_empty(map.get("schema")),
            annotations: map_or_empty(map.get("annotations")),
        }
    }

    /// TS `compiledSchema` — `schema` wins; else `fields` compiles to a
    /// JSON-schema object (`?` suffix marks optional); else `None`.
    #[must_use]
    pub fn compiled_schema(&self) -> Option<Value> {
        // JSON-schema primitive names `fields:` understands; anything
        // else compiles to `string` (TS `compiledSchema`).
        const KNOWN: &[&str] = &[
            "string", "integer", "number", "boolean", "array", "object", "null",
        ];
        if !self.schema.is_empty() {
            return Some(Value::Object(self.schema.clone()));
        }
        if self.fields.is_empty() {
            return None;
        }
        let mut properties = Map::new();
        let mut required = Vec::new();
        for (name, type_value) in &self.fields {
            let type_str = type_value.as_str().unwrap_or_default();
            let (optional, json_type) = match type_str.strip_suffix('?') {
                Some(t) => (true, t),
                None => (false, type_str),
            };
            let mut prop = Map::new();
            let resolved = if KNOWN.contains(&json_type) {
                json_type
            } else {
                "string"
            };
            prop.insert("type".to_string(), Value::String(resolved.to_string()));
            properties.insert(name.clone(), Value::Object(prop));
            if !optional {
                required.push(Value::String(name.clone()));
            }
        }
        let mut compiled = Map::new();
        compiled.insert("type".to_string(), Value::String("object".to_string()));
        compiled.insert("properties".to_string(), Value::Object(properties));
        if !required.is_empty() {
            compiled.insert("required".to_string(), Value::Array(required));
        }
        Some(Value::Object(compiled))
    }

    /// TS `toDict` — `type` + `fields` always; `schema`/`annotations`
    /// only when non-empty.
    #[must_use]
    pub fn to_dict(&self) -> Value {
        let mut d = Map::new();
        d.insert("type".to_string(), Value::String(self.kind.clone()));
        d.insert("fields".to_string(), Value::Object(self.fields.clone()));
        if !self.schema.is_empty() {
            d.insert("schema".to_string(), Value::Object(self.schema.clone()));
        }
        if !self.annotations.is_empty() {
            d.insert(
                "annotations".to_string(),
                Value::Object(self.annotations.clone()),
            );
        }
        Value::Object(d)
    }
}

// ---------------------------------------------------------------------------
// PythonImport / FunctionDefinition
// ---------------------------------------------------------------------------

/// `PythonImport` — a `file` imported under an optional `as` alias.
/// Serializes as the bare filename when the alias is empty (TS `toDict`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PythonImport {
    /// Module file path.
    pub file: String,
    /// Import alias (TS `as`).
    pub alias: String,
}

impl serde::Serialize for PythonImport {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if self.alias.is_empty() {
            return serializer.serialize_str(&self.file);
        }
        let mut map = Map::new();
        map.insert("file".to_string(), Value::String(self.file.clone()));
        map.insert("as".to_string(), Value::String(self.alias.clone()));
        map.serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for PythonImport {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = Value::deserialize(deserializer)?;
        Ok(match &value {
            Value::String(file) => Self {
                file: file.clone(),
                alias: String::new(),
            },
            Value::Object(map) => Self {
                file: coerce_to_string(map.get("file")),
                alias: coerce_to_string(map.get("as")),
            },
            _ => Self::default(),
        })
    }
}

/// TS `parsePythonImports` — a single value wraps into a list; strings
/// become `{file, as: ""}`; dicts need a string `file`.
fn parse_python_imports(value: Option<&Value>) -> Vec<PythonImport> {
    let items: &[Value] = match value {
        None | Some(Value::Null) => return Vec::new(),
        Some(Value::Array(items)) => items,
        Some(other) => std::slice::from_ref(other),
    };
    items
        .iter()
        .filter_map(|item| match item {
            Value::String(file) => Some(PythonImport {
                file: file.clone(),
                alias: String::new(),
            }),
            Value::Object(map) if map.get("file").is_some_and(Value::is_string) => {
                Some(PythonImport {
                    file: coerce_to_string(map.get("file")),
                    alias: coerce_to_string(map.get("as")),
                })
            }
            _ => None,
        })
        .collect()
}

/// `FunctionDefinition` — a reusable YAML-declared function
/// (`functions:` / `*.fn.yml`).
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FunctionDefinition {
    /// Function name — the dict key when `functions:` is a mapping.
    pub name: String,
    /// Driver that runs it.
    pub driver: String,
    /// Call parameters.
    #[serde(default)]
    pub params: Map<String, Value>,
    /// Capability contract.
    #[serde(default)]
    pub capabilities: Map<String, Value>,
    /// Whether the call needs a capability grant (`requires_capability`).
    #[serde(default)]
    pub requires_capability: bool,
    /// Cacheable/pure marker.
    #[serde(default)]
    pub deterministic: bool,
    /// Declared return type, if any.
    #[serde(default)]
    pub returns: Option<String>,
}

impl FunctionDefinition {
    /// TS `FunctionDefinition.fromDict`.
    #[must_use]
    pub fn from_dict(raw: &Value) -> Self {
        let empty = Map::new();
        let map = raw.as_object().unwrap_or(&empty);
        Self {
            name: coerce_to_string(map.get("name")),
            driver: coerce_to_string(map.get("driver")),
            params: map_or_empty(map.get("params")),
            capabilities: map_or_empty(map.get("capabilities")),
            requires_capability: map.get("requires_capability").is_some_and(js_truthy),
            deterministic: map.get("deterministic").is_some_and(js_truthy),
            returns: map
                .get("returns")
                .and_then(Value::as_str)
                .map(str::to_string),
        }
    }

    /// TS `FunctionDefinition.toDict`.
    #[must_use]
    pub fn to_dict(&self) -> Value {
        let mut d = Map::new();
        d.insert("name".to_string(), Value::String(self.name.clone()));
        d.insert("driver".to_string(), Value::String(self.driver.clone()));
        if !self.params.is_empty() {
            d.insert("params".to_string(), Value::Object(self.params.clone()));
        }
        if !self.capabilities.is_empty() {
            d.insert(
                "capabilities".to_string(),
                Value::Object(self.capabilities.clone()),
            );
        }
        if self.requires_capability {
            d.insert("requires_capability".to_string(), Value::Bool(true));
        }
        if self.deterministic {
            d.insert("deterministic".to_string(), Value::Bool(true));
        }
        if let Some(returns) = &self.returns {
            d.insert("returns".to_string(), Value::String(returns.clone()));
        }
        Value::Object(d)
    }
}

// ---------------------------------------------------------------------------
// Node
// ---------------------------------------------------------------------------

/// `parseNodeInput` — a node `input:` mapping compiles to a typeless
/// `EventSchema`; anything else is `None`.
fn parse_node_input(value: Option<&Value>) -> Option<EventSchema> {
    value
        .and_then(Value::as_object)
        .map(|map| EventSchema::from_map(map, ""))
}

/// `parseNodeOutput` — `output:` maps each event-type key to an
/// `EventSchema`; non-dict bodies declare a bare type.
fn parse_node_output(value: Option<&Value>) -> Vec<EventSchema> {
    value
        .and_then(Value::as_object)
        .map_or_else(Vec::new, |map| {
            map.iter()
                .map(|(kind, body)| match body {
                    Value::Object(m) => EventSchema::from_map(m, kind),
                    _ => EventSchema {
                        kind: kind.clone(),
                        ..EventSchema::default()
                    },
                })
                .collect()
        })
}

/// A node in a campaign graph — edges come from `on` triggers, not
/// explicit links. Port of the TS `Node` model.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Node {
    /// Node id — unique within the campaign.
    pub id: String,
    /// Driver name (`shell`, `agent`, …).
    pub driver: String,
    /// Optional handler reference within the driver.
    #[serde(default)]
    pub handler: String,
    /// Trigger event types.
    #[serde(default)]
    pub on: Vec<String>,
    /// Event types this node may emit.
    #[serde(default)]
    pub emits: Vec<String>,
    /// Trigger semantics — `any` or `all`.
    #[serde(default)]
    pub mode: TriggerMode,
    /// Driver parameters (kept as an untyped dict).
    #[serde(default)]
    pub params: serde_json::Value,
    /// Fire cap — retires the node after N fires.
    #[serde(default)]
    pub max_fires: MaxFires,
    /// Stop-listening triggers.
    #[serde(default)]
    pub until: Vec<String>,
    /// Retire-on triggers.
    #[serde(default)]
    pub retire_on: Vec<String>,
    /// Retry policy.
    #[serde(default)]
    pub retry: Map<String, Value>,
    /// Circuit-breaker policy.
    #[serde(default)]
    pub circuit_breaker: Map<String, Value>,
    /// Polling config — presence flips the node into poll mode.
    #[serde(default)]
    pub poll: Option<Map<String, Value>>,
    /// Fan-in (join) config.
    #[serde(default)]
    pub fan_in: Option<Map<String, Value>>,
    /// Event types forwarded verbatim.
    #[serde(default)]
    pub forward: Vec<String>,
    /// Resource declarations.
    #[serde(default)]
    pub resources: Map<String, Value>,
    /// Capability requirements.
    #[serde(default)]
    pub capabilities: Map<String, Value>,
    /// Per-node state seed.
    #[serde(default)]
    pub state: Map<String, Value>,
    /// Free-form metadata (ADR-0009).
    #[serde(default)]
    pub annotations: Map<String, Value>,
    /// Input contract.
    #[serde(default)]
    pub input: Option<EventSchema>,
    /// Output contracts per emitted type.
    #[serde(default)]
    pub output: Vec<EventSchema>,
}

impl Default for Node {
    fn default() -> Self {
        Self {
            id: String::new(),
            driver: String::new(),
            handler: String::new(),
            on: Vec::new(),
            emits: Vec::new(),
            mode: TriggerMode::default(),
            params: Value::Object(Map::new()),
            max_fires: MaxFires::default(),
            until: Vec::new(),
            retire_on: Vec::new(),
            retry: Map::new(),
            circuit_breaker: Map::new(),
            poll: None,
            fan_in: None,
            forward: Vec::new(),
            resources: Map::new(),
            capabilities: Map::new(),
            state: Map::new(),
            annotations: Map::new(),
            input: None,
            output: Vec::new(),
        }
    }
}

impl Node {
    /// TS `Node.fromDict` — including the poll-node normalization
    /// (synthetic `<id>.tick` trigger/emit, `poll.until` merge,
    /// `params.interval` backfill) and the `cli`/`polling` self-trigger
    /// `max_fires` default of `100`.
    #[must_use]
    pub fn from_dict(raw: &Value) -> Self {
        let empty = Map::new();
        let raw = raw.as_object().unwrap_or(&empty);
        let id = coerce_to_string(raw.get("id"));
        let driver = coerce_to_string(raw.get("driver"));

        // `if (poll)` — JS truthiness, so `poll: {}` takes the branch
        // while `poll: false`/`poll: 0` do not.
        if raw.get("poll").is_some_and(js_truthy) {
            let poll = raw.get("poll").and_then(Value::as_object);
            let tick = format!("{id}.tick");
            let mut on = string_list(raw.get("on"), &["start"]);
            if !on.contains(&tick) {
                on.push(tick.clone());
            }
            let mut emits = string_list(raw.get("emits"), &[]);
            if !emits.contains(&tick) {
                emits.push(tick);
            }
            let mut until = string_list(raw.get("until"), &[]);
            for event in string_list(poll.and_then(|p| p.get("until")), &[]) {
                if !until.contains(&event) {
                    until.push(event);
                }
            }
            let mut params = map_or_empty(raw.get("params"));
            if let Some(interval) = poll
                .and_then(|p| p.get("interval"))
                .filter(|v| v.is_number())
            {
                params
                    .entry("interval".to_string())
                    .or_insert_with(|| interval.clone());
            }
            return Self {
                id,
                driver,
                handler: coerce_to_string(raw.get("handler")),
                on,
                emits,
                mode: trigger_mode(raw.get("mode")),
                params: Value::Object(params),
                max_fires: parse_max_fires(raw.get("max_fires"), MaxFires::Fires(1)),
                until,
                retire_on: string_list(raw.get("retire_on"), &[]),
                retry: map_or_empty(raw.get("retry")),
                circuit_breaker: map_or_empty(raw.get("circuit_breaker")),
                poll: opt_map(raw.get("poll")),
                fan_in: opt_map(raw.get("fan_in")),
                forward: string_list(raw.get("forward"), &[]),
                resources: map_or_empty(raw.get("resources")),
                capabilities: map_or_empty(raw.get("capabilities")),
                state: map_or_empty(raw.get("state")),
                annotations: map_or_empty(raw.get("annotations")),
                input: parse_node_input(raw.get("input")),
                output: parse_node_output(raw.get("output")),
            };
        }

        let params = map_or_empty(raw.get("params"));
        let is_self_trigger = (driver == "cli" || driver == "polling")
            && params
                .get("mode")
                .is_none_or(|m| m.as_str() != Some("interval"));
        let mut on = string_list(raw.get("on"), &[]);
        let mut emits = string_list(raw.get("emits"), &[]);
        // `driver: schedule` recurs through a synthetic `{id}.fire`
        // re-arm trigger — the `poll:` `<id>.tick` convention. Each
        // landed fire re-invokes the driver, which arms the next
        // cron occurrence (ADR-0018): `max_fires` then bounds
        // emissions exactly (`1` = one-shot, `unlimited` — a
        // `schedule` node's default — = recurring), and `until`/
        // `retire_on` stop the loop through the usual machinery.
        let is_schedule = driver == "schedule";
        if is_schedule {
            for internal in [format!("{id}.fire"), format!("{id}.scheduled")] {
                if !emits.contains(&internal) {
                    emits.push(internal);
                }
            }
            let fire = format!("{id}.fire");
            if !on.contains(&fire) {
                on.push(fire);
            }
        }
        let default_max_fires = if is_schedule {
            MaxFires::Unlimited
        } else {
            MaxFires::Fires(if is_self_trigger { 100 } else { 1 })
        };
        Self {
            id,
            driver,
            handler: coerce_to_string(raw.get("handler")),
            on,
            emits,
            mode: trigger_mode(raw.get("mode")),
            params: Value::Object(params),
            max_fires: parse_max_fires(raw.get("max_fires"), default_max_fires),
            until: string_list(raw.get("until"), &[]),
            retire_on: string_list(raw.get("retire_on"), &[]),
            retry: map_or_empty(raw.get("retry")),
            circuit_breaker: map_or_empty(raw.get("circuit_breaker")),
            poll: opt_map(raw.get("poll")),
            fan_in: opt_map(raw.get("fan_in")),
            forward: string_list(raw.get("forward"), &[]),
            resources: map_or_empty(raw.get("resources")),
            capabilities: map_or_empty(raw.get("capabilities")),
            state: map_or_empty(raw.get("state")),
            annotations: map_or_empty(raw.get("annotations")),
            input: parse_node_input(raw.get("input")),
            output: parse_node_output(raw.get("output")),
        }
    }

    /// TS `Node.toDict` — canonical YAML dict shape; optional keys
    /// appear only when non-default.
    #[must_use]
    pub fn to_dict(&self) -> Value {
        let mut d = Map::new();
        d.insert("id".to_string(), Value::String(self.id.clone()));
        d.insert("driver".to_string(), Value::String(self.driver.clone()));
        d.insert("handler".to_string(), Value::String(self.handler.clone()));
        d.insert(
            "on".to_string(),
            Value::Array(self.on.iter().map(|s| Value::String(s.clone())).collect()),
        );
        d.insert(
            "emits".to_string(),
            Value::Array(
                self.emits
                    .iter()
                    .map(|s| Value::String(s.clone()))
                    .collect(),
            ),
        );
        d.insert(
            "mode".to_string(),
            serde_json::to_value(self.mode).unwrap_or(Value::Null),
        );
        d.insert("params".to_string(), self.params.clone());
        if self.max_fires != MaxFires::default() {
            d.insert(
                "max_fires".to_string(),
                serde_json::to_value(self.max_fires).unwrap_or(Value::Null),
            );
        }
        if !self.until.is_empty() {
            d.insert(
                "until".to_string(),
                Value::Array(
                    self.until
                        .iter()
                        .map(|s| Value::String(s.clone()))
                        .collect(),
                ),
            );
        }
        if !self.retire_on.is_empty() {
            d.insert(
                "retire_on".to_string(),
                Value::Array(
                    self.retire_on
                        .iter()
                        .map(|s| Value::String(s.clone()))
                        .collect(),
                ),
            );
        }
        if !self.retry.is_empty() {
            d.insert("retry".to_string(), Value::Object(self.retry.clone()));
        }
        if !self.circuit_breaker.is_empty() {
            d.insert(
                "circuit_breaker".to_string(),
                Value::Object(self.circuit_breaker.clone()),
            );
        }
        if let Some(poll) = &self.poll {
            d.insert("poll".to_string(), Value::Object(poll.clone()));
        }
        if let Some(fan_in) = &self.fan_in {
            d.insert("fan_in".to_string(), Value::Object(fan_in.clone()));
        }
        if !self.forward.is_empty() {
            d.insert(
                "forward".to_string(),
                Value::Array(
                    self.forward
                        .iter()
                        .map(|s| Value::String(s.clone()))
                        .collect(),
                ),
            );
        }
        if !self.resources.is_empty() {
            d.insert(
                "resources".to_string(),
                Value::Object(self.resources.clone()),
            );
        }
        if !self.capabilities.is_empty() {
            d.insert(
                "capabilities".to_string(),
                Value::Object(self.capabilities.clone()),
            );
        }
        if !self.state.is_empty() {
            d.insert("state".to_string(), Value::Object(self.state.clone()));
        }
        if let Some(input) = &self.input {
            // `input` carries no `type` key — strip it like the TS.
            if let Value::Object(mut schema) = input.to_dict() {
                schema.remove("type");
                d.insert("input".to_string(), Value::Object(schema));
            }
        }
        if !self.output.is_empty() {
            let mut output = Map::new();
            for schema in &self.output {
                if let Value::Object(mut d) = schema.to_dict() {
                    if let Some(Value::String(kind)) = d.remove("type") {
                        output.insert(kind, Value::Object(d));
                    }
                }
            }
            d.insert("output".to_string(), Value::Object(output));
        }
        if !self.annotations.is_empty() {
            d.insert(
                "annotations".to_string(),
                Value::Object(self.annotations.clone()),
            );
        }
        Value::Object(d)
    }
}

/// `(raw.mode as TriggerMode) ?? "any"` — only `"all"` is non-default.
fn trigger_mode(value: Option<&Value>) -> TriggerMode {
    match value.and_then(Value::as_str) {
        Some("all") => TriggerMode::All,
        _ => TriggerMode::Any,
    }
}

// ---------------------------------------------------------------------------
// Campaign
// ---------------------------------------------------------------------------

/// `Workflow.fromDict` failure — mirrors the `TypeError` the TS throws
/// when `nodes`/`events` is present but not an array; `supersede:`
/// schema violations carry the serde detail.
#[derive(Debug)]
pub struct ModelError {
    /// The offending top-level field.
    pub field: &'static str,
    /// Validation detail beyond `"'{field}' must be an array"` —
    /// populated by `supersede:` schema errors.
    pub detail: Option<String>,
}

impl ModelError {
    /// `'{field}' must be an array` — the TS `TypeError` shape.
    fn not_array(field: &'static str) -> Self {
        Self {
            field,
            detail: None,
        }
    }
}

impl std::fmt::Display for ModelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.detail {
            Some(detail) => write!(f, "'{}' is invalid: {detail}", self.field),
            None => write!(f, "'{}' must be an array", self.field),
        }
    }
}

impl std::error::Error for ModelError {}

/// `functions:` normalization — array keeps its dict entries, mapping
/// becomes `{name, ...body}` records (TS `Workflow.fromDict`).
fn function_records(value: Option<&Value>) -> Vec<Value> {
    match value {
        Some(Value::Array(items)) => items.iter().filter(|v| v.is_object()).cloned().collect(),
        Some(Value::Object(map)) => map
            .iter()
            .map(|(name, body)| {
                let mut record = body.as_object().cloned().unwrap_or_default();
                record.insert("name".to_string(), Value::String(name.clone()));
                Value::Object(record)
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// `supersede.mode` — how pending jobs dedupe by supersede key.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SupersedeMode {
    /// A new event replaces a pending job with the same key — only
    /// the newest trigger for a target runs.
    #[default]
    LatestWins,
    /// Every event appends; same-key events each get a run.
    Fifo,
}

/// The `supersede:` campaign config — which runs happen when trigger
/// events outpace the executor (docs/reference/queue-supersede.md).
///
/// ```yaml
/// supersede:
///   key: "{repo}:{ref}"     # template over the trigger event payload
///   mode: latest-wins       # | fifo
///   cancel_running: true    # cancel the active run vs queue behind it
/// ```
///
/// This is the schema mirror — the scheduler (`Queue` + the `{path}`
/// key-template resolver) lives in `peregrin-queue`, which depends on
/// `peregrin-core` and can't be depended on in return. The node
/// converts the spec at run setup (`peregrin_queue::Supersede::from`)
/// and `peregrin_node::supervisor::supervise` drives it.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Supersede {
    /// Key template over the trigger event's `payload` —
    /// `{dotted.path}` placeholders with JS-style stringification;
    /// misses stay verbatim. An empty template resolves to a constant
    /// key — every trigger targets the same logical run.
    #[serde(default)]
    pub key: String,
    /// Pending-job dedupe mode.
    #[serde(default)]
    pub mode: SupersedeMode,
    /// `true`: a new trigger marks the *active* run superseded — the
    /// queue raises a one-shot cancel signal the supervisor delivers
    /// as a `workflow.cancel` on the run's own ingress channel.
    /// `false`: new triggers always queue behind the active run.
    #[serde(default)]
    pub cancel_running: bool,
}

/// A campaign — a named graph of nodes plus its triggers (port of the
/// TS `Workflow`).
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Campaign {
    /// Campaign name.
    #[serde(default)]
    pub name: String,
    /// Declared event contracts.
    #[serde(default)]
    pub events: Vec<EventSchema>,
    /// Graph nodes.
    #[serde(default)]
    pub nodes: Vec<Node>,
    /// Reusable function definitions.
    #[serde(default)]
    pub functions: Vec<FunctionDefinition>,
    /// Campaign-level config (exposed to templates as `${config.*}`).
    #[serde(default)]
    pub config: Map<String, Value>,
    /// Included file list — serialized under `include` (`includes` reads too).
    #[serde(default, rename = "include", alias = "includes")]
    pub imports: Vec<String>,
    /// Python module imports (`import:`).
    #[serde(default, rename = "import")]
    pub python_imports: Vec<PythonImport>,
    /// Event types that terminate the campaign.
    #[serde(default)]
    pub stop_on: Vec<String>,
    /// Payload schemas keyed by event type.
    #[serde(default)]
    pub payload_schemas: Map<String, Value>,
    /// Campaign-level capability policy.
    #[serde(default)]
    pub capabilities: Map<String, Value>,
    /// Strict validation mode (ADR-0001).
    #[serde(default)]
    pub strict: bool,
    /// Campaign-level state seed (`${state.*}`).
    #[serde(default)]
    pub state: Map<String, Value>,
    /// Free-form metadata (ADR-0009).
    #[serde(default)]
    pub annotations: Map<String, Value>,
    /// The `supersede:` queue policy (docs/reference/
    /// queue-supersede.md) — which runs happen when trigger events
    /// outpace the executor. `None` keeps single-run semantics: one
    /// long-running run contains ingress and workers. Declared *plus*
    /// an ingress-driver node engages supervisor mode — persistent
    /// listeners feed a `RunSupervisor` and each trigger spawns a
    /// fresh run over the worker nodes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supersede: Option<Supersede>,
    /// `runs_on:` — runner labels this campaign requires (PL-007 C4
    /// fleet scheduling). Every entry must appear in the hosting
    /// node's advertised capability list (`/api/node` `capabilities`,
    /// fed by `PEREGRIN_RUNNER_LABELS`) for the node to claim the
    /// campaign — `campaign.start` ops and `campaign serve` hosting
    /// both enforce the match. Empty = any node may run it.
    ///
    /// ```yaml
    /// runs_on: [runner:linux, gpu]
    /// ```
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub runs_on: Vec<String>,
    /// Directory the campaign was loaded from — the anchor `include:`
    /// and `subworkflow` `params.path` resolve against (ADR-0022).
    /// Provenance metadata the loader stamps after [`Campaign::from_dict`];
    /// not part of the serialized model and `None` for campaigns built
    /// programmatically.
    #[serde(skip)]
    pub source_dir: Option<std::path::PathBuf>,
}

impl Campaign {
    /// TS `Workflow.fromDict` — the load-time normalization applied to
    /// the expanded raw mapping.
    ///
    /// # Errors
    /// [`ModelError`] when `nodes`/`events` is present but not an array
    /// (the TS `TypeError` on `.map` over a non-array).
    pub fn from_dict(raw: &Value) -> Result<Self, ModelError> {
        let empty = Map::new();
        let raw = raw.as_object().unwrap_or(&empty);
        let events = match raw.get("events") {
            None | Some(Value::Null) => Vec::new(),
            Some(Value::Array(items)) => items.iter().map(EventSchema::from_dict).collect(),
            Some(_) => return Err(ModelError::not_array("events")),
        };
        let nodes = match raw.get("nodes") {
            None | Some(Value::Null) => Vec::new(),
            Some(Value::Array(items)) => items.iter().map(Node::from_dict).collect(),
            Some(_) => return Err(ModelError::not_array("nodes")),
        };
        // `supersede:` validates strictly — it's the schema mirror of
        // `peregrin_queue::Supersede` (`deny_unknown_fields`, a closed
        // `mode` enum): a typo'd key or mode is a load error, not a
        // silently-misparsed policy.
        let supersede = match raw.get("supersede") {
            None | Some(Value::Null) => None,
            Some(value) => Some(serde_json::from_value::<Supersede>(value.clone()).map_err(
                |e| ModelError {
                    field: "supersede",
                    detail: Some(e.to_string()),
                },
            )?),
        };
        // `raw.include ?? raw.includes ?? []` — JS `??` also falls
        // through on `null`.
        let imports = match raw.get("include") {
            Some(Value::Null) | None => string_list(raw.get("includes"), &[]),
            Some(v) => string_list(Some(v), &[]),
        };
        Ok(Self {
            name: coerce_to_string(raw.get("name")),
            events,
            nodes,
            functions: function_records(raw.get("functions"))
                .iter()
                .map(FunctionDefinition::from_dict)
                .collect(),
            config: map_or_empty(raw.get("config")),
            imports,
            python_imports: parse_python_imports(raw.get("import")),
            stop_on: string_list(raw.get("stop_on"), &[]),
            payload_schemas: map_or_empty(raw.get("payload_schemas")),
            capabilities: map_or_empty(raw.get("capabilities")),
            strict: raw.get("strict").is_some_and(js_truthy),
            state: map_or_empty(raw.get("state")),
            annotations: map_or_empty(raw.get("annotations")),
            supersede,
            runs_on: string_list(raw.get("runs_on"), &[]),
            source_dir: None,
        })
    }

    /// TS `Workflow.toDict`.
    #[must_use]
    pub fn to_dict(&self) -> Value {
        let mut d = Map::new();
        d.insert("name".to_string(), Value::String(self.name.clone()));
        d.insert(
            "events".to_string(),
            Value::Array(self.events.iter().map(EventSchema::to_dict).collect()),
        );
        d.insert(
            "nodes".to_string(),
            Value::Array(self.nodes.iter().map(Node::to_dict).collect()),
        );
        d.insert("config".to_string(), Value::Object(self.config.clone()));
        if !self.functions.is_empty() {
            let mut functions = Map::new();
            for f in &self.functions {
                functions.insert(f.name.clone(), f.to_dict());
            }
            d.insert("functions".to_string(), Value::Object(functions));
        }
        if !self.imports.is_empty() {
            d.insert(
                "include".to_string(),
                Value::Array(
                    self.imports
                        .iter()
                        .map(|s| Value::String(s.clone()))
                        .collect(),
                ),
            );
        }
        if !self.python_imports.is_empty() {
            d.insert(
                "import".to_string(),
                serde_json::to_value(&self.python_imports).unwrap_or(Value::Null),
            );
        }
        if !self.stop_on.is_empty() {
            d.insert(
                "stop_on".to_string(),
                Value::Array(
                    self.stop_on
                        .iter()
                        .map(|s| Value::String(s.clone()))
                        .collect(),
                ),
            );
        }
        if !self.capabilities.is_empty() {
            d.insert(
                "capabilities".to_string(),
                Value::Object(self.capabilities.clone()),
            );
        }
        if self.strict {
            d.insert("strict".to_string(), Value::Bool(true));
        }
        if !self.state.is_empty() {
            d.insert("state".to_string(), Value::Object(self.state.clone()));
        }
        if !self.annotations.is_empty() {
            d.insert(
                "annotations".to_string(),
                Value::Object(self.annotations.clone()),
            );
        }
        if !self.runs_on.is_empty() {
            d.insert(
                "runs_on".to_string(),
                Value::Array(
                    self.runs_on
                        .iter()
                        .map(|s| Value::String(s.clone()))
                        .collect(),
                ),
            );
        }
        if let Some(supersede) = &self.supersede {
            d.insert(
                "supersede".to_string(),
                serde_json::to_value(supersede).unwrap_or(Value::Null),
            );
        }
        Value::Object(d)
    }

    /// TS `Workflow.merge` — pull in `other`'s missing event types,
    /// function names and config keys.
    pub fn merge(&mut self, other: &Self) {
        for event in &other.events {
            if !self.events.iter().any(|e| e.kind == event.kind) {
                self.events.push(event.clone());
            }
        }
        for function in &other.functions {
            if !self.functions.iter().any(|f| f.name == function.name) {
                self.functions.push(function.clone());
            }
        }
        for (key, value) in &other.config {
            self.config
                .entry(key.clone())
                .or_insert_with(|| value.clone());
        }
        if self.supersede.is_none() {
            self.supersede.clone_from(&other.supersede);
        }
        for label in &other.runs_on {
            if !self.runs_on.contains(label) {
                self.runs_on.push(label.clone());
            }
        }
    }

    /// Whether this node's advertised capability labels satisfy the
    /// campaign's `runs_on` requirements — every required label must
    /// be present (AND semantics; empty `runs_on` matches anywhere).
    #[must_use]
    pub fn runs_on_matches(&self, node_labels: &[String]) -> bool {
        self.runs_on
            .iter()
            .all(|req| node_labels.iter().any(|l| l == req))
    }
}
