use ac_runtime_values::NonFunctionValue;
use serde_json::Value as JsonValue;

/// Builds the default namespace from the document's data fields.
pub fn build_doc_namespace(data: &NonFunctionValue) -> JsonValue {
    data.inner().clone()
}

/// Builds an `env` namespace from process environment variables.
pub fn build_env_namespace() -> JsonValue {
    let mut map = serde_json::Map::new();
    for (key, value) in std::env::vars() {
        map.insert(key, JsonValue::String(value));
    }
    JsonValue::Object(map)
}
