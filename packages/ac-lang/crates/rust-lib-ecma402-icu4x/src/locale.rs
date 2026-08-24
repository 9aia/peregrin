use std::fmt;

use ecma402_traits::locale::{HourCycle, Locale as LocaleTrait, LocaleInfo, LocaleOverrides};
use icu::locale::{
    Locale as IcuLocale,
    extensions::unicode::{Key, Value},
    subtags::{Language, Region, Script},
};

/// ICU4X-backed ECMA-402 `Intl.Locale`.
pub struct Locale;

/// Errors that can occur when parsing or modifying a locale.
#[derive(Debug)]
pub enum Error {
    InvalidLocale(String),
    InvalidSubtag { subtag: String, message: String },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLocale(s) => write!(f, "invalid locale: {s}"),
            Self::InvalidSubtag { subtag, message } => {
                write!(f, "invalid subtag {subtag}: {message}")
            }
        }
    }
}

impl std::error::Error for Error {}

impl LocaleTrait for Locale {
    type Error = Error;

    fn parse(tag: &str, overrides: LocaleOverrides) -> Result<LocaleInfo, Self::Error> {
        let mut locale = tag
            .parse::<IcuLocale>()
            .map_err(|e| Error::InvalidLocale(format!("{tag}: {e}")))?;

        if let Some(lang) = overrides.language {
            let parsed = lang.parse::<Language>().map_err(|e| Error::InvalidSubtag {
                subtag: lang,
                message: e.to_string(),
            })?;
            locale.id.language = parsed;
        }

        if let Some(script) = overrides.script {
            let parsed = script.parse::<Script>().map_err(|e| Error::InvalidSubtag {
                subtag: script,
                message: e.to_string(),
            })?;
            locale.id.script = Some(parsed);
        }

        if let Some(region) = overrides.region {
            let parsed = region.parse::<Region>().map_err(|e| Error::InvalidSubtag {
                subtag: region,
                message: e.to_string(),
            })?;
            locale.id.region = Some(parsed);
        }

        for (opt_key, opt_val) in [
            ("ca", overrides.calendar.as_deref()),
            ("nu", overrides.numbering_system.as_deref()),
        ] {
            if let Some(value) = opt_val {
                let key = opt_key.parse::<Key>().map_err(|e| Error::InvalidSubtag {
                    subtag: opt_key.to_string(),
                    message: e.to_string(),
                })?;
                let val = Value::try_from_str(value).map_err(|e| Error::InvalidSubtag {
                    subtag: value.to_string(),
                    message: e.to_string(),
                })?;
                locale.extensions.unicode.keywords.set(key, val);
            }
        }

        if let Some(hc) = overrides.hour_cycle {
            let key = "hc".parse::<Key>().map_err(|e| Error::InvalidSubtag {
                subtag: "hc".to_string(),
                message: e.to_string(),
            })?;
            let value = hc.as_str();
            let val = Value::try_from_str(value).map_err(|e| Error::InvalidSubtag {
                subtag: value.to_string(),
                message: e.to_string(),
            })?;
            locale.extensions.unicode.keywords.set(key, val);
        }

        let base_name = locale.to_string();
        let language = locale.id.language.to_string();
        let script = locale.id.script.map(|s| s.to_string());
        let region = locale.id.region.map(|r| r.to_string());

        let cal_key: Key = "ca".parse().unwrap();
        let nu_key: Key = "nu".parse().unwrap();
        let hc_key: Key = "hc".parse().unwrap();

        let calendar = locale
            .extensions
            .unicode
            .keywords
            .get(&cal_key)
            .and_then(|v| v.as_subtags_slice().first().map(|s| s.to_string()));
        let numbering_system = locale
            .extensions
            .unicode
            .keywords
            .get(&nu_key)
            .and_then(|v| v.as_subtags_slice().first().map(|s| s.to_string()));
        let hour_cycle = locale
            .extensions
            .unicode
            .keywords
            .get(&hc_key)
            .and_then(|v| v.as_subtags_slice().first())
            .and_then(|s| s.as_str().parse::<HourCycle>().ok());

        Ok(LocaleInfo {
            base_name,
            language,
            script,
            region,
            calendar,
            numbering_system,
            hour_cycle,
        })
    }
}

#[cfg(test)]
#[path = "locale_tests.rs"]
mod locale_tests;