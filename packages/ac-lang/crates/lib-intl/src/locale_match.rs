use ac_runtime::NonFunctionValue;
use ac_vm::{Context, Error, Function};

use super::common::require_string;
use super::error::IntlError;

/// `intl.get_canonical_locales(locales: string | string[]) → string[]`
///
/// Returns the canonicalized form of the given locale or list of locales.
///
/// Examples:
/// - `intl.get_canonical_locales("en-us")` → `["en-US"]`
/// - `intl.get_canonical_locales(["de", "ja-jp"])` → `["de", "ja-JP"]`
#[derive(Clone)]
pub struct GetCanonicalLocales;

impl Default for GetCanonicalLocales {
    fn default() -> Self {
        Self
    }
}

impl Function for GetCanonicalLocales {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let locales = if args.is_empty() {
            return Err(IntlError::InvalidArgument {
                message: "at least one locale required".into(),
            }
            .into_vm_error());
        } else if let Some(arr) = args[0].as_array() {
            arr.iter()
                .map(|v| {
                    v.as_str()
                        .ok_or_else(|| IntlError::InvalidArgument {
                            message: "locale must be a string".into(),
                        }
                        .into_vm_error())
                        .map(|s| s.to_string())
                })
                .collect::<Result<Vec<_>, Error>>()?
        } else {
            let single = require_string(args, 0)?;
            vec![single.to_string()]
        };

        let canonical: Vec<serde_json::Value> = locales
            .iter()
            .map(|l| {
                let locale: String = l
                    .parse::<icu::locale::Locale>()
                    .map(|loc| loc.to_string())
                    .unwrap_or_else(|_| l.to_string());
                serde_json::json!(locale)
            })
            .collect();

        Ok(NonFunctionValue::new(serde_json::Value::Array(canonical)))
    }
}

/// `intl.supported_locales_of(locales: string | string[]) → string[]`
///
/// Filters the requested locales to those supported by the compiled CLDR data.
/// Uses best-fit matching (language subtag match).
///
/// Examples:
/// - `intl.supported_locales_of("en")` → `["en"]`
/// - `intl.supported_locales_of(["de", "xxx"])` → `["de"]`
#[derive(Clone)]
pub struct SupportedLocalesOf {
    provider: Option<super::provider::IntlProvider>,
}

impl SupportedLocalesOf {
    pub fn new() -> Self {
        Self { provider: None }
    }

    pub fn with_provider(provider: super::provider::IntlProvider) -> Self {
        Self { provider: Some(provider) }
    }
}

impl Default for SupportedLocalesOf {
    fn default() -> Self {
        Self::new()
    }
}

const SUPPORTED_LANGUAGES: &[&str] = &[
    "af", "am", "ar", "az", "be", "bg", "bn", "bs", "ca", "cs",
    "cy", "da", "de", "el", "en", "es", "et", "eu", "fa", "fi",
    "fil", "fr", "ga", "gl", "gu", "he", "hi", "hr", "hu", "hy",
    "id", "is", "it", "ja", "ka", "kk", "km", "kn", "ko", "ky",
    "lo", "lt", "lv", "mk", "ml", "mn", "mr", "ms", "my", "nb",
    "ne", "nl", "pl", "pt", "ro", "ru", "si", "sk", "sl", "sq",
    "sr", "sv", "sw", "ta", "te", "th", "tr", "uk", "ur", "uz",
    "vi", "zh",
];

fn is_supported(locale: &str) -> bool {
    if let Ok(loc) = locale.parse::<icu::locale::Locale>() {
        let lang = loc.id.language.as_str();
        SUPPORTED_LANGUAGES.contains(&lang)
    } else {
        false
    }
}

impl Function for SupportedLocalesOf {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        if let Some(provider) = &self.provider {
            let _ = provider.as_bytes();
        }

        let requested = if args.is_empty() {
            let arr: Vec<serde_json::Value> = SUPPORTED_LANGUAGES
                .iter()
                .map(|l| serde_json::json!(l))
                .collect();
            return Ok(NonFunctionValue::new(serde_json::Value::Array(arr)));
        } else if let Some(arr) = args[0].as_array() {
            arr.iter()
                .map(|v| {
                    v.as_str()
                        .ok_or_else(|| IntlError::InvalidArgument {
                            message: "locale must be a string".into(),
                        }
                        .into_vm_error())
                        .map(|s| s.to_string())
                })
                .collect::<Result<Vec<_>, Error>>()?
        } else {
            let single = require_string(args, 0)?;
            vec![single.to_string()]
        };

        let filtered: Vec<serde_json::Value> = requested
            .iter()
            .filter(|l| is_supported(l))
            .map(|l| serde_json::json!(l))
            .collect();

        Ok(NonFunctionValue::new(serde_json::Value::Array(filtered)))
    }
}
