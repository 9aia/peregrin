use serde_json::Value as JsonValue;

/// A runtime value in the language.
///
/// A transparent newtype around `serde_json::Value`. Deliberately
/// boring — the kernel does not define its own type system.
///
/// `HostValue` is a type alias for `RuntimeValue`, used at the
/// VM/host boundary.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RuntimeValue(JsonValue);

/// Alias used at the VM/host boundary.
pub type HostValue = RuntimeValue;

impl RuntimeValue {
    /// Creates a new runtime value from anything convertible to
    /// `serde_json::Value`.
    pub fn new(value: impl Into<JsonValue>) -> Self {
        RuntimeValue(value.into())
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

    // ── object helpers ──────────────────────────────────────────

    pub fn get(&self, key: &str) -> Option<&JsonValue> {
        self.0.get(key)
    }
}
