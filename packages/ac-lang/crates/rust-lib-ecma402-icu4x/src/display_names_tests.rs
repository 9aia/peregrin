use ecma402_traits::display_names::DisplayNames as _;

use super::{DisplayNames, Error, Fallback, LanguageDisplay, Options, Style, Type};

fn opts(style: Style, fallback: Fallback, language_display: LanguageDisplay) -> Options {
    Options {
        style,
        fallback,
        language_display}
}

#[test]
fn language_long() -> Result<(), Error> {
    let dn = DisplayNames::new("en", Options::default())?;
    let name = dn.of(
        "en",
        Type::Language,
        opts(Style::Long, Fallback::Code, LanguageDisplay::Dialect),
    )?;
    assert!(!name.is_empty());
    assert_ne!(name, "en");
    Ok(())
}

#[test]
fn language_styles_differ_in_length() -> Result<(), Error> {
    let dn = DisplayNames::new("en", Options::default())?;
    let short = dn.of(
        "en",
        Type::Language,
        opts(Style::Short, Fallback::Code, LanguageDisplay::Dialect),
    )?;
    let narrow = dn.of(
        "en",
        Type::Language,
        opts(Style::Narrow, Fallback::Code, LanguageDisplay::Dialect),
    )?;
    let long = dn.of(
        "en",
        Type::Language,
        opts(Style::Long, Fallback::Code, LanguageDisplay::Dialect),
    )?;
    assert!(!short.is_empty());
    assert!(!narrow.is_empty());
    assert!(!long.is_empty());
    assert!(short.len() <= long.len());
    assert!(narrow.len() <= long.len());
    Ok(())
}

#[test]
fn language_display_dialect_and_standard() -> Result<(), Error> {
    let dn = DisplayNames::new("en", Options::default())?;
    let dialect = dn.of(
        "pt-BR",
        Type::Language,
        opts(Style::Long, Fallback::Code, LanguageDisplay::Dialect),
    )?;
    let standard = dn.of(
        "pt-BR",
        Type::Language,
        opts(Style::Long, Fallback::Code, LanguageDisplay::Standard),
    )?;
    assert!(!dialect.is_empty());
    assert!(!standard.is_empty());
    Ok(())
}

#[test]
fn language_invalid_code() -> Result<(), Error> {
    let dn = DisplayNames::new("en", Options::default())?;
    assert!(
        dn.of("not-a-code", Type::Language, Options::default())
            .is_err()
    );
    Ok(())
}

#[test]
fn region_long_and_short() -> Result<(), Error> {
    let dn = DisplayNames::new("en", Options::default())?;
    let long = dn.of(
        "US",
        Type::Region,
        opts(Style::Long, Fallback::Code, LanguageDisplay::default()),
    )?;
    let short = dn.of(
        "US",
        Type::Region,
        opts(Style::Short, Fallback::Code, LanguageDisplay::default()),
    )?;
    assert!(!long.is_empty());
    assert!(!short.is_empty());
    assert!(short.len() <= long.len());
    Ok(())
}

#[test]
fn region_invalid_code() -> Result<(), Error> {
    let dn = DisplayNames::new("en", Options::default())?;
    assert!(dn.of("Q", Type::Region, Options::default()).is_err());
    Ok(())
}

#[test]
fn script_long_and_short() -> Result<(), Error> {
    let dn = DisplayNames::new("en", Options::default())?;
    let long = dn.of(
        "Latn",
        Type::Script,
        opts(Style::Long, Fallback::Code, LanguageDisplay::default()),
    )?;
    let short = dn.of(
        "Latn",
        Type::Script,
        opts(Style::Short, Fallback::Code, LanguageDisplay::default()),
    )?;
    assert!(!long.is_empty());
    assert!(!short.is_empty());
    Ok(())
}

#[test]
fn script_invalid_code() -> Result<(), Error> {
    let dn = DisplayNames::new("en", Options::default())?;
    assert!(dn.of("ABC", Type::Script, Options::default()).is_err());
    Ok(())
}

#[test]
fn variant_valid() -> Result<(), Error> {
    let dn = DisplayNames::new("en", Options::default())?;
    let name = dn.of("fonipa", Type::Variant, Options::default())?;
    assert!(!name.is_empty());
    assert_ne!(name, "fonipa");
    Ok(())
}

#[test]
fn variant_invalid_code() -> Result<(), Error> {
    let dn = DisplayNames::new("en", Options::default())?;
    assert!(dn.of("!nvalid", Type::Variant, Options::default()).is_err());
    Ok(())
}

#[test]
fn currency_valid() -> Result<(), Error> {
    let dn = DisplayNames::new("en", Options::default())?;
    let name = dn.of("USD", Type::Currency, Options::default())?;
    assert!(!name.is_empty());
    Ok(())
}

#[test]
fn currency_invalid_code() -> Result<(), Error> {
    let dn = DisplayNames::new("en", Options::default())?;
    assert!(dn.of("US$", Type::Currency, Options::default()).is_err());
    Ok(())
}

#[test]
fn calendar_valid() -> Result<(), Error> {
    let dn = DisplayNames::new("en", Options::default())?;
    let name = dn.of("gregory", Type::Calendar, Options::default())?;
    assert_eq!(name, "Gregorian Calendar");
    Ok(())
}

#[test]
fn calendar_invalid_fallback_code() -> Result<(), Error> {
    let dn = DisplayNames::new("en", Options::default())?;
    let name = dn.of(
        "unknown-cal",
        Type::Calendar,
        opts(Style::Long, Fallback::Code, LanguageDisplay::default()),
    )?;
    assert_eq!(name, "unknown-cal");
    Ok(())
}

#[test]
fn calendar_invalid_fallback_none() -> Result<(), Error> {
    let dn = DisplayNames::new("en", Options::default())?;
    let name = dn.of(
        "unknown-cal",
        Type::Calendar,
        opts(Style::Long, Fallback::None, LanguageDisplay::default()),
    )?;
    assert!(name.is_empty());
    Ok(())
}

#[test]
fn datetime_field_valid() -> Result<(), Error> {
    let dn = DisplayNames::new("en", Options::default())?;
    let name = dn.of("month", Type::DateTimeField, Options::default())?;
    assert_eq!(name, "month");
    Ok(())
}

#[test]
fn datetime_field_invalid_fallback_code() -> Result<(), Error> {
    let dn = DisplayNames::new("en", Options::default())?;
    let name = dn.of(
        "unknown-field",
        Type::DateTimeField,
        opts(Style::Long, Fallback::Code, LanguageDisplay::default()),
    )?;
    assert_eq!(name, "unknown-field");
    Ok(())
}

#[test]
fn datetime_field_invalid_fallback_none() -> Result<(), Error> {
    let dn = DisplayNames::new("en", Options::default())?;
    let name = dn.of(
        "unknown-field",
        Type::DateTimeField,
        opts(Style::Long, Fallback::None, LanguageDisplay::default()),
    )?;
    assert!(name.is_empty());
    Ok(())
}
