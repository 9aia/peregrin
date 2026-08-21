use crate::dialect::Dialect;
use crate::dialect::dialect_stub::StubDialect;

#[test]
fn find_call_open_simple() {
    let d = StubDialect;
    assert_eq!(d.parser().find_call_open("add(1, 2)"), Some(3));
}

#[test]
fn find_call_open_none() {
    let d = StubDialect;
    assert_eq!(d.parser().find_call_open("doc.name"), None);
}

#[test]
fn find_call_open_inside_string() {
    let d = StubDialect;
    assert_eq!(
        d.parser().find_call_open(r#"concat("not (here)", x)"#),
        Some(6)
    );
}

#[test]
fn find_call_open_single_quoted_string() {
    let d = StubDialect;
    assert_eq!(
        d.parser().find_call_open("concat('no (paren)', x)"),
        Some(6)
    );
}

#[test]
fn split_args_simple() {
    let d = StubDialect;
    assert_eq!(d.parser().split_args("1, 2, 3"), vec!["1", "2", "3"]);
}

#[test]
fn split_args_nested_call() {
    let d = StubDialect;
    assert_eq!(
        d.parser().split_args("add(1, 2), 3"),
        vec!["add(1, 2)", "3"]
    );
}

#[test]
fn split_args_string_with_comma() {
    let d = StubDialect;
    assert_eq!(d.parser().split_args("\"a, b\", 3"), vec!["\"a, b\"", "3"]);
}

#[test]
fn split_args_single_quoted_with_comma() {
    let d = StubDialect;
    assert_eq!(d.parser().split_args("'a, b', 3"), vec!["'a, b'", "3"]);
}

#[test]
fn parse_string_literal_double_quoted() {
    let d = StubDialect;
    assert_eq!(
        d.parser().parse_string_literal("\"hello\""),
        Some("hello".to_string())
    );
}

#[test]
fn parse_string_literal_single_quoted() {
    let d = StubDialect;
    assert_eq!(
        d.parser().parse_string_literal("'hello'"),
        Some("hello".to_string())
    );
}

#[test]
fn parse_string_literal_not_a_string() {
    let d = StubDialect;
    assert_eq!(d.parser().parse_string_literal("hello"), None);
}

#[test]
fn parse_number_literal_integer() {
    let d = StubDialect;
    assert_eq!(d.parser().parse_number_literal("42"), Some(42.0));
}

#[test]
fn parse_number_literal_float() {
    let d = StubDialect;
    assert_eq!(d.parser().parse_number_literal("1.5"), Some(1.5));
}

#[test]
fn parse_number_literal_negative() {
    let d = StubDialect;
    assert_eq!(d.parser().parse_number_literal("-3"), Some(-3.0));
}

#[test]
fn parse_bool_literal_true() {
    let d = StubDialect;
    assert_eq!(d.parser().parse_bool_literal("true"), Some(true));
}

#[test]
fn parse_bool_literal_false() {
    let d = StubDialect;
    assert_eq!(d.parser().parse_bool_literal("false"), Some(false));
}

#[test]
fn parse_bool_literal_invalid() {
    let d = StubDialect;
    assert_eq!(d.parser().parse_bool_literal("yes"), None);
}

#[test]
fn is_null_literal_true() {
    let d = StubDialect;
    assert!(d.parser().is_null_literal("null"));
}

#[test]
fn is_null_literal_false() {
    let d = StubDialect;
    assert!(!d.parser().is_null_literal("none"));
}
