use super::HttpError;

#[test]
fn network_display() {
    let err = HttpError::Network {
        message: "timeout".into(),
    };
    assert_eq!(err.to_string(), "network error: timeout");
}

#[test]
fn invalid_argument_display() {
    let err = HttpError::InvalidArgument {
        message: "bad url".into(),
    };
    assert_eq!(err.to_string(), "invalid argument: bad url");
}

#[test]
fn converts_to_vm_error() {
    let err = HttpError::Network {
        message: "timeout".into(),
    };
    let vm_err: ac_runtime::Error = err.into();
    match vm_err {
        ac_runtime::Error::PrimitiveError(msg) => assert!(msg.contains("network error")),
        other => panic!("expected PrimitiveError, got {other:?}"),
    }
}