use indexmap::IndexMap;
use serde_json::{json, Value as JsonValue};

/// Compose a root JSON Schema from clean field schema fragments.
///
/// Each fragment is `(field_name, schema_json)`. The output is a JSON
/// Schema draft-07 document with:
/// - `$schema` set to draft-07
/// - `type: object`
/// - `properties` built from the fragments
/// - `additionalProperties: false`
pub fn compose_schema(fields: &IndexMap<String, JsonValue>) -> JsonValue {
    let mut properties = serde_json::Map::new();
    for (name, schema) in fields {
        properties.insert(name.clone(), schema.clone());
    }

    json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "type": "object",
        "properties": JsonValue::Object(properties),
        "additionalProperties": false
    })
}
