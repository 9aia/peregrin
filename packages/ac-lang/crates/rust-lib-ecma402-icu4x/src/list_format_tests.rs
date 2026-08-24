use ecma402_traits::list_format::{ListFormat as _, ListStyle, ListType};

use super::list_format::{ListFormat, Options};

#[test]
fn conjunction_en_three() {
    let formatter = ListFormat::new("en", Options::default()).unwrap();
    assert_eq!(formatter.format(&["a", "b", "c"]).unwrap(), "a, b, and c");
}

#[test]
fn conjunction_en_two() {
    let formatter = ListFormat::new("en", Options::default()).unwrap();
    assert_eq!(formatter.format(&["a", "b"]).unwrap(), "a and b");
}

#[test]
fn disjunction() {
    let formatter = ListFormat::new(
        "en",
        Options {
            list_type: ListType::Disjunction,
            style: ListStyle::default(),
        },
    )
    .unwrap();
    assert_eq!(formatter.format(&["a", "b", "c"]).unwrap(), "a, b, or c");
}

#[test]
fn unit() {
    let formatter = ListFormat::new(
        "en",
        Options {
            list_type: ListType::Unit,
            style: ListStyle::default(),
        },
    )
    .unwrap();
    let formatted = formatter.format(&["5", "7"]).unwrap();
    assert!(formatted.contains("5"));
    assert!(formatted.contains("7"));
}

#[test]
fn spanish_pair() {
    let formatter = ListFormat::new("es", Options::default()).unwrap();
    assert_eq!(
        formatter.format(&["España", "Suiza"]).unwrap(),
        "España y Suiza"
    );
}

#[test]
fn empty_and_single() {
    let formatter = ListFormat::new("en", Options::default()).unwrap();
    assert_eq!(formatter.format(&[]).unwrap(), "");
    assert_eq!(formatter.format(&["x"]).unwrap(), "x");
}

#[test]
fn format_to_parts_three() {
    let formatter = ListFormat::new("en", Options::default()).unwrap();
    let parts = formatter.format_to_parts(&["a", "b", "c"]).unwrap();
    assert_eq!(parts.len(), 5);
    assert_eq!(parts[0].part_type.as_str(), "element");
    assert_eq!(parts[0].value, "a");
    assert_eq!(parts[1].part_type.as_str(), "literal");
    assert_eq!(parts[1].value, ", ");
    assert_eq!(parts[2].part_type.as_str(), "element");
    assert_eq!(parts[2].value, "b");
    assert_eq!(parts[3].part_type.as_str(), "literal");
    assert_eq!(parts[3].value, ", and ");
    assert_eq!(parts[4].part_type.as_str(), "element");
    assert_eq!(parts[4].value, "c");
}

#[test]
fn format_to_parts_two() {
    let formatter = ListFormat::new("en", Options::default()).unwrap();
    let parts = formatter.format_to_parts(&["a", "b"]).unwrap();
    assert_eq!(parts.len(), 3);
    assert_eq!(parts[0].part_type.as_str(), "element");
    assert_eq!(parts[0].value, "a");
    assert_eq!(parts[1].part_type.as_str(), "literal");
    assert_eq!(parts[1].value, " and ");
    assert_eq!(parts[2].part_type.as_str(), "element");
    assert_eq!(parts[2].value, "b");
}