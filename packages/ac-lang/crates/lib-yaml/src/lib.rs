//! # ac-lib-yaml
//!
//! YAML `Encoder`/`Decoder` for the `ac-lib-serialize` framework.
//!
//! Backed by `serde_yaml`.

use ac_lib_serialize::{Decoder, Encoder, SerializeError};
use serde_yaml::Value;

/// Combined YAML codec.
#[derive(Debug, Clone, Copy, Default)]
pub struct Yaml;

/// YAML encoder.
#[derive(Debug, Clone, Copy, Default)]
pub struct YamlEncoder;

/// YAML decoder.
#[derive(Debug, Clone, Copy, Default)]
pub struct YamlDecoder;

impl Encoder<Value> for Yaml {
    type Output = String;

    fn encode(&self, value: &Value) -> Result<Self::Output, SerializeError> {
        serde_yaml::to_string(value).map_err(|e| SerializeError::new(e.to_string()))
    }
}

impl Decoder<str, Value> for Yaml {
    fn decode(&self, input: &str) -> Result<Value, SerializeError> {
        serde_yaml::from_str(input).map_err(|e| SerializeError::new(e.to_string()))
    }
}

impl Encoder<Value> for YamlEncoder {
    type Output = String;

    fn encode(&self, value: &Value) -> Result<Self::Output, SerializeError> {
        Yaml.encode(value)
    }
}

impl Decoder<str, Value> for YamlDecoder {
    fn decode(&self, input: &str) -> Result<Value, SerializeError> {
        Yaml.decode(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_document() {
        let input = r#"
hello: world
list:
  - 1
  - 2
  - 3
nested:
  answer: 42
"#;
        let value = Yaml.decode(input).unwrap();
        let output = Yaml.encode(&value).unwrap();
        let round = Yaml.decode(&output).unwrap();
        assert_eq!(value, round);
    }

    #[test]
    fn rejects_invalid_yaml() {
        assert!(Yaml.decode("  [").is_err());
    }
}
