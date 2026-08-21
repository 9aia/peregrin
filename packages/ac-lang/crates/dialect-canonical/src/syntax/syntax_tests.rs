use crate::AcDialect;
use ac_dialect::Dialect;

#[test]
fn interpolation_markers() {
    let d = AcDialect;
    assert_eq!(d.syntax().interpolation_prefix(), "${");
    assert_eq!(d.syntax().interpolation_suffix(), "}");
}

#[test]
fn path_separator() {
    let d = AcDialect;
    assert_eq!(d.syntax().path_separator(), '.');
}

#[test]
fn call_markers() {
    let d = AcDialect;
    assert_eq!(d.syntax().call_open(), '(');
    assert_eq!(d.syntax().call_close(), ')');
}
