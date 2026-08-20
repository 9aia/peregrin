use serde_json::{json, Value as JsonValue};

/// Compose a root JSON Schema from clean field schema fragments.
///
/// Each fragment is `(field_name, schema_json)`. The output is a JSON
/// Schema draft-07 document with:
/// - `$schema` set to draft-07
/// - `type: object`
/// - `properties` built from the fragments
/// - `additionalProperties: false`
pub fn compose_schema(fields: &[(String, JsonValue)]) -> JsonValue {
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn composes_simple_fields() {
        let fields = vec![
            ("host".to_string(), json!({ "type": "string" })),
            ("port".to_string(), json!({ "type": "number" })),
        ];
        let schema = compose_schema(&fields);

        assert_eq!(schema["$schema"], "http://json-schema.org/draft-07/schema#");
        assert_eq!(schema["type"], "object");
        assert_eq!(schema["additionalProperties"], false);
        assert_eq!(schema["properties"]["host"]["type"], "string");
        assert_eq!(schema["properties"]["port"]["type"], "number");
    }

    #[test]
    fn composes_empty_fields() {
        let fields: Vec<(String, JsonValue)> = vec![];
        let schema = compose_schema(&fields);

        assert_eq!(schema["type"], "object");
        assert_eq!(schema["properties"], json!({}));
        assert_eq!(schema["additionalProperties"], false);
    }

    #[test]
    fn preserves_complex_schema_vocabulary() {
        let fields = vec![
            ("config".to_string(), json!({
                "type": "object",
                "properties": {
                    "host": { "type": "string" },
                    "port": { "type": "number" }
                },
                "required": ["host"]
            })),
        ];
        let schema = compose_schema(&fields);

        assert_eq!(schema["properties"]["config"]["type"], "object");
        assert_eq!(schema["properties"]["config"]["required"], json!(["host"]));
        assert_eq!(
            schema["properties"]["config"]["properties"]["host"]["type"],
            "string"
        );
    }

    #[test]
    fn preserves_enum_and_pattern() {
        let fields = vec![
            ("level".to_string(), json!({
                "type": "string",
                "enum": ["debug", "info", "warn", "error"]
            })),
            ("name".to_string(), json!({
                "type": "string",
                "pattern": "^[a-z_]+$"
            })),
        ];
        let schema = compose_schema(&fields);

        assert_eq!(
            schema["properties"]["level"]["enum"],
            json!(["debug", "info", "warn", "error"])
        );
        assert_eq!(schema["properties"]["name"]["pattern"], "^[a-z_]+$");
    }

    #[test]
    fn merge_fields_excluded() {
        let fields = vec![
            ("host".to_string(), json!({ "type": "string" })),
            ("port".to_string(), json!({ "type": "number" })),
            // Note: merge fields are already filtered out by TypegenContext.
            // This test verifies compose doesn't add anything extra.
        ];
        let schema = compose_schema(&fields);

        let props = schema["properties"].as_object().unwrap();
        assert_eq!(props.len(), 2);
        assert!(!props.contains_key("_base"));
        assert!(!props.contains_key("import"));
    }
}
