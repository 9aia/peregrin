//! Integration tests for `ecma402-traits` using a trivial mock backend.
//!
//! These tests do not verify real formatting or locale data. They prove that
//! the public API is implementable and callable.

use std::cmp::Ordering;

use rust_lib_ecma402_traits::collator::Collator;
use rust_lib_ecma402_traits::date_time_format::{
    DateTimeFormat, DateTimeFormatPart, DateTimeFormatPartType,
};
use rust_lib_ecma402_traits::display_names::{DisplayNames, DisplayNamesOptions, Type};
use rust_lib_ecma402_traits::list_format::{ListFormat, ListFormatPart, ListFormatPartType};
use rust_lib_ecma402_traits::locale::{Locale, LocaleInfo, LocaleOverrides};
use rust_lib_ecma402_traits::locale_match::{CanonicalLocales, SupportedLocalesOf};
use rust_lib_ecma402_traits::number_format::{
    NumberFormat, NumberFormatPart, NumberFormatPartType,
};
use rust_lib_ecma402_traits::plural_rules::{PluralCategory, PluralRules};
use rust_lib_ecma402_traits::relative_time::{RelativeTimeFormat, RelativeTimeFormatPart};
use rust_lib_ecma402_traits::segmenter::{Segment, Segmenter};

#[derive(Debug)]
struct MockCollator;

impl Collator for MockCollator {
    type Error = String;

    fn compare(&self, a: &str, b: &str) -> Result<Ordering, Self::Error> {
        Ok(a.cmp(b))
    }
}

#[test]
fn collator_compare() {
    let collator = MockCollator;
    assert_eq!(collator.compare("a", "b").unwrap(), Ordering::Less);
    assert_eq!(collator.compare("a", "a").unwrap(), Ordering::Equal);
    assert_eq!(collator.compare("b", "a").unwrap(), Ordering::Greater);
}

#[derive(Debug)]
struct MockDateTimeFormat;

impl DateTimeFormat for MockDateTimeFormat {
    type Error = String;

    fn format(&self, value: &str) -> Result<String, Self::Error> {
        Ok(value.to_string())
    }

    fn format_to_parts(&self, value: &str) -> Result<Vec<DateTimeFormatPart>, Self::Error> {
        Ok(vec![DateTimeFormatPart {
            part_type: DateTimeFormatPartType::Literal,
            value: value.to_string(),
        }])
    }
}

#[test]
fn date_time_format() {
    let formatter = MockDateTimeFormat;
    assert_eq!(formatter.format("2024-01-15").unwrap(), "2024-01-15");
    assert_eq!(
        formatter.format_to_parts("2024-01-15").unwrap(),
        vec![DateTimeFormatPart {
            part_type: DateTimeFormatPartType::Literal,
            value: "2024-01-15".to_string()
        }]
    );
}

#[derive(Debug)]
struct MockDisplayNames;

impl DisplayNames for MockDisplayNames {
    type Error = String;

    fn of(
        &self,
        code: &str,
        _type: Type,
        _options: DisplayNamesOptions,
    ) -> Result<String, Self::Error> {
        Ok(code.to_string())
    }
}

#[test]
fn display_names_of() {
    let names = MockDisplayNames;
    let options = DisplayNamesOptions::default();
    assert_eq!(
        names.of("en", Type::Language, options.clone()).unwrap(),
        "en"
    );
    assert_eq!(names.of("US", Type::Region, options).unwrap(), "US");
}

#[derive(Debug)]
struct MockListFormat;

impl ListFormat for MockListFormat {
    type Error = String;

    fn format(&self, items: &[&str]) -> Result<String, Self::Error> {
        Ok(items.join(", "))
    }

    fn format_to_parts(&self, items: &[&str]) -> Result<Vec<ListFormatPart>, Self::Error> {
        Ok(items
            .iter()
            .enumerate()
            .flat_map(|(i, item)| {
                let mut v = Vec::new();
                if i > 0 {
                    v.push(ListFormatPart {
                        part_type: ListFormatPartType::Literal,
                        value: ", ".to_string(),
                    });
                }
                v.push(ListFormatPart {
                    part_type: ListFormatPartType::Element,
                    value: (*item).to_string(),
                });
                v
            })
            .collect())
    }
}

#[test]
fn list_format() {
    let formatter = MockListFormat;
    assert_eq!(formatter.format(&["a", "b", "c"]).unwrap(), "a, b, c");
}

#[derive(Debug)]
struct MockLocale;

impl Locale for MockLocale {
    type Error = String;

    fn parse(tag: &str, _overrides: LocaleOverrides) -> Result<LocaleInfo, Self::Error> {
        Ok(LocaleInfo {
            base_name: tag.to_string(),
            language: tag.split('-').next().unwrap_or(tag).to_string(),
            script: None,
            region: None,
            calendar: None,
            numbering_system: None,
            hour_cycle: None,
        })
    }
}

#[test]
fn locale_parse() {
    let info = MockLocale::parse("en-Latn-US", LocaleOverrides::default()).unwrap();
    assert_eq!(info.base_name, "en-Latn-US");
    assert_eq!(info.language, "en");
}

#[derive(Debug)]
struct MockLocaleMatch;

impl CanonicalLocales for MockLocaleMatch {
    type Error = String;

    fn canonicalize(locales: &[&str]) -> Result<Vec<String>, Self::Error> {
        Ok(locales.iter().map(|s| s.to_string()).collect())
    }
}

impl SupportedLocalesOf for MockLocaleMatch {
    type Error = String;

    fn supported_locales_of(&self, requested: &[&str]) -> Result<Vec<String>, Self::Error> {
        Ok(requested
            .iter()
            .filter(|l| l.starts_with("en"))
            .map(|s| s.to_string())
            .collect())
    }
}

#[test]
fn canonical_locales() {
    assert_eq!(
        MockLocaleMatch::canonicalize(&["en", "fr"]),
        Ok(vec!["en".to_string(), "fr".to_string()])
    );
}

#[test]
fn supported_locales_of() {
    let matcher = MockLocaleMatch;
    assert_eq!(
        matcher.supported_locales_of(&["en", "fr", "en-GB"]),
        Ok(vec!["en".to_string(), "en-GB".to_string()])
    );
}

#[derive(Debug)]
struct MockNumberFormat;

impl NumberFormat for MockNumberFormat {
    type Error = String;

    fn format(&self, value: f64) -> Result<String, Self::Error> {
        Ok(format!("{value}"))
    }

    fn format_to_parts(&self, value: f64) -> Result<Vec<NumberFormatPart>, Self::Error> {
        Ok(vec![NumberFormatPart {
            part_type: NumberFormatPartType::Literal,
            value: format!("{value}"),
        }])
    }
}

#[test]
fn number_format() {
    let formatter = MockNumberFormat;
    assert_eq!(formatter.format(42.0).unwrap(), "42");
}

#[derive(Debug)]
struct MockPluralRules;

impl PluralRules for MockPluralRules {
    type Error = String;

    fn select(&self, n: f64) -> Result<PluralCategory, Self::Error> {
        Ok(if n == 1.0 {
            PluralCategory::One
        } else {
            PluralCategory::Other
        })
    }
}

#[test]
fn plural_rules() {
    let rules = MockPluralRules;
    assert_eq!(rules.select(1.0).unwrap(), PluralCategory::One);
    assert_eq!(rules.select(5.0).unwrap(), PluralCategory::Other);
}

#[derive(Debug)]
struct MockRelativeTimeFormat;

impl RelativeTimeFormat for MockRelativeTimeFormat {
    type Error = String;

    fn format(&self, value: f64) -> Result<String, Self::Error> {
        Ok(format!("{value}"))
    }

    fn format_to_parts(&self, value: f64) -> Result<Vec<RelativeTimeFormatPart>, Self::Error> {
        let text = format!("{value}");
        let pos = text.find('.').unwrap_or(text.len());
        let mut parts = Vec::new();
        parts.push(RelativeTimeFormatPart {
            part_type: NumberFormatPartType::Integer,
            value: text[..pos].to_string(),
        });
        if pos < text.len() {
            parts.push(RelativeTimeFormatPart {
                part_type: NumberFormatPartType::Decimal,
                value: ".".to_string(),
            });
            parts.push(RelativeTimeFormatPart {
                part_type: NumberFormatPartType::Fraction,
                value: text[pos + 1..].to_string(),
            });
        }
        Ok(parts)
    }
}

#[test]
fn relative_time_format() {
    let formatter = MockRelativeTimeFormat;
    assert_eq!(formatter.format(3.0).unwrap(), "3");
}

#[derive(Debug)]
struct MockSegmenter;

impl Segmenter for MockSegmenter {
    type Error = String;

    fn segment(&self, text: &str) -> Result<Vec<Segment>, Self::Error> {
        Ok(vec![Segment {
            text: text.to_string(),
            index: 0,
            is_word_like: None,
        }])
    }
}

#[test]
fn segmenter() {
    let segmenter = MockSegmenter;
    let segments = segmenter.segment("hello").unwrap();
    assert_eq!(segments.len(), 1);
    assert_eq!(segments[0].text, "hello");
    assert_eq!(segments[0].index, 0);
    assert_eq!(segments[0].is_word_like, None);
}
