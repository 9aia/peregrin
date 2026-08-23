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
