use super::validation_error::ValidationError;

#[test]
fn validation_error_display_no_path() {
    let err = ValidationError {
        message: "missing field".to_string(),
        path: String::new(),
        location: None,
    };
    assert_eq!(err.to_string(), "missing field");
}

#[test]
fn validation_error_display_with_path() {
    let err = ValidationError {
        message: "expected string".to_string(),
        path: "user.name".to_string(),
        location: None,
    };
    assert_eq!(err.to_string(), "user.name: expected string");
}

#[test]
fn validation_error_clone_eq() {
    let err = ValidationError {
        message: "test".to_string(),
        path: "a.b".to_string(),
        location: None,
    };
    let cloned = err.clone();
    assert_eq!(err, cloned);
}
