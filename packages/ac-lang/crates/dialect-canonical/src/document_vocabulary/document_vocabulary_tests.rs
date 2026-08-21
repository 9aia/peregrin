use crate::AcDialect;
use ac_dialect::Dialect;

#[test]
fn key_names() {
    let d = AcDialect;
    let v = d.document_vocabulary();
    assert_eq!(v.fields_key(), "fields");
    assert_eq!(v.functions_key(), "functions");
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
fn top_level_reserved_keys() {
    let d = AcDialect;
    let reserved = d.document_vocabulary().top_level_reserved_keys();
    assert!(reserved.contains(&"fields"));
    assert!(reserved.contains(&"functions"));
}

#[test]
fn reserved_field_keys() {
    let d = AcDialect;
    let keys = d.document_vocabulary().reserved_field_keys();
    assert!(keys.contains(&"expr"));
    assert!(keys.contains(&"arg"));
    assert!(keys.contains(&"merge"));
    assert!(keys.contains(&"inheritable"));
    assert!(keys.contains(&"mutable"));
}
