#[test]
fn yaml_to_json_basic() {
    let yaml: serde_yaml::Value = serde_yaml::from_str("name: ac\nversion: 1\n").unwrap();
    let json = super::yaml_to_json(yaml).unwrap();
    assert_eq!(json["name"], "ac");
    assert_eq!(json["version"], 1);
}

#[test]
fn yaml_to_json_nested() {
    let yaml: serde_yaml::Value =
        serde_yaml::from_str("author:\n  name: alice\n  email: alice@example.com\n").unwrap();
    let json = super::yaml_to_json(yaml).unwrap();
    assert_eq!(json["author"]["name"], "alice");
    assert_eq!(json["author"]["email"], "alice@example.com");
}

#[test]
fn yaml_to_json_array() {
    let yaml: serde_yaml::Value = serde_yaml::from_str("items:\n  - a\n  - b\n  - c\n").unwrap();
    let json = super::yaml_to_json(yaml).unwrap();
    assert_eq!(json["items"][0], "a");
    assert_eq!(json["items"][2], "c");
}

#[test]
fn json_to_yaml_roundtrip() {
    let json = serde_json::json!({"name": "ac", "version": 1});
    let yaml = super::json_to_yaml(&json);
    let yaml_str = serde_yaml::to_string(&yaml).unwrap();
    assert!(yaml_str.contains("name: ac"));
    assert!(yaml_str.contains("version: 1"));
}
