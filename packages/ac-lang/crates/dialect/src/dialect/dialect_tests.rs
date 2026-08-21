use super::dialect_stub::StubDialect;

use super::Dialect;

#[test]
fn schema_vocabulary_id() {
    let d = StubDialect;
    assert_eq!(d.schema_vocabulary().id(), "stub");
}
