use std::cmp::Ordering;

use ecma402_traits::collator::{Collator as _, Sensitivity};

use super::collator::{Collator, Options};

#[test]
fn compare_less() {
    let collator = Collator::new("en", Options::default()).unwrap();
    assert_eq!(collator.compare("a", "b").unwrap(), Ordering::Less);
}

#[test]
fn compare_equal() {
    let collator = Collator::new("en", Options::default()).unwrap();
    assert_eq!(collator.compare("a", "a").unwrap(), Ordering::Equal);
}

#[test]
fn compare_greater() {
    let collator = Collator::new("en", Options::default()).unwrap();
    assert_eq!(collator.compare("b", "a").unwrap(), Ordering::Greater);
}

#[test]
fn base_sensitivity() {
    let options = Options {
        sensitivity: Sensitivity::Base,
        ..Default::default()
    };
    let collator = Collator::new("de", options).unwrap();
    assert_eq!(collator.compare("ä", "a").unwrap(), Ordering::Equal);
}

#[test]
fn numeric_sort() {
    let options = Options {
        numeric: Some(true),
        ..Default::default()
    };
    let collator = Collator::new("en", options).unwrap();
    assert_eq!(
        collator.compare("file10", "file2").unwrap(),
        Ordering::Greater
    );
}
