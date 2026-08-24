use ecma402_traits::number_format::NumberFormatPartType;
use ecma402_traits::plural_rules::PluralCategory;
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

#[test]
fn english_second_singular() {
    let formatter = RelativeTimeFormat::new("en", Options::new(RelativeTimeUnit::Second)).unwrap();
    assert_eq!(formatter.format(1.0).unwrap(), "in 1 second");
    assert_eq!(formatter.format(-1.0).unwrap(), "1 second ago");
}

#[test]
fn english_second_plural() {
    let formatter = RelativeTimeFormat::new("en", Options::new(RelativeTimeUnit::Second)).unwrap();
    assert_eq!(formatter.format(2.0).unwrap(), "in 2 seconds");
    assert_eq!(formatter.format(-2.0).unwrap(), "2 seconds ago");
}

#[test]
fn german_second_singular() {
    let formatter = RelativeTimeFormat::new("de", Options::new(RelativeTimeUnit::Second)).unwrap();
    assert_eq!(formatter.format(1.0).unwrap(), "in 1 Sekunde");
}

#[test]
fn select_plural_english() {
    let formatter = RelativeTimeFormat::new("en", Options::new(RelativeTimeUnit::Second)).unwrap();
    assert_eq!(formatter.select_plural(1.0).unwrap(), PluralCategory::One);
    assert_eq!(formatter.select_plural(2.0).unwrap(), PluralCategory::Other);
}

#[test]
fn select_plural_polish() {
    let formatter = RelativeTimeFormat::new("pl", Options::new(RelativeTimeUnit::Second)).unwrap();
    assert_eq!(formatter.select_plural(1.0).unwrap(), PluralCategory::One);
    assert_eq!(formatter.select_plural(2.0).unwrap(), PluralCategory::Few);
    assert_eq!(formatter.select_plural(5.0).unwrap(), PluralCategory::Many);
}

#[test]
fn select_plural_russian() {
    let formatter = RelativeTimeFormat::new("ru", Options::new(RelativeTimeUnit::Second)).unwrap();
    assert_eq!(formatter.select_plural(1.0).unwrap(), PluralCategory::One);
    assert_eq!(formatter.select_plural(2.0).unwrap(), PluralCategory::Few);
    assert_eq!(formatter.select_plural(5.0).unwrap(), PluralCategory::Many);
}

#[test]
fn select_plural_arabic() {
    let formatter = RelativeTimeFormat::new("ar", Options::new(RelativeTimeUnit::Second)).unwrap();
    assert_eq!(formatter.select_plural(0.0).unwrap(), PluralCategory::Zero);
    assert_eq!(formatter.select_plural(1.0).unwrap(), PluralCategory::One);
    assert_eq!(formatter.select_plural(2.0).unwrap(), PluralCategory::Two);
    assert_eq!(formatter.select_plural(3.0).unwrap(), PluralCategory::Few);
    assert_eq!(formatter.select_plural(11.0).unwrap(), PluralCategory::Many);
    assert_eq!(
        formatter.select_plural(100.0).unwrap(),
        PluralCategory::Other
    );
}

#[test]
fn select_plural_bengali_japanese() {
    let bn = RelativeTimeFormat::new("bn", Options::new(RelativeTimeUnit::Second)).unwrap();
    assert_eq!(bn.select_plural(1.0).unwrap(), PluralCategory::One);
    assert_eq!(bn.select_plural(2.0).unwrap(), PluralCategory::Other);

    let ja = RelativeTimeFormat::new("ja", Options::new(RelativeTimeUnit::Second)).unwrap();
    assert_eq!(ja.select_plural(1.0).unwrap(), PluralCategory::Other);
}