//! # handl-lib-json
//!
//! JSON `Encoder`/`Decoder` for the `handl-lib-serialize` framework.
//!
//! Backed by `serde_json`.

use handl_lib_serialize::{Decoder, Encoder, SerializeError};
use serde_json::Value;

/// Combined JSON codec.
#[derive(Debug, Clone, Copy, Default)]
pub struct Json;

/// JSON encoder.
#[derive(Debug, Clone, Copy, Default)]
pub struct JsonEncoder;

/// JSON decoder.
#[derive(Debug, Clone, Copy, Default)]
pub struct JsonDecoder;

impl Encoder<Value> for Json {
    type Output = String;

    fn encode(&self, value: &Value) -> Result<Self::Output, SerializeError> {
        serde_json::to_string(value).map_err(|e| SerializeError::new(e.to_string()))
    }
}

impl Decoder<str, Value> for Json {
    fn decode(&self, input: &str) -> Result<Value, SerializeError> {
        serde_json::from_str(input).map_err(|e| SerializeError::new(e.to_string()))
    }
}

impl Encoder<Value> for JsonEncoder {
    type Output = String;

    fn encode(&self, value: &Value) -> Result<Self::Output, SerializeError> {
        Json.encode(value)
    }
}

impl Decoder<str, Value> for JsonDecoder {
    fn decode(&self, input: &str) -> Result<Value, SerializeError> {
        Json.decode(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_object() {
        let input = r#"{"hello":"world","list":[1,2,3]}"#;
        let value = Json.decode(input).unwrap();
        let output = Json.encode(&value).unwrap();
        let round = Json.decode(&output).unwrap();
        assert_eq!(value, round);
    }

    #[test]
    fn round_trip_primitives() {
        for input in ["\"hello\"", "42", "true", "null", "[1,2]"] {
            let value = Json.decode(input).unwrap();
            let output = Json.encode(&value).unwrap();
            let round = Json.decode(&output).unwrap();
            assert_eq!(value, round);
        }
    }

    #[test]
    fn rejects_invalid_json() {
        assert!(Json.decode("not json").is_err());
    }
}
