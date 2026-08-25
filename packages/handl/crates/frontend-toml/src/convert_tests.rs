#[test]
fn toml_to_json_basic() {
    let toml_val: toml::Value = toml::from_str("name = \"handl\"\nversion = 1\n").unwrap();
    let json = super::toml_to_json(toml_val);
    assert_eq!(json["name"], "handl");
    assert_eq!(json["version"], 1);
}

#[test]
fn toml_to_json_nested() {
    let toml_val: toml::Value = toml::from_str("[author]\nname = \"alice\"\n").unwrap();
    let json = super::toml_to_json(toml_val);
    assert_eq!(json["author"]["name"], "alice");
}

#[test]
fn toml_to_json_array() {
    let toml_val: toml::Value = toml::from_str("items = [\"a\", \"b\", \"c\"]\n").unwrap();
    let json = super::toml_to_json(toml_val);
    assert_eq!(json["items"][0], "a");
    assert_eq!(json["items"][2], "c");
}

#[test]
fn json_to_toml_roundtrip() {
    let json = serde_json::json!({"name": "handl", "version": 1});
    let toml_val = super::json_to_toml(&json);
    let toml_str = toml::to_string(&toml_val).unwrap();
    assert!(toml_str.contains("name = \"handl\""));
    assert!(toml_str.contains("version = 1"));
}
