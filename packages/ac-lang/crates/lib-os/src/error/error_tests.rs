use super::OsError;

#[test]
fn not_found_display() {
    let err = OsError::NotFound {
        path: "/tmp/missing".into(),
    };
    assert_eq!(err.to_string(), "not found: /tmp/missing");
}

#[test]
fn permission_denied_display() {
    let err = OsError::PermissionDenied {
        path: "/secret".into(),
    };
    assert_eq!(err.to_string(), "permission denied: /secret");
}

#[test]
fn already_exists_display() {
    let err = OsError::AlreadyExists {
        path: "/tmp/file".into(),
    };
    assert_eq!(err.to_string(), "already exists: /tmp/file");
}

#[test]
fn invalid_argument_display() {
    let err = OsError::InvalidArgument {
        message: "bad path".into(),
    };
    assert_eq!(err.to_string(), "invalid argument: bad path");
}

#[test]
fn io_display() {
    let err = OsError::Io {
        message: "disk full".into(),
    };
    assert_eq!(err.to_string(), "io error: disk full");
}

#[test]
fn converts_to_vm_error() {
    let err = OsError::NotFound { path: "/x".into() };
    let vm_err: ac_runtime::Error = err.into();
    match vm_err {
        ac_runtime::Error::PrimitiveError(msg) => assert!(msg.contains("not found")),
        other => panic!("expected PrimitiveError, got {other:?}"),
    }
}