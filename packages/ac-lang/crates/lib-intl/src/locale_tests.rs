use super::locale::Locale;

#[test]
fn parse_en() {
    let loc = Locale::parse("en").unwrap();
    assert_eq!(loc.language(), "en");
    assert_eq!(loc.region(), None);
}

#[test]
fn parse_pt_br() {
    let loc = Locale::parse("pt-BR").unwrap();
    assert_eq!(loc.language(), "pt");
}

#[test]
fn parse_uppercase_lang() {
    let loc = Locale::parse("EN").unwrap();
    assert_eq!(loc.language(), "en");
}

#[test]
fn parse_unknown_locale() {
    let result = Locale::parse("xyz");
    assert!(result.is_err());
}

#[test]
fn fallback_is_en() {
    let loc = Locale::fallback();
    assert_eq!(loc.language(), "en");
}

#[test]
fn default_is_fallback() {
    let loc = Locale::default();
    assert_eq!(loc, Locale::fallback());
}

#[test]
fn display() {
    let loc = Locale::parse("pt-BR").unwrap();
    assert!(format!("{loc}").starts_with("pt"));
}

#[test]
fn data_lookup() {
    let loc = Locale::parse("pt").unwrap();
    let data = loc.data();
    assert_eq!(data.decimal_sep, ",");
    assert_eq!(data.thousands_sep, ".");
}

#[test]
fn data_fallback_for_unknown() {
    let loc = Locale::fallback();
    let data = loc.data();
    assert_eq!(data.decimal_sep, ".");
}
