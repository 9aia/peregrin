use ac_schema::{Schema, ValidationError};
use ac_runtime::RuntimeValue;
use serde_json::Value as JsonValue;

/// A JSON Schema document that can validate `RuntimeValue`s.
///
/// Stores the raw schema as a `serde_json::Value` and implements
/// [`Schema<RuntimeValue>`] using the [`jsonschema`] crate for full
/// JSON Schema draft 2020-12 validation.
#[derive(Debug, Clone)]
pub struct JsonSchema {
    /// The raw schema document.
    pub raw: JsonValue,
}

impl JsonSchema {
    /// Creates a JSON schema from a `serde_json::Value`.
    pub fn new(raw: JsonValue) -> Self {
        JsonSchema { raw }
    }

    /// Creates a JSON schema by parsing a JSON string.
    pub fn parse(s: &str) -> Result<Self, serde_json::Error> {
        Ok(JsonSchema {
            raw: serde_json::from_str(s)?,
        })
    }

    /// Returns the schema's declared type, if any.
    ///
    /// For single-type schemas (e.g. `"type": "string"`) returns the
    /// type name. For multi-type schemas (e.g. `"type": ["string",
    /// "null"]`) returns the first type. Returns `None` if no `type`
    /// keyword is present.
    pub fn type_name(&self) -> Option<&str> {
        match self.raw.get("type") {
            Some(JsonValue::String(s)) => Some(s.as_str()),
            Some(JsonValue::Array(arr)) => arr.first().and_then(|v| v.as_str()),
            _ => None,
        }
    }
}

impl Schema<RuntimeValue> for JsonSchema {
    fn validate(&self, value: &RuntimeValue) -> Result<(), ValidationError> {
        let validator = jsonschema::draft202012::new(&self.raw)
            .map_err(|e| ValidationError {
                message: format!("invalid JSON schema: {e}"),
                path: String::new(),
            })?;

        validator
            .validate(value.inner())
            .map_err(|e| ValidationError {
                message: e.to_string(),
                path: e.instance_path().to_string(),
            })
    }
}
