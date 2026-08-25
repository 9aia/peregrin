use crate::{Error, NonFunctionValue};

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
    let err = Error::TypeMismatch {
        expected: "string",
        got: "number",
    };
    assert_eq!(
        err.to_string(),
        "type mismatch: expected string, got number"
    );
}

#[test]
fn error_display_argument_count() {
    let err = Error::ArgumentCount {
        expected: 2,
        got: 1,
    };
    assert_eq!(
        err.to_string(),
        "argument count mismatch: expected 2, got 1"
    );
}

#[test]
fn error_clone_eq() {
    let err = Error::UnknownFunction("test".to_string());
    let cloned = err.clone();
    assert_eq!(err, cloned);
}

#[test]
fn error_display_yield() {
    let err = Error::Yield(NonFunctionValue::new(42));
    assert!(err.to_string().contains("yielded"));
}

#[test]
fn error_yield_clone_eq() {
    let err = Error::Yield(NonFunctionValue::new("hello"));
    let cloned = err.clone();
    assert_eq!(err, cloned);
}

#[test]
fn error_display_operation() {
    let err = Error::Operation {
        name: "fetch".to_string(),
        args: vec![NonFunctionValue::new("url")],
    };
    let s = err.to_string();
    assert!(s.contains("fetch"));
    assert!(s.contains("operation"));
}

#[test]
fn error_operation_clone_eq() {
    let err = Error::Operation {
        name: "fetch".to_string(),
        args: vec![NonFunctionValue::new(42)],
    };
    let cloned = err.clone();
    assert_eq!(err, cloned);
}

#[test]
fn error_operation_with_empty_args_display() {
    let err = Error::Operation {
        name: "ping".to_string(),
        args: vec![],
    };
    let s = err.to_string();
    assert!(s.contains("ping"));
    assert!(s.contains("0 args"));
}

#[test]
fn error_display_resource_limit() {
    let err = Error::ResourceLimit("depth limit exceeded: 100".to_string());
    let s = err.to_string();
    assert!(s.contains("resource limit exceeded"));
    assert!(s.contains("depth limit exceeded: 100"));
}

#[test]
fn error_all_variants_are_cloneable() {
    let variants: Vec<Error> = vec![
        Error::UnknownFunction("f".to_string()),
        Error::UnknownVariable("v".to_string()),
        Error::PrimitiveError("boom".to_string()),
        Error::TypeMismatch {
            expected: "int",
            got: "str",
        },
        Error::ArgumentCount {
            expected: 1,
            got: 2,
        },
        Error::Yield(NonFunctionValue::new(0)),
        Error::Operation {
            name: "op".to_string(),
            args: vec![],
        },
        Error::ResourceLimit("depth limit exceeded: 100".to_string()),
    ];
    for v in &variants {
        let _ = v.clone();
    }
}
