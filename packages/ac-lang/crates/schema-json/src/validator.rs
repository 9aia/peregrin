use ac_schema::ValidationError;
use serde_json::Value as JsonValue;

pub fn check_type(value: &JsonValue, expected: &str, path: &str) -> Result<(), ValidationError> {
    let actual = match value {
        JsonValue::Null => "null",
        JsonValue::Bool(_) => "boolean",
        JsonValue::Number(_) => "number",
        JsonValue::String(_) => "string",
        JsonValue::Array(_) => "array",
        JsonValue::Object(_) => "object",
    };

    if expected == "integer"
        && let JsonValue::Number(n) = value
        && n.as_f64().is_some_and(|f| f.fract() == 0.0)
    {
        return Ok(());
    }

    if actual != expected {
        return Err(ValidationError {
            message: format!("expected type {expected}, got {actual}"),
            path: path.to_string(),
        });
    }

    Ok(())
}
