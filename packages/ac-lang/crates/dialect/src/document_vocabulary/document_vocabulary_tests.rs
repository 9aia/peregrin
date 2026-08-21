use crate::dialect::Dialect;
use crate::dialect::dialect_stub::StubDialect;

#[test]
fn default_key_names() {
    let d = StubDialect;
    let v = d.document_vocabulary();
    assert_eq!(v.fields_key(), "$");
    assert_eq!(v.expr_key(), "expr");
    assert_eq!(v.arg_key(), "arg");
    assert_eq!(v.merge_key(), "merge");
    assert_eq!(v.inheritable_key(), "inheritable");
    assert_eq!(v.mutable_key(), "mutable");
    assert_eq!(v.params_key(), "params");
    assert_eq!(v.return_key(), "return");
    assert_eq!(v.body_key(), "body");
}

#[test]
fn reserved_field_keys_default() {
    let d = StubDialect;
    let keys = d.document_vocabulary().reserved_field_keys();
    assert_eq!(keys, vec!["expr", "arg", "merge", "inheritable", "mutable"]);
}

#[test]
fn top_level_reserved_keys_default() {
    let d = StubDialect;
    let keys = d.document_vocabulary().top_level_reserved_keys();
    assert_eq!(keys, vec!["$"]);
}
