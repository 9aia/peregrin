use crate::list::*;
use ecma402_traits::list_format::{ListFormatPart, ListFormatPartType};

#[test]
fn placeholder_for_index() {
    assert_eq!(placeholder_for(0), "\u{E000}0\u{E001}");
    assert_eq!(placeholder_for(42), "\u{E000}42\u{E001}");
}

#[test]
fn placeholders_count() {
    assert_eq!(placeholders(0), Vec::<String>::new());
    assert_eq!(
        placeholders(2),
        vec![placeholder_for(0), placeholder_for(1)]
    );
}

#[test]
fn split_parts_empty_items() {
    assert_eq!(split_parts(&[], "anything"), Vec::<ListFormatPart>::new());
}

#[test]
fn split_parts_one_item_exact() {
    let ph = placeholder_for(0);
    let formatted = ph.clone();
    assert_eq!(
        split_parts(&["x"], &formatted),
        vec![ListFormatPart {
            part_type: ListFormatPartType::Element,
            value: "x".to_string()
        }]
    );
}

#[test]
fn split_parts_two_items_with_separator() {
    let formatted = format!("{}, {}", placeholder_for(0), placeholder_for(1));
    assert_eq!(
        split_parts(&["a", "b"], &formatted),
        vec![
            ListFormatPart {
                part_type: ListFormatPartType::Element,
                value: "a".to_string()
            },
            ListFormatPart {
                part_type: ListFormatPartType::Literal,
                value: ", ".to_string()
            },
            ListFormatPart {
                part_type: ListFormatPartType::Element,
                value: "b".to_string()
            },
        ]
    );
}

#[test]
fn split_parts_with_leading_and_trailing() {
    let formatted = format!(
        "start: {} and {} end",
        placeholder_for(0),
        placeholder_for(1)
    );
    assert_eq!(
        split_parts(&["a", "b"], &formatted),
        vec![
            ListFormatPart {
                part_type: ListFormatPartType::Literal,
                value: "start: ".to_string()
            },
            ListFormatPart {
                part_type: ListFormatPartType::Element,
                value: "a".to_string()
            },
            ListFormatPart {
                part_type: ListFormatPartType::Literal,
                value: " and ".to_string()
            },
            ListFormatPart {
                part_type: ListFormatPartType::Element,
                value: "b".to_string()
            },
            ListFormatPart {
                part_type: ListFormatPartType::Literal,
                value: " end".to_string()
            },
        ]
    );
}

#[test]
fn split_parts_missing_first_marker_non_empty_remaining() {
    assert_eq!(
        split_parts(&["a", "b"], "no markers"),
        vec![ListFormatPart {
            part_type: ListFormatPartType::Literal,
            value: "no markers".to_string()
        }]
    );
}

#[test]
fn split_parts_missing_first_marker_empty_remaining() {
    assert_eq!(split_parts(&["a"], ""), Vec::<ListFormatPart>::new());
}

#[test]
fn split_parts_missing_later_marker_non_empty_remaining() {
    let ph = placeholder_for(0);
    let formatted = format!("{} tail", ph);
    assert_eq!(
        split_parts(&["a", "b"], &formatted),
        vec![
            ListFormatPart {
                part_type: ListFormatPartType::Element,
                value: "a".to_string()
            },
            ListFormatPart {
                part_type: ListFormatPartType::Literal,
                value: " tail".to_string()
            },
        ]
    );
}

#[test]
fn split_parts_missing_later_marker_empty_remaining() {
    let ph = placeholder_for(0);
    assert_eq!(
        split_parts(&["a", "b"], &ph),
        vec![ListFormatPart {
            part_type: ListFormatPartType::Element,
            value: "a".to_string()
        }]
    );
}