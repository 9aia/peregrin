use super::*;

#[test]
fn parse_json_basic() {
    let value = parse(r#"{"name": "handl", "version": 1}"#).unwrap();
    assert_eq!(value.get("name").unwrap().as_str(), Some("handl"));
    assert_eq!(value.get("version").unwrap().as_i64(), Some(1));
}

#[test]
fn parse_json_invalid() {
    assert!(parse("{invalid}").is_err());
}

#[test]
fn json_string_roundtrip() {
    let value = parse(r#"{"name": "handl"}"#).unwrap();
    let s = to_string(&value).unwrap();
    let value2 = parse(&s).unwrap();
    assert_eq!(value2.get("name").unwrap().as_str(), Some("handl"));
}

#[test]
fn json_string_pretty() {
    let value = parse(r#"{"name": "handl"}"#).unwrap();
    let s = to_string_pretty(&value).unwrap();
    assert!(s.contains("\n"));
}

#[test]
fn from_json_value() {
    let json = serde_json::json!({"key": "value"});
    let rv = from_json(json);
    assert_eq!(rv.get("key").unwrap().as_str(), Some("value"));
}

#[test]
fn to_json_value() {
    let rv = parse(r#"{"key": "value"}"#).unwrap();
    let json = to_json(&rv);
    assert_eq!(json["key"], "value");
}

#[test]
fn frontend_trait_impl() {
    let frontend = JsonFrontend;
    let value = frontend.parse(r#"{"name": "handl"}"#).unwrap();
    assert_eq!(value.get("name").unwrap().as_str(), Some("handl"));
    let serialized = frontend.serialize(&value).unwrap();
    assert!(serialized.contains("handl"));
}
