use super::{GetCanonicalLocales, SupportedLocalesOf};
use ecma402_traits::locale_match::SupportedLocalesOf as _;
use icu_provider_blob::BlobDataProvider;

#[test]
fn canonicalize_valid_and_invalid() {
    let valid = &["fr", "en-US", "de-DE"];
    let canonical = GetCanonicalLocales::canonicalize(valid).unwrap();
    assert_eq!(canonical, vec!["fr", "en-US", "de-DE"]);

    assert!(GetCanonicalLocales::canonicalize(&[""]).is_err());
    assert!(GetCanonicalLocales::canonicalize(&["en-US", ""]).is_err());
}

#[test]
fn supported_locales_compiled() {
    let matcher = SupportedLocalesOf::new();
    let result = matcher
        .supported_locales_of(&["en", "xx-XX", "de"])
        .unwrap();
    assert_eq!(result, vec!["en", "de"]);

    let all = matcher.all_supported().unwrap();
    assert!(all.iter().any(|l| l.starts_with("en")));
    assert!(all.iter().any(|l| l.starts_with("de")));
    assert!(!all.iter().any(|l| l.starts_with("xx")));
}

#[test]
fn supported_locales_blob() {
    let blob: &'static [u8] = include_bytes!("../tests/data/decimal.postcard");
    let provider = BlobDataProvider::try_new_from_static_blob(blob).unwrap();
    let matcher = SupportedLocalesOf::with_blob(&provider);

    let result = matcher.supported_locales_of(&["en", "fr", "de"]).unwrap();
    assert_eq!(result, vec!["en", "de"]);
}
