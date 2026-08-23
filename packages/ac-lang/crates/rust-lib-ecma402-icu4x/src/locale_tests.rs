use ecma402_traits::locale::{HourCycle, Locale as _, LocaleOverrides};

use super::{Error, Locale};

#[test]
fn parse_no_overrides() {
    let info = Locale::parse("en-Latn-US", LocaleOverrides::default()).unwrap();
    assert_eq!(info.language, "en");
    assert_eq!(info.script, Some("Latn".to_string()));
    assert_eq!(info.region, Some("US".to_string()));
    assert_eq!(info.calendar, None);
    assert_eq!(info.numbering_system, None);
    assert_eq!(info.hour_cycle, None);
    assert_eq!(info.base_name, "en-Latn-US");
}

#[test]
fn parse_all_overrides() {
    let overrides = LocaleOverrides {
        language: Some("fr".to_string()),
        script: Some("Latn".to_string()),
        region: Some("CA".to_string()),
        calendar: Some("buddhist".to_string()),
        numbering_system: Some("latn".to_string()),
        hour_cycle: Some(HourCycle::H23),
    };
    let info = Locale::parse("en", overrides).unwrap();
    assert_eq!(info.language, "fr");
    assert_eq!(info.script, Some("Latn".to_string()));
    assert_eq!(info.region, Some("CA".to_string()));
    assert_eq!(info.calendar, Some("buddhist".to_string()));
    assert_eq!(info.numbering_system, Some("latn".to_string()));
    assert_eq!(info.hour_cycle, Some(HourCycle::H23));
    assert!(info.base_name.starts_with("fr-Latn-CA-u-"));
    assert!(info.base_name.contains("-ca-buddhist"));
    assert!(info.base_name.contains("-nu-latn"));
    assert!(info.base_name.contains("-hc-h23"));
}

#[test]
fn parse_invalid_locale() {
    let result = Locale::parse("en!", LocaleOverrides::default());
    assert!(matches!(result, Err(Error::InvalidLocale(_))));
}

#[test]
fn parse_invalid_subtag() {
    let overrides = LocaleOverrides {
        language: Some("!!!".to_string()),
        ..Default::default()
    };
    let result = Locale::parse("en", overrides);
    assert!(matches!(result, Err(Error::InvalidSubtag { .. })));
}
