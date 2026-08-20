use super::*;

#[test]
fn validate_mutable_no_violation() {
    let data = serde_json::json!({
        "env_name": "prod",
        "config": {
            "env_name": "prod",
            "nested": {
                "env_name": "prod"
            }
        }
    });
    let errors = validate_mutable(&data, &["env_name".to_string()]);
    assert!(errors.is_empty());
}

#[test]
fn validate_mutable_violation() {
    let data = serde_json::json!({
        "env_name": "prod",
        "config": {
            "env_name": "dev",
            "nested": {
                "env_name": "prod"
            }
        }
    });
    let errors = validate_mutable(&data, &["env_name".to_string()]);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].contains("config.env_name"));
    assert!(errors[0].contains("immutable"));
}

#[test]
fn validate_mutable_empty_is_noop() {
    let data = serde_json::json!({"name": "ac"});
    let errors = validate_mutable(&data, &[]);
    assert!(errors.is_empty());
}
