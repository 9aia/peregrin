use ac_runtime::FunctionRepr;
use serde_json::Value as JsonValue;

/// A serializable function value — pure, no closures.
///
/// A first-class function is just its parameter schemas, optional
/// return schema, and a body expression string. Since the body is
/// a pure expression (no captured state), the whole value is plain
/// data that serializes as JSON.
///
/// A function value is detected by the presence of a `body` string
/// field in a JSON object. No special tag is needed.
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionValue {
    /// `(param_name, raw_schema)` pairs, in declaration order.
    pub params: Vec<(String, JsonValue)>,
    /// Raw schema value for the return type, if declared.
    pub return_schema: Option<JsonValue>,
    /// The body expression string (e.g. `"mul(x, 2)"`).
    pub body: String,
    /// Original pattern JSON for each parameter, in declaration order.
    pub patterns: Vec<JsonValue>,
}

impl From<FunctionValue> for JsonValue {
    /// Encodes this function value as a plain JSON object.
    fn from(value: FunctionValue) -> Self {
        value.to_json()
    }
}

impl FunctionValue {
    /// Encodes this function value as a plain JSON object.
    ///
    /// The object has `params`, optional `return`, and `body` keys.
    /// No wrapping tag — a function is just an object with a `body`.
    pub fn to_json(&self) -> JsonValue {
        let params = JsonValue::Array(
            self.params
                .iter()
                .enumerate()
                .map(|(i, (name, schema))| {
                    let pattern = self.patterns.get(i).cloned().unwrap_or(JsonValue::Null);
                    serde_json::json!({
                        "name": name,
                        "schema": schema,
                        "pattern": pattern})
                })
                .collect(),
        );

        let mut obj = serde_json::Map::new();
        obj.insert("params".to_string(), params);
        if let Some(ret) = &self.return_schema {
            obj.insert("return".to_string(), ret.clone());
        }
        obj.insert("body".to_string(), JsonValue::String(self.body.clone()));
        JsonValue::Object(obj)
    }

    /// Decodes a function value from a JSON object.
    ///
    /// Returns `None` if the value is not an object or does not have
    /// a `body` string field. The `params` field is optional (defaults
    /// to empty); `return` is optional.
    ///
    /// `params` may be in either format:
    /// - **Array**: `[{"name": "x", "schema": {"type": "number"}}]`
    /// - **Map**: `{"x": {"type": "number"}}` (YAML convention)
    pub fn from_json(value: &JsonValue) -> Option<Self> {
        let obj = value.as_object()?;

        // A function value is identified by having a `body` string.
        let body = obj.get("body").and_then(|v| v.as_str())?.to_string();

        let params = obj
            .get("params")
            .map(|p| {
                if let Some(arr) = p.as_array() {
                    // Array format: [{"name": "x", "schema": {...}}, ...]
                    arr.iter()
                        .filter_map(|p| {
                            let name = p.get("name")?.as_str()?.to_string();
                            let schema = p.get("schema").cloned().unwrap_or(JsonValue::Null);
                            Some((name, schema))
                        })
                        .collect::<Vec<_>>()
                } else if let Some(map) = p.as_object() {
                    // Map format: {"x": {"type": "number"}, ...}
                    map.iter()
                        .map(|(name, schema)| (name.clone(), schema.clone()))
                        .collect::<Vec<_>>()
                } else {
                    Vec::new()
                }
            })
            .unwrap_or_default();

        let return_schema = obj.get("return").cloned();

        let patterns = obj
            .get("params")
            .and_then(|p| p.as_array())
            .map(|arr| {
                arr.iter()
                    .map(|p| p.get("pattern").cloned().unwrap_or(JsonValue::Null))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        Some(FunctionValue {
            params,
            return_schema,
            body,
            patterns,
        })
    }
}

/// A runtime value in the language.
///
/// A transparent newtype around `serde_json::Value`. Deliberately
/// boring — the kernel does not define its own type system.
///
/// `HostValue` is a type alias for `NonFunctionValue`, used at the
/// VM/host boundary.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct NonFunctionValue(JsonValue);

/// Alias used at the VM/host boundary.
pub type HostValue = NonFunctionValue;

impl NonFunctionValue {
    /// Creates a new runtime value from anything convertible to
    /// `serde_json::Value`.
    pub fn new(value: impl Into<JsonValue>) -> Self {
        NonFunctionValue(value.into())
    }

    /// Returns a reference to the underlying `serde_json::Value`.
    pub fn inner(&self) -> &JsonValue {
        &self.0
    }

    /// Returns a mutable reference to the underlying `serde_json::Value`.
    pub fn inner_mut(&mut self) -> &mut JsonValue {
        &mut self.0
    }

    /// Consumes self and returns the underlying `serde_json::Value`.
    pub fn into_inner(self) -> JsonValue {
        self.0
    }

    // ── predicates ──────────────────────────────────────────────

    pub fn is_null(&self) -> bool {
        self.0.is_null()
    }

    pub fn is_bool(&self) -> bool {
        self.0.is_boolean()
    }

    pub fn is_number(&self) -> bool {
        self.0.is_number()
    }

    pub fn is_string(&self) -> bool {
        self.0.is_string()
    }

    pub fn is_array(&self) -> bool {
        self.0.is_array()
    }

    pub fn is_object(&self) -> bool {
        self.0.is_object()
    }

    // ── accessors ───────────────────────────────────────────────

    pub fn as_bool(&self) -> Option<bool> {
        self.0.as_bool()
    }

    pub fn as_number(&self) -> Option<f64> {
        self.0.as_f64()
    }

    pub fn as_str(&self) -> Option<&str> {
        self.0.as_str()
    }

    pub fn as_array(&self) -> Option<&[JsonValue]> {
        self.0.as_array().map(|v| v.as_slice())
    }

    pub fn as_object(&self) -> Option<&serde_json::Map<String, JsonValue>> {
        self.0.as_object()
    }

    pub fn as_object_mut(&mut self) -> Option<&mut serde_json::Map<String, JsonValue>> {
        self.0.as_object_mut()
    }

    pub fn get(&self, key: &str) -> Option<&JsonValue> {
        self.0.get(key)
    }
}

impl FunctionRepr<NonFunctionValue> for FunctionValue {
    fn params(&self) -> Vec<(String, NonFunctionValue)> {
        self.params
            .iter()
            .map(|(n, v)| (n.clone(), NonFunctionValue::new(v.clone())))
            .collect()
    }

    fn return_schema(&self) -> Option<NonFunctionValue> {
        self.return_schema.clone().map(NonFunctionValue::new)
    }

    fn body(&self) -> &str {
        &self.body
    }
}

impl FunctionRepr<JsonValue> for FunctionValue {
    fn params(&self) -> Vec<(String, JsonValue)> {
        self.params.clone()
    }

    fn return_schema(&self) -> Option<JsonValue> {
        self.return_schema.clone()
    }

    fn body(&self) -> &str {
        &self.body
    }
}
