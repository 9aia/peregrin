use crate::relative_time::*;
use ecma402_traits::number_format::{NumberFormatPart, NumberFormatPartType};
use ecma402_traits::relative_time::RelativeTimeFormatPart;

fn np(t: NumberFormatPartType, v: &str) -> NumberFormatPart {
    NumberFormatPart {
        part_type: t,
        value: v.to_string()}
}

fn rnp(t: NumberFormatPartType, v: &str) -> RelativeTimeFormatPart {
    RelativeTimeFormatPart {
        part_type: t,
        value: v.to_string()}
}

#[test]
fn split_parts_no_number_parts_non_empty_remaining() {
    assert_eq!(
        split_parts("hello", &[]),
        vec![rnp(NumberFormatPartType::Literal, "hello")]
    );
}

#[test]
fn split_parts_no_number_parts_empty_remaining() {
    assert_eq!(split_parts("", &[]), Vec::<RelativeTimeFormatPart>::new());
}

#[test]
fn split_parts_number_at_start() {
    assert_eq!(
        split_parts("42", &[np(NumberFormatPartType::Integer, "42")]),
        vec![rnp(NumberFormatPartType::Integer, "42")]
    );
}

#[test]
fn split_parts_number_in_middle() {
    assert_eq!(
        split_parts("in 42 days", &[np(NumberFormatPartType::Integer, "42")]),
        vec![
            rnp(NumberFormatPartType::Literal, "in "),
            rnp(NumberFormatPartType::Integer, "42"),
            rnp(NumberFormatPartType::Literal, " days"),
        ]
    );
}

#[test]
fn split_parts_multiple_numbers() {
    assert_eq!(
        split_parts(
            "1.5",
            &[
                np(NumberFormatPartType::Integer, "1"),
                np(NumberFormatPartType::Decimal, "."),
                np(NumberFormatPartType::Fraction, "5"),
            ]
        ),
        vec![
            rnp(NumberFormatPartType::Integer, "1"),
            rnp(NumberFormatPartType::Decimal, "."),
            rnp(NumberFormatPartType::Fraction, "5"),
        ]
    );
}

#[test]
fn split_parts_number_not_found_non_empty_remaining() {
    assert_eq!(
        split_parts("just text", &[np(NumberFormatPartType::Integer, "99")]),
        vec![rnp(NumberFormatPartType::Literal, "just text")]
    );
}

#[test]
fn split_parts_number_not_found_empty_remaining() {
    assert_eq!(
        split_parts("", &[np(NumberFormatPartType::Integer, "99")]),
        Vec::<RelativeTimeFormatPart>::new()
    );
}

#[test]
fn split_parts_missing_first_number_but_finding_second() {
    assert_eq!(
        split_parts(
            "42",
            &[
                np(NumberFormatPartType::Integer, "99"),
                np(NumberFormatPartType::Integer, "42"),
            ]
        ),
        vec![rnp(NumberFormatPartType::Integer, "42")]
    );
}

#[test]
fn split_parts_first_number_found_then_missing_with_remaining() {
    assert_eq!(
        split_parts(
            "42 days later",
            &[
                np(NumberFormatPartType::Integer, "42"),
                np(NumberFormatPartType::Integer, "99"),
            ]
        ),
        vec![
            rnp(NumberFormatPartType::Integer, "42"),
            rnp(NumberFormatPartType::Literal, " days later"),
        ]
    );
}
