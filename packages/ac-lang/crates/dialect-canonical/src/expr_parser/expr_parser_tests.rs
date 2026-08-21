use crate::AcDialect;
use ac_dialect::Dialect;

#[test]
fn find_call_open_basic() {
    let d = AcDialect;
    assert_eq!(d.parser().find_call_open("add(1, 2)"), Some(3));
    assert_eq!(d.parser().find_call_open("doc.name"), None);
}

#[test]
fn find_call_open_in_string() {
    let d = AcDialect;
    assert_eq!(d.parser().find_call_open("\"hello(world\""), None);
}

#[test]
fn split_args_basic() {
    let d = AcDialect;
    let args = d.parser().split_args("1, 2, 3");
    assert_eq!(args, vec!["1", " 2", " 3"]);
}

#[test]
fn split_args_nested() {
    let d = AcDialect;
    let args = d.parser().split_args("add(1, 2), 3");
    assert_eq!(args.len(), 2);
}

#[test]
fn parse_string_literal_double() {
    let d = AcDialect;
    assert_eq!(
        d.parser().parse_string_literal("\"hello\""),
        Some("hello".to_string())
    );
}

#[test]
fn parse_string_literal_single() {
    let d = AcDialect;
    assert_eq!(
        d.parser().parse_string_literal("'hello'"),
        Some("hello".to_string())
    );
}

#[test]
fn parse_string_literal_backslash_is_literal() {
    let d = AcDialect;
    assert_eq!(
        d.parser().parse_string_literal("\"hello\\nworld\""),
        Some("hello\\nworld".to_string())
    );
    assert_eq!(
        d.parser().parse_string_literal("\"\\t\\n\\r\""),
        Some("\\t\\n\\r".to_string())
    );
}

#[test]
fn parse_string_literal_embedded_opposite_quote() {
    let d = AcDialect;
    assert_eq!(
        d.parser().parse_string_literal("\"it's\""),
        Some("it's".to_string())
    );
    assert_eq!(
        d.parser().parse_string_literal("'say \"hi\"'"),
        Some("say \"hi\"".to_string())
    );
}

#[test]
fn parse_number_literal() {
    let d = AcDialect;
    assert_eq!(d.parser().parse_number_literal("42"), Some(42.0));
    assert_eq!(d.parser().parse_number_literal("1.5"), Some(1.5));
    assert_eq!(d.parser().parse_number_literal("-3"), Some(-3.0));
    assert_eq!(d.parser().parse_number_literal("not a number"), None);
}

#[test]
fn parse_bool_literal() {
    let d = AcDialect;
    assert_eq!(d.parser().parse_bool_literal("true"), Some(true));
    assert_eq!(d.parser().parse_bool_literal("false"), Some(false));
    assert_eq!(d.parser().parse_bool_literal("yes"), None);
    assert_eq!(d.parser().parse_bool_literal("null"), None);
}

#[test]
fn is_null_literal() {
    let d = AcDialect;
    assert!(d.parser().is_null_literal("null"));
    assert!(!d.parser().is_null_literal("none"));
    assert!(!d.parser().is_null_literal("true"));
}
