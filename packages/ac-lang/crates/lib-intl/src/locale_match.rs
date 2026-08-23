use ac_runtime::NonFunctionValue;
use ac_runtime::{Context, Error, Function};
use icu::decimal::provider::DecimalSymbolsV1;
use icu_provider::{DataMarker, IterableDynamicDataProvider};

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
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let locales = if args.is_empty() {
            return Err(IntlError::InvalidArgument {
                message: "at least one locale required".into(),
            }
            .into_vm_error());
        } else if let Some(arr) = args[0].as_array() {
            arr.iter()
                .map(|v| {
                    v.as_str()
                        .ok_or_else(|| {
                            IntlError::InvalidArgument {
                                message: "locale must be a string".into(),
                            }
                            .into_vm_error()
                        })
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
/// Filters the requested locales to those supported by the available data.
/// With the default compiled provider, uses a hardcoded list of supported languages.
/// With a blob provider, queries the blob's actual locale inventory via
/// `DecimalSymbolsV1` data identifiers — does not claim support for locales
/// the blob doesn't contain.
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
        Self {
            provider: Some(provider),
        }
    }
}

impl Default for SupportedLocalesOf {
    fn default() -> Self {
        Self::new()
    }
}

/// Languages known to be present in the compiled ICU4X CLDR data.
///
/// ICU4X compiled data does not expose a public locale inventory API, so this
/// list is the practical fallback for `supported_locales_of` when no blob
/// provider is configured. With a blob provider, the actual inventory is
/// enumerated dynamically.
const SUPPORTED_LANGUAGES: &[&str] = &[
    "af", "am", "ar", "az", "be", "bg", "bn", "bs", "ca", "cs", "cy", "da", "de", "el", "en", "es",
    "et", "eu", "fa", "fi", "fil", "fr", "ga", "gl", "gu", "he", "hi", "hr", "hu", "hy", "id",
    "is", "it", "ja", "ka", "kk", "km", "kn", "ko", "ky", "lo", "lt", "lv", "mk", "ml", "mn", "mr",
    "ms", "my", "nb", "ne", "nl", "pl", "pt", "ro", "ru", "si", "sk", "sl", "sq", "sr", "sv", "sw",
    "ta", "te", "th", "tr", "uk", "ur", "uz", "vi", "zh",
];

fn is_supported_compiled(locale: &str) -> bool {
    if let Ok(loc) = locale.parse::<icu::locale::Locale>() {
        let lang = loc.id.language.as_str();
        SUPPORTED_LANGUAGES.contains(&lang)
    } else {
        false
    }
}

fn blob_supported_languages(
    provider: &super::provider::IntlProvider,
) -> Result<Vec<String>, Error> {
    let bdp = provider.as_blob_data_provider().map_err(|e| {
        IntlError::DataProvider {
            message: e.to_string(),
        }
        .into_vm_error()
    })?;
    let ids = bdp
        .iter_ids_for_marker(DecimalSymbolsV1::INFO)
        .map_err(|e| {
            IntlError::DataProvider {
                message: e.to_string(),
            }
            .into_vm_error()
        })?;
    let langs: Vec<String> = ids
        .into_iter()
        .map(|id| id.locale.to_string())
        .filter(|l| l != "und")
        .collect();
    Ok(langs)
}

impl Function for SupportedLocalesOf {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let blob_langs = if let Some(provider) = &self.provider {
            Some(blob_supported_languages(provider)?)
        } else {
            None
        };

        let requested = if args.is_empty() {
            let langs = match &blob_langs {
                Some(l) => l.clone(),
                None => SUPPORTED_LANGUAGES.iter().map(|s| s.to_string()).collect(),
            };
            let arr: Vec<serde_json::Value> =
                langs.into_iter().map(|l| serde_json::json!(l)).collect();
            return Ok(NonFunctionValue::new(serde_json::Value::Array(arr)));
        } else if let Some(arr) = args[0].as_array() {
            arr.iter()
                .map(|v| {
                    v.as_str()
                        .ok_or_else(|| {
                            IntlError::InvalidArgument {
                                message: "locale must be a string".into(),
                            }
                            .into_vm_error()
                        })
                        .map(|s| s.to_string())
                })
                .collect::<Result<Vec<_>, Error>>()?
        } else {
            let single = require_string(args, 0)?;
            vec![single.to_string()]
        };

        let is_supported = |l: &str| -> bool {
            match &blob_langs {
                Some(langs) => {
                    let parsed = l.parse::<icu::locale::Locale>().ok();
                    match parsed {
                        Some(loc) => {
                            let lang = loc.id.language.as_str();
                            langs.iter().any(|supported| {
                                supported == l
                                    || supported == lang
                                    || supported
                                        .parse::<icu::locale::Locale>()
                                        .is_ok_and(|s| s.id.language.as_str() == lang)
                            })
                        }
                        None => langs.iter().any(|supported| supported == l),
                    }
                }
                None => is_supported_compiled(l),
            }
        };

        let filtered: Vec<serde_json::Value> = requested
            .iter()
            .filter(|l| is_supported(l))
            .map(|l| serde_json::json!(l))
            .collect();

        Ok(NonFunctionValue::new(serde_json::Value::Array(filtered)))
    }
}
