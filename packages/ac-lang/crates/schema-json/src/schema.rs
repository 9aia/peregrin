use ac_schema::{Schema, ValidationError};
use ac_runtime::RuntimeValue;
use serde_json::Value as JsonValue;

/// A JSON Schema document that can validate `RuntimeValue`s.
///
/// Stores the raw schema as a `serde_json::Value` and implements
/// [`Schema<RuntimeValue>`] with a built-in minimal validator.
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
    pub fn type_name(&self) -> Option<&str> {
        self.raw.get("type").and_then(|v| v.as_str())
    }
}

impl Schema<RuntimeValue> for JsonSchema {
    fn validate(&self, value: &RuntimeValue) -> Result<(), ValidationError> {
        if let Some(type_name) = self.type_name() {
            crate::validator::check_type(value.inner(), type_name, "")?;
        }

        if let Some(required) = self.raw.get("required").and_then(|v| v.as_array())
            && let Some(entries) = value.inner().as_object()
        {
            for req in required {
                if let Some(field) = req.as_str()
                    && !entries.contains_key(field)
                {
                    return Err(ValidationError {
                        message: format!("missing required field: {field}"),
                        path: String::new(),
                    });
                }
            }
        }

        Ok(())
    }
}
