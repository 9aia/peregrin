use serde_json::Value as JsonValue;

pub fn yaml_to_json(v: serde_yaml::Value) -> Result<JsonValue, crate::error::YamlError> {
    match v {
        serde_yaml::Value::Null => Ok(JsonValue::Null),
        serde_yaml::Value::Bool(b) => Ok(JsonValue::Bool(b)),
        serde_yaml::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(JsonValue::Number(serde_json::Number::from(i)))
            } else if let Some(u) = n.as_u64() {
                Ok(JsonValue::Number(serde_json::Number::from(u)))
            } else if let Some(f) = n.as_f64() {
                serde_json::Number::from_f64(f)
                    .map(JsonValue::Number)
                    .ok_or_else(|| crate::error::YamlError::Convert(format!("cannot represent number: {f}")))
            } else {
                Ok(JsonValue::Null)
            }
        }
        serde_yaml::Value::String(s) => Ok(JsonValue::String(s)),
        serde_yaml::Value::Sequence(seq) => {
            let arr = seq
                .into_iter()
                .map(yaml_to_json)
                .collect::<Result<Vec<_>, _>>()?;
            Ok(JsonValue::Array(arr))
        }
        serde_yaml::Value::Mapping(map) => {
            let mut obj = serde_json::Map::new();
            for (k, v) in map {
                let key = match k {
                    serde_yaml::Value::String(s) => s,
                    other => serde_yaml::to_string(&other)
                        .map_err(crate::error::YamlError::from)?
                        .trim()
                        .to_string(),
                };
                obj.insert(key, yaml_to_json(v)?);
            }
            Ok(JsonValue::Object(obj))
        }
        serde_yaml::Value::Tagged(tagged) => yaml_to_json(tagged.value),
    }
}

pub fn json_to_yaml(value: &JsonValue) -> serde_yaml::Value {
    match value {
        JsonValue::Null => serde_yaml::Value::Null,
        JsonValue::Bool(b) => serde_yaml::Value::Bool(*b),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                serde_yaml::Value::Number(serde_yaml::Number::from(i))
            } else if let Some(u) = n.as_u64() {
                serde_yaml::Value::Number(serde_yaml::Number::from(u))
            } else if let Some(f) = n.as_f64() {
                serde_yaml::Value::Number(serde_yaml::Number::from(f))
            } else {
                serde_yaml::Value::Null
            }
        }
        JsonValue::String(s) => serde_yaml::Value::String(s.clone()),
        JsonValue::Array(arr) => {
            serde_yaml::Value::Sequence(arr.iter().map(json_to_yaml).collect())
        }
        JsonValue::Object(obj) => {
            let mut map = serde_yaml::Mapping::new();
            for (k, v) in obj {
                map.insert(serde_yaml::Value::String(k.clone()), json_to_yaml(v));
            }
            serde_yaml::Value::Mapping(map)
        }
    }
}
