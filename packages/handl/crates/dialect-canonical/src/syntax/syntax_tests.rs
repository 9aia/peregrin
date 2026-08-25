use crate::HandlDialect;
use handl_dialect::Dialect;

#[test]
fn interpolation_markers() {
    let d = HandlDialect;
    assert_eq!(d.syntax().interpolation_prefix(), "${");
    assert_eq!(d.syntax().interpolation_suffix(), "}");
}

#[test]
fn path_separator() {
    let d = HandlDialect;
    assert_eq!(d.syntax().path_separator(), '.');
}

#[test]
fn call_markers() {
    let d = HandlDialect;
    assert_eq!(d.syntax().call_open(), '(');
    assert_eq!(d.syntax().call_close(), ')');
}
