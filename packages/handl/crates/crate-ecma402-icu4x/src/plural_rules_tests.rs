use ecma402_traits::plural_rules::{PluralCategory, PluralRuleType, PluralRules as _}; // trait in scope

use super::plural_rules::{Options, PluralRules};

#[test]
fn cardinal_en() {
    let rules = PluralRules::new("en", Options::default()).unwrap();
    assert_eq!(rules.select(1.0).unwrap(), PluralCategory::One);
    assert_eq!(rules.select(2.0).unwrap(), PluralCategory::Other);
    assert_eq!(rules.select(0.0).unwrap(), PluralCategory::Other);
}

#[test]
fn ordinal_en() {
    let rules = PluralRules::new(
        "en",
        Options {
            rule_type: PluralRuleType::Ordinal,
        },
    )
    .unwrap();
    assert_eq!(rules.select(1.0).unwrap(), PluralCategory::One);
    assert_eq!(rules.select(2.0).unwrap(), PluralCategory::Two);
    assert_eq!(rules.select(11.0).unwrap(), PluralCategory::Other);
}

#[test]
fn cardinal_ar() {
    let rules = PluralRules::new("ar", Options::default()).unwrap();
    assert_eq!(rules.select(0.0).unwrap(), PluralCategory::Zero);
    assert_eq!(rules.select(1.0).unwrap(), PluralCategory::One);
    assert_eq!(rules.select(2.0).unwrap(), PluralCategory::Two);
    assert_eq!(rules.select(3.0).unwrap(), PluralCategory::Few);
    assert_eq!(rules.select(11.0).unwrap(), PluralCategory::Many);
    assert_eq!(rules.select(100.0).unwrap(), PluralCategory::Other);
}

#[test]
fn invalid_locale_errors() {
    let result = PluralRules::new("!", Options::default());
    assert!(result.is_err());
}
