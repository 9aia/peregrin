use super::*;
use serde_json::json;

#[test]
fn no_immutable_fields_no_errors() {
    let data = json!({ "name": "alice" });
    let errors = check(&[], &data);
    assert!(errors.is_empty());
}

#[test]
fn no_override_no_errors() {
    let immutable = vec!["env".to_string()];
    let data = json!({
        "env": "prod",
        "config": {
            "name": "alice"
        }
    });
    let errors = check(&immutable, &data);
    assert!(errors.is_empty());
}

#[test]
fn override_reports_error() {
    let immutable = vec!["env".to_string()];
    let data = json!({
        "env": "prod",
        "config": {
            "env": "dev"
        }
    });
    let errors = check(&immutable, &data);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].field, "env");
    assert!(errors[0].path.contains("config"));
}

#[test]
fn deep_override_reports_error() {
    let immutable = vec!["env".to_string()];
    let data = json!({
        "env": "prod",
        "server": {
            "deep": {
                "env": "staging"
            }
        }
    });
    let errors = check(&immutable, &data);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].path.contains("server"));
    assert!(errors[0].path.contains("deep"));
}

#[test]
fn same_value_override_is_ok() {
    let immutable = vec!["env".to_string()];
    let data = json!({
        "env": "prod",
        "config": {
            "env": "prod"
        }
    });
    let errors = check(&immutable, &data);
    assert!(errors.is_empty());
}

#[test]
fn is_immutable_detects_false() {
    assert!(is_immutable(&json!({ "type": "string", "mutable": false })));
    assert!(!is_immutable(&json!({ "type": "string", "mutable": true })));
    assert!(!is_immutable(&json!({ "type": "string" })));
}