use crate::HandlDialect;
use handl_dialect::Dialect;

#[test]
fn schema_vocabulary_id() {
    let d = HandlDialect;
    assert_eq!(d.schema_vocabulary().id(), "json-schema-draft-2020-12");
}
