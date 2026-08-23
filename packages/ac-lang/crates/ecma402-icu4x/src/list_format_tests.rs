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
