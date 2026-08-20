use super::*;

#[test]
fn error_display_unknown_function() {
    let err = Error::UnknownFunction("foo".to_string());
    assert_eq!(err.to_string(), "unknown function: foo");
}

#[test]
fn error_display_unknown_variable() {
    let err = Error::UnknownVariable("bar".to_string());
    assert_eq!(err.to_string(), "unknown variable: bar");
}

#[test]
fn error_display_primitive_error() {
    let err = Error::PrimitiveError("boom".to_string());
    assert_eq!(err.to_string(), "primitive error: boom");
}

#[test]
fn error_display_type_mismatch() {
    let err = Error::TypeMismatch { expected: "string", got: "number" };
    assert_eq!(err.to_string(), "type mismatch: expected string, got number");
}

#[test]
fn error_display_argument_count() {
    let err = Error::ArgumentCount { expected: 2, got: 1 };
    assert_eq!(err.to_string(), "argument count mismatch: expected 2, got 1");
}

#[test]
fn error_clone_eq() {
    let err = Error::UnknownFunction("test".to_string());
    let cloned = err.clone();
    assert_eq!(err, cloned);
}
