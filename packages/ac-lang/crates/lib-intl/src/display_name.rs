use ac_runtime_values::NonFunctionValue;
use ac_runtime_values::{Error};
use ac_runtime_values::{Context, Function};
use icu::experimental::dimension::provider::currency::displayname::CurrencyDisplaynameV1;
use icu::locale::DataLocale;
use icu::locale::names::{
    LanguageDisplay, LanguageIdentifierDisplayName, LanguageIdentifierDisplayNameOptions,
    RegionDisplayName, ScriptDisplayName, VariantDisplayName,
};
use icu_provider::buf::AsDeserializingBufferProvider;
use icu_provider::{
    DataIdentifierBorrowed, DataMarkerAttributes, DataProvider, DataRequest, ResultDataError,
};
use writeable::Writeable;

use super::common::{make_provider, opt_str, optional_locale, optional_object, require_string, s};
use super::error::IntlError;
use super::provider::IntlProvider;

/// `intl.display_name(code: string, type: string, locale?: string, opts?: object) → string`
///
/// Returns the display name for a language, region, script, variant, currency, calendar,
/// or dateTimeField code.
///
/// `type` must be one of: `"language"`, `"region"`, `"script"`, `"variant"`,
/// `"currency"`, `"calendar"`, or `"dateTimeField"`.
///
/// Options:
/// - `style`: `"long"` (default), `"short"`, or `"narrow"`
/// - `fallback`: `"code"` (default) or `"none"` (returns error if not found)
/// - `languageDisplay`: `"dialect"` (default) or `"standard"` (for `type: "language"`)
///
/// **Note:** `calendar` and `dateTimeField` types return English-only display names
/// using static CLDR-based fallbacks. ICU4X does not yet expose locale-aware APIs
/// for these types. The `locale` argument is ignored for these types.
///
/// Examples:
/// - `intl.display_name("en", "language", "pt")` → `"Inglês"`
/// - `intl.display_name("US", "region", "en")` → `"United States"`
/// - `intl.display_name("US", "region", "en", {style: "short"})` → `"US"`
/// - `intl.display_name("Latn", "script", "en")` → `"Latin"`
/// - `intl.display_name("fonipa", "variant", "en")` → `"IPA Phonetics"`
#[derive(Clone)]
pub struct DisplayName {
    provider: Option<IntlProvider>,
}

impl DisplayName {
    pub fn new() -> Self {
        Self { provider: None }
    }

    pub fn with_provider(provider: IntlProvider) -> Self {
        Self {
            provider: Some(provider),
        }
    }
}

impl Default for DisplayName {
    fn default() -> Self {
        Self::new()
    }
}

impl Function<NonFunctionValue> for DisplayName {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let code = require_string(args, 0)?;
        let name_type = require_string(args, 1)?;
        let locale = optional_locale(args, 2)?;
        let opts = optional_object(args, 3)?;
        let dp = make_provider(&self.provider)?;

        let style = opt_str(opts, "style").unwrap_or("long");
        let fallback = opt_str(opts, "fallback").unwrap_or("code");
        let no_fallback = fallback == "none";

        let language_display = match opt_str(opts, "languageDisplay").unwrap_or("dialect") {
            "standard" => LanguageDisplay::Standard,
            _ => LanguageDisplay::Dialect,
        };
        let mut dn_opts = LanguageIdentifierDisplayNameOptions::default();
        dn_opts.language_display = Some(language_display);

        let formatted = match name_type {
            "language" => {
                let lang_id = code.parse().map_err(|e| {
                    IntlError::InvalidArgument {
                        message: format!("invalid language code: {e}"),
                    }
                    .into_vm_error()
                })?;
                // style: long → try_new_long_light, short → try_new_short_light, narrow → try_new_light
                let result = match style {
                    "short" => match dp.as_buffer_provider() {
                        Some(p) => {
                            LanguageIdentifierDisplayName::try_new_short_light_with_buffer_provider(
                                p,
                                locale.into(),
                                lang_id,
                                dn_opts,
                            )
                        }
                        None => LanguageIdentifierDisplayName::try_new_short_light(
                            locale.into(),
                            lang_id,
                            dn_opts,
                        ),
                    },
                    "narrow" => match dp.as_buffer_provider() {
                        Some(p) => {
                            LanguageIdentifierDisplayName::try_new_light_with_buffer_provider(
                                p,
                                locale.into(),
                                lang_id,
                                dn_opts,
                            )
                        }
                        None => LanguageIdentifierDisplayName::try_new_light(
                            locale.into(),
                            lang_id,
                            dn_opts,
                        ),
                    },
                    _ => match dp.as_buffer_provider() {
                        Some(p) => {
                            LanguageIdentifierDisplayName::try_new_long_light_with_buffer_provider(
                                p,
                                locale.into(),
                                lang_id,
                                dn_opts,
                            )
                        }
                        None => LanguageIdentifierDisplayName::try_new_long_light(
                            locale.into(),
                            lang_id,
                            dn_opts,
                        ),
                    },
                };
                match result {
                    Ok(display) => display.as_borrowed().write_to_string().into_owned(),
                    Err(e) if no_fallback => {
                        return Err(IntlError::Formatting {
                            message: e.to_string(),
                        }
                        .into_vm_error());
                    }
                    Err(_) => code.to_string(),
                }
            }
            "region" => {
                let region = code.parse().map_err(|e| {
                    IntlError::InvalidArgument {
                        message: format!("invalid region code: {e}"),
                    }
                    .into_vm_error()
                })?;
                // style: long → light, short → short_tiny, narrow → tiny
                let display = match style {
                    "short" => {
                        let r = match dp.as_buffer_provider() {
                            Some(p) => RegionDisplayName::try_new_short_tiny_with_buffer_provider(
                                p,
                                locale.into(),
                                region,
                            ),
                            None => RegionDisplayName::try_new_short_tiny(locale.into(), region),
                        };
                        r.map(|d| d.write_to_string().into_owned())
                    }
                    "narrow" => {
                        let r = match dp.as_buffer_provider() {
                            Some(p) => RegionDisplayName::try_new_tiny_with_buffer_provider(
                                p,
                                locale.into(),
                                region,
                            ),
                            None => RegionDisplayName::try_new_tiny(locale.into(), region),
                        };
                        r.map(|d| d.write_to_string().into_owned())
                    }
                    _ => {
                        let r = match dp.as_buffer_provider() {
                            Some(p) => RegionDisplayName::try_new_light_with_buffer_provider(
                                p,
                                locale.into(),
                                region,
                            ),
                            None => RegionDisplayName::try_new_light(locale.into(), region),
                        };
                        r.map(|d| d.write_to_string().into_owned())
                    }
                };
                match display {
                    Ok(name) => name,
                    Err(e) if no_fallback => {
                        return Err(IntlError::Formatting {
                            message: e.to_string(),
                        }
                        .into_vm_error());
                    }
                    Err(_) => code.to_string(),
                }
            }
            "script" => {
                let script = code.parse().map_err(|e| {
                    IntlError::InvalidArgument {
                        message: format!("invalid script code: {e}"),
                    }
                    .into_vm_error()
                })?;
                // style: long → heavy, short → short_heavy, narrow → light
                let display = match style {
                    "short" => {
                        let r = match dp.as_buffer_provider() {
                            Some(p) => ScriptDisplayName::try_new_short_heavy_with_buffer_provider(
                                p,
                                locale.into(),
                                script,
                            ),
                            None => ScriptDisplayName::try_new_short_heavy(locale.into(), script),
                        };
                        r.map(|d| d.write_to_string().into_owned())
                    }
                    "narrow" => {
                        let r = match dp.as_buffer_provider() {
                            Some(p) => ScriptDisplayName::try_new_light_with_buffer_provider(
                                p,
                                locale.into(),
                                script,
                            ),
                            None => ScriptDisplayName::try_new_light(locale.into(), script),
                        };
                        r.map(|d| d.write_to_string().into_owned())
                    }
                    _ => {
                        let r = match dp.as_buffer_provider() {
                            Some(p) => ScriptDisplayName::try_new_heavy_with_buffer_provider(
                                p,
                                locale.into(),
                                script,
                            ),
                            None => ScriptDisplayName::try_new_heavy(locale.into(), script),
                        };
                        r.map(|d| d.write_to_string().into_owned())
                    }
                };
                match display {
                    Ok(name) => name,
                    Err(e) if no_fallback => {
                        return Err(IntlError::Formatting {
                            message: e.to_string(),
                        }
                        .into_vm_error());
                    }
                    Err(_) => code.to_string(),
                }
            }
            "variant" => {
                let variant = code.parse().map_err(|e| {
                    IntlError::InvalidArgument {
                        message: format!("invalid variant code: {e}"),
                    }
                    .into_vm_error()
                })?;
                let result = match dp.as_buffer_provider() {
                    Some(p) => VariantDisplayName::try_new_heavy_with_buffer_provider(
                        p,
                        locale.into(),
                        variant,
                    ),
                    None => VariantDisplayName::try_new_heavy(locale.into(), variant),
                };
                match result {
                    Ok(display) => display.write_to_string().into_owned(),
                    Err(e) if no_fallback => {
                        return Err(IntlError::Formatting {
                            message: e.to_string(),
                        }
                        .into_vm_error());
                    }
                    Err(_) => code.to_string(),
                }
            }
            "currency" => {
                let code_upper = code.to_uppercase();
                let attrs = DataMarkerAttributes::try_from_str(&code_upper).map_err(|_| {
                    IntlError::InvalidArgument {
                        message: format!("invalid currency code: {code}"),
                    }
                    .into_vm_error()
                })?;
                let data_locale: DataLocale = locale.into();
                let response_opt = match dp.as_buffer_provider() {
                    Some(p) => DataProvider::<CurrencyDisplaynameV1>::load(
                        &p.as_deserializing(),
                        DataRequest {
                            id: DataIdentifierBorrowed::for_marker_attributes_and_locale(
                                attrs,
                                &data_locale,
                            ),
                            ..Default::default()
                        },
                    ),
                    None => DataProvider::<CurrencyDisplaynameV1>::load(
                        &icu::experimental::provider::Baked,
                        DataRequest {
                            id: DataIdentifierBorrowed::for_marker_attributes_and_locale(
                                attrs,
                                &data_locale,
                            ),
                            ..Default::default()
                        },
                    ),
                }
                .allow_identifier_not_found()
                .map_err(|e| {
                    IntlError::Formatting {
                        message: e.to_string(),
                    }
                    .into_vm_error()
                })?;
                match response_opt {
                    Some(response) => {
                        let display_name = response.payload.get();
                        display_name.to_string()
                    }
                    None if no_fallback => {
                        return Err(IntlError::Formatting {
                            message: format!("no display name for currency: {code}"),
                        }
                        .into_vm_error());
                    }
                    None => code.to_string(),
                }
            }
            "calendar" => {
                let name = calendar_display_name(code, style);
                match name {
                    Some(n) => n,
                    None if no_fallback => {
                        return Err(IntlError::Formatting {
                            message: format!("no display name for calendar: {code}"),
                        }
                        .into_vm_error());
                    }
                    None => code.to_string(),
                }
            }
            "dateTimeField" => {
                let name = datetime_field_display_name(code, style);
                match name {
                    Some(n) => n,
                    None if no_fallback => {
                        return Err(IntlError::Formatting {
                            message: format!("no display name for dateTimeField: {code}"),
                        }
                        .into_vm_error());
                    }
                    None => code.to_string(),
                }
            }
            _ => {
                return Err(IntlError::InvalidType {
                    type_name: name_type.to_string(),
                    valid: "language, region, script, variant, currency, calendar, dateTimeField"
                        .into(),
                }
                .into_vm_error());
            }
        };

        Ok(s(formatted))
    }
}

/// Returns a locale-aware display name for a calendar code.
/// Uses static CLDR-based English names as fallback (ICU4X does not yet expose calendar display name APIs).
fn calendar_display_name(code: &str, _style: &str) -> Option<String> {
    let name = match code {
        "buddhist" => "Buddhist Calendar",
        "chinese" => "Chinese Calendar",
        "coptic" => "Coptic Calendar",
        "dangi" => "Dangi Calendar",
        "ethioaa" => "Ethiopic Calendar, Amete Alem",
        "ethiopic" => "Ethiopic Calendar",
        "gregory" => "Gregorian Calendar",
        "hebrew" => "Hebrew Calendar",
        "indian" => "Indian National Calendar",
        "islamic" => "Islamic Calendar",
        "islamic-umalqura" => "Islamic Calendar, Umm al-Qura",
        "islamic-tbla" => {
            "Islamic Calendar, Tabular (leap years 2,5,7,10,13,16,18,21,24,26,29 - astronomical epoch)"
        }
        "islamic-civil" => {
            "Islamic Calendar, Tabular (leap years 2,5,7,10,13,16,18,21,24,26,29 - civil epoch)"
        }
        "islamic-rgsa" => "Islamic Calendar, Saudi Arabia sighting",
        "iso8601" => "ISO 8601 Calendar",
        "japanese" => "Japanese Calendar",
        "persian" => "Persian Calendar",
        "roc" => "Republic of China Calendar",
        _ => return None,
    };
    Some(name.to_string())
}

/// Returns a locale-aware display name for a dateTimeField code.
/// Uses static CLDR-based English names as fallback (ICU4X does not yet expose dateTimeField display name APIs).
fn datetime_field_display_name(code: &str, _style: &str) -> Option<String> {
    let name = match code {
        "era" => "era",
        "year" => "year",
        "quarter" => "quarter",
        "month" => "month",
        "weekOfYear" => "week of year",
        "weekOfMonth" => "week of month",
        "weekday" => "weekday",
        "day" => "day",
        "dayPeriod" => "dayperiod",
        "hour" => "hour",
        "minute" => "minute",
        "second" => "second",
        "timeZoneName" => "time zone name",
        _ => return None,
    };
    Some(name.to_string())
}