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
fn split_args_array_literal() {
    let d = AcDialect;
    let args = d
        .parser()
        .split_args("is_compatible, [obj.get(actual, \"item\"), obj.get(expected, \"item\")]");
    assert_eq!(args.len(), 2);
    assert_eq!(args[0], "is_compatible");
    assert_eq!(
        args[1].trim(),
        "[obj.get(actual, \"item\"), obj.get(expected, \"item\")]"
    );
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

// ── handle detection ─────────────────────────────────────────

#[test]
fn find_handle_open_basic() {
    let d = AcDialect;
    assert_eq!(
        d.parser()
            .find_handle_open("handle(fetch(\"data\")) { \"fetch\"(url) as k: k(42) }"),
        Some(6)
    );
}

#[test]
fn find_handle_open_none_for_function_call() {
    let d = AcDialect;
    assert_eq!(d.parser().find_handle_open("add(1, 2)"), None);
}

#[test]
fn find_handle_open_none_for_path() {
    let d = AcDialect;
    assert_eq!(d.parser().find_handle_open("doc.name"), None);
}

#[test]
fn find_handle_open_none_for_literal() {
    let d = AcDialect;
    assert_eq!(d.parser().find_handle_open("\"hello\""), None);
}

#[test]
fn find_handle_open_not_in_string() {
    let d = AcDialect;
    assert_eq!(d.parser().find_handle_open("\"handle(world\""), None);
}

#[test]
fn find_handle_open_not_partial_word() {
    let d = AcDialect;
    // "handleIt(" should not match — "handle" must be a standalone keyword
    assert_eq!(d.parser().find_handle_open("handleIt(1)"), None);
}

#[test]
fn split_handler_clauses_single() {
    let d = AcDialect;
    let clauses = d
        .parser()
        .split_handler_clauses("\"fetch\"(url) as k: k(42)");
    assert_eq!(clauses.len(), 1);
    assert_eq!(clauses[0], "\"fetch\"(url) as k: k(42)");
}

#[test]
fn split_handler_clauses_multiple() {
    let d = AcDialect;
    let clauses = d
        .parser()
        .split_handler_clauses("\"fetch\"(url) as k: k(42), \"log\"(msg) as k: k(1)");
    assert_eq!(clauses.len(), 2);
    assert_eq!(clauses[0], "\"fetch\"(url) as k: k(42)");
    assert_eq!(clauses[1], " \"log\"(msg) as k: k(1)");
}

#[test]
fn split_handler_clauses_empty() {
    let d = AcDialect;
    let clauses = d.parser().split_handler_clauses("");
    assert_eq!(clauses.len(), 0);
}

#[test]
fn parse_handler_clause_basic() {
    let d = AcDialect;
    let parsed = d
        .parser()
        .parse_handler_clause("\"fetch\"(url) as k: k(42)");
    let parsed = parsed.expect("should parse");
    assert_eq!(parsed.op_name, "fetch");
    assert_eq!(parsed.params, vec!["url"]);
    assert_eq!(parsed.continuation_name, "k");
    assert_eq!(parsed.body, "k(42)");
}

#[test]
fn parse_handler_clause_no_params() {
    let d = AcDialect;
    let parsed = d
        .parser()
        .parse_handler_clause("\"fetch\"() as k: \"cached\"");
    let parsed = parsed.expect("should parse");
    assert_eq!(parsed.op_name, "fetch");
    assert!(parsed.params.is_empty());
    assert_eq!(parsed.continuation_name, "k");
    assert_eq!(parsed.body, "\"cached\"");
}

#[test]
fn parse_handler_clause_multiple_params() {
    let d = AcDialect;
    let parsed = d
        .parser()
        .parse_handler_clause("\"fetch\"(url, method) as k: k(42)");
    let parsed = parsed.expect("should parse");
    assert_eq!(parsed.op_name, "fetch");
    assert_eq!(parsed.params, vec!["url", "method"]);
    assert_eq!(parsed.continuation_name, "k");
    assert_eq!(parsed.body, "k(42)");
}

#[test]
fn parse_handler_clause_fails_without_op_name() {
    let d = AcDialect;
    assert!(
        d.parser()
            .parse_handler_clause("(url) as k: k(42)")
            .is_none()
    );
}

#[test]
fn parse_handler_clause_fails_without_as() {
    let d = AcDialect;
    assert!(
        d.parser()
            .parse_handler_clause("\"fetch\"(url) k: k(42)")
            .is_none()
    );
}

#[test]
fn find_block_open_basic() {
    let d = AcDialect;
    let s = "handle(fetch(\"data\")) { \"fetch\"(url) as k: k(42) }";
    let pos = d.parser().find_block_open(s);
    assert_eq!(pos, Some(22));
}

#[test]
fn find_block_open_none() {
    let d = AcDialect;
    assert_eq!(d.parser().find_block_open("add(1, 2)"), None);
}
