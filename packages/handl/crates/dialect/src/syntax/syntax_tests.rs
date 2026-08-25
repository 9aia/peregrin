use crate::dialect::Dialect;
use crate::dialect::dialect_stub::StubDialect;

#[test]
fn syntax_tokens() {
    let d = StubDialect;
    let s = d.syntax();
    assert_eq!(s.interpolation_prefix(), "${");
    assert_eq!(s.interpolation_suffix(), "}");
    assert_eq!(s.path_separator(), '.');
    assert_eq!(s.call_open(), '(');
    assert_eq!(s.call_close(), ')');
}
