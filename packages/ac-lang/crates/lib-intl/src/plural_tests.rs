use super::locale::Locale;
use super::plural::{plural_category, PluralCategory};

#[test]
fn english_one() {
    let loc = Locale::en();
    assert_eq!(plural_category(&loc, 1.0), PluralCategory::One);
}

#[test]
fn english_other() {
    let loc = Locale::en();
    assert_eq!(plural_category(&loc, 0.0), PluralCategory::Other);
    assert_eq!(plural_category(&loc, 2.0), PluralCategory::Other);
    assert_eq!(plural_category(&loc, 100.0), PluralCategory::Other);
}

#[test]
fn arabic_zero() {
    let loc = Locale::parse("ar").unwrap();
    assert_eq!(plural_category(&loc, 0.0), PluralCategory::Zero);
}

#[test]
fn arabic_one() {
    let loc = Locale::parse("ar").unwrap();
    assert_eq!(plural_category(&loc, 1.0), PluralCategory::One);
}

#[test]
fn arabic_two() {
    let loc = Locale::parse("ar").unwrap();
    assert_eq!(plural_category(&loc, 2.0), PluralCategory::Two);
}

#[test]
fn arabic_few() {
    let loc = Locale::parse("ar").unwrap();
    assert_eq!(plural_category(&loc, 3.0), PluralCategory::Few);
    assert_eq!(plural_category(&loc, 10.0), PluralCategory::Few);
}

#[test]
fn arabic_many() {
    let loc = Locale::parse("ar").unwrap();
    assert_eq!(plural_category(&loc, 11.0), PluralCategory::Many);
    assert_eq!(plural_category(&loc, 99.0), PluralCategory::Many);
}

#[test]
fn arabic_other() {
    let loc = Locale::parse("ar").unwrap();
    assert_eq!(plural_category(&loc, 100.0), PluralCategory::Other);
    assert_eq!(plural_category(&loc, 101.0), PluralCategory::Other);
}

#[test]
fn japanese_other_only() {
    let loc = Locale::parse("ja").unwrap();
    assert_eq!(plural_category(&loc, 1.0), PluralCategory::Other);
    assert_eq!(plural_category(&loc, 100.0), PluralCategory::Other);
}

#[test]
fn russian_few() {
    let loc = Locale::parse("ru").unwrap();
    assert_eq!(plural_category(&loc, 2.0), PluralCategory::Few);
    assert_eq!(plural_category(&loc, 3.0), PluralCategory::Few);
    assert_eq!(plural_category(&loc, 4.0), PluralCategory::Few);
}

#[test]
fn russian_many() {
    let loc = Locale::parse("ru").unwrap();
    assert_eq!(plural_category(&loc, 5.0), PluralCategory::Many);
    assert_eq!(plural_category(&loc, 0.0), PluralCategory::Many);
}

#[test]
fn category_as_str() {
    assert_eq!(PluralCategory::One.as_str(), "one");
    assert_eq!(PluralCategory::Other.as_str(), "other");
    assert_eq!(PluralCategory::Zero.as_str(), "zero");
}
