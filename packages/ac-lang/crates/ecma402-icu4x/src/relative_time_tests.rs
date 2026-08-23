use ecma402_traits::number_format::NumberFormatPartType;
use ecma402_traits::relative_time::{
    RelativeTimeFormat as _, RelativeTimeNumeric, RelativeTimeStyle, RelativeTimeUnit,
};

use super::relative_time::{Options, RelativeTimeFormat};

#[test]
fn future_seconds() {
    let formatter = RelativeTimeFormat::new("en", Options::new(RelativeTimeUnit::Second)).unwrap();
    assert_eq!(formatter.format(5.0).unwrap(), "in 5 seconds");
}

#[test]
fn past_seconds() {
    let formatter = RelativeTimeFormat::new("en", Options::new(RelativeTimeUnit::Second)).unwrap();
    assert_eq!(formatter.format(-10.0).unwrap(), "10 seconds ago");
}

#[test]
fn short_style() {
    let formatter = RelativeTimeFormat::new(
        "en",
        Options {
            unit: RelativeTimeUnit::Second,
            style: RelativeTimeStyle::Short,
            numeric: RelativeTimeNumeric::default(),
        },
    )
    .unwrap();
    assert_eq!(formatter.format(5.0).unwrap(), "in 5 sec.");
}

#[test]
fn numeric_auto() {
    let formatter = RelativeTimeFormat::new(
        "en",
        Options {
            unit: RelativeTimeUnit::Day,
            style: RelativeTimeStyle::default(),
            numeric: RelativeTimeNumeric::Auto,
        },
    )
    .unwrap();
    assert_eq!(formatter.format(0.0).unwrap(), "today");
}

#[test]
fn german() {
    let formatter = RelativeTimeFormat::new("de", Options::new(RelativeTimeUnit::Second)).unwrap();
    assert_eq!(formatter.format(5.0).unwrap(), "in 5 Sekunden");
}

#[test]
fn format_to_parts_future() {
    let formatter = RelativeTimeFormat::new("en", Options::new(RelativeTimeUnit::Second)).unwrap();
    let parts = formatter.format_to_parts(5.0).unwrap();
    assert_eq!(parts.len(), 3);
    assert_eq!(parts[0].part_type, NumberFormatPartType::Literal);
    assert_eq!(parts[0].value, "in ");
    assert_eq!(parts[1].part_type, NumberFormatPartType::Integer);
    assert_eq!(parts[1].value, "5");
    assert_eq!(parts[2].part_type, NumberFormatPartType::Literal);
    assert_eq!(parts[2].value, " seconds");
}

#[test]
fn format_to_parts_past() {
    let formatter = RelativeTimeFormat::new("en", Options::new(RelativeTimeUnit::Second)).unwrap();
    let parts = formatter.format_to_parts(-10.0).unwrap();
    assert_eq!(parts.len(), 2);
    assert_eq!(parts[0].part_type, NumberFormatPartType::Integer);
    assert_eq!(parts[0].value, "10");
    assert_eq!(parts[1].part_type, NumberFormatPartType::Literal);
    assert_eq!(parts[1].value, " seconds ago");
}

#[test]
fn format_to_parts_auto() {
    let formatter = RelativeTimeFormat::new(
        "en",
        Options {
            unit: RelativeTimeUnit::Day,
            style: RelativeTimeStyle::default(),
            numeric: RelativeTimeNumeric::Auto,
        },
    )
    .unwrap();
    let parts = formatter.format_to_parts(0.0).unwrap();
    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0].part_type, NumberFormatPartType::Literal);
    assert_eq!(parts[0].value, "today");
}
