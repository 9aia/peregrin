use crate::AcDialect;
use ac_dialect::Dialect;

#[test]
fn schema_vocabulary_id() {
    let d = AcDialect;
    assert_eq!(d.schema_vocabulary().id(), "json-schema-draft-2020-12");
}
