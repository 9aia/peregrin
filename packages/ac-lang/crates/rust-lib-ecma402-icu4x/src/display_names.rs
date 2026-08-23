use std::fmt;

use ecma402_traits::display_names::DisplayNames as DisplayNamesTrait;
use icu::experimental::dimension::provider::currency::displayname::CurrencyDisplaynameV1;
use icu::locale::names::{
    LanguageDisplay as IcuLanguageDisplay, LanguageIdentifierDisplayName,
    LanguageIdentifierDisplayNameOptions, RegionDisplayName, ScriptDisplayName, VariantDisplayName};
use icu::locale::{DataLocale, LanguageIdentifier, Locale};
use icu_provider::DataError;
use icu_provider::{
    DataIdentifierBorrowed, DataMarkerAttributes, DataProvider, DataRequest, ResultDataError,
    buf::AsDeserializingBufferProvider};
use icu_provider_blob::BlobDataProvider;
use writeable::Writeable;

pub use ecma402_traits::display_names::{
    DisplayNamesOptions as Options, Fallback, LanguageDisplay, Style, Type};

/// ICU4X-backed ECMA-402 `Intl.DisplayNames`.
pub struct DisplayNames<'p> {
    locale: Locale,
    provider: Option<&'p BlobDataProvider>}

/// Errors that can occur when constructing or using a `DisplayNames`.
#[derive(Debug)]
pub enum Error {
    InvalidLocale(String),
    InvalidCode { type_: Type, code: String },
    NotFound { type_: Type, code: String },
    Data(DataError)}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLocale(s) => write!(f, "invalid locale: {s}"),
            Self::InvalidCode { type_, code } => {
                write!(f, "invalid {} code: {code}", type_.as_str())
            }
            Self::NotFound { type_, code } => {
                write!(f, "no display name for {}: {code}", type_.as_str())
            }
            Self::Data(e) => write!(f, "data provider error: {e}")}
    }
}

impl std::error::Error for Error {}

impl From<DataError> for Error {
    fn from(e: DataError) -> Self {
        Self::Data(e)
    }
}

fn parse_locale(locale: &str) -> Result<Locale, Error> {
    Locale::try_from_str(locale).map_err(|_| Error::InvalidLocale(locale.to_string()))
}

fn map_language_display(display: LanguageDisplay) -> IcuLanguageDisplay {
    match display {
        LanguageDisplay::Dialect => IcuLanguageDisplay::Dialect,
        LanguageDisplay::Standard => IcuLanguageDisplay::Standard}
}

fn fallback_or_error(
    code: &str,
    _type_: Type,
    fallback: Fallback,
    _err: Error,
) -> Result<String, Error> {
    match fallback {
        Fallback::Code => Ok(code.to_string()),
        // ECMA-402 `fallback: "none"` returns an empty display name.
        Fallback::None => Ok(String::new())}
}

impl<'p> DisplayNames<'p> {
    /// Create a new `DisplayNames` for the given BCP 47 locale tag using compiled CLDR data.
    pub fn new(locale: &str, _options: Options) -> Result<Self, Error> {
        let locale = parse_locale(locale)?;
        Ok(Self {
            locale,
            provider: None})
    }

    /// Create a new `DisplayNames` using an ICU4X blob data provider.
    pub fn with_blob(
        locale: &str,
        _options: Options,
        provider: &'p BlobDataProvider,
    ) -> Result<Self, Error> {
        let locale = parse_locale(locale)?;
        Ok(Self {
            locale,
            provider: Some(provider)})
    }

    fn language_name(&self, code: &str, options: &Options) -> Result<String, Error> {
        let lang_id: LanguageIdentifier = code.parse().map_err(|_| Error::InvalidCode {
            type_: Type::Language,
            code: code.to_string()})?;

        let mut dn_opts = LanguageIdentifierDisplayNameOptions::default();
        dn_opts.language_display = Some(map_language_display(options.language_display));

        let result = match options.style {
            Style::Short => match self.provider {
                Some(p) => LanguageIdentifierDisplayName::try_new_short_light_with_buffer_provider(
                    p,
                    self.locale.clone().into(),
                    lang_id,
                    dn_opts,
                ),
                None => LanguageIdentifierDisplayName::try_new_short_light(
                    self.locale.clone().into(),
                    lang_id,
                    dn_opts,
                )},
            Style::Narrow => match self.provider {
                Some(p) => LanguageIdentifierDisplayName::try_new_light_with_buffer_provider(
                    p,
                    self.locale.clone().into(),
                    lang_id,
                    dn_opts,
                ),
                None => LanguageIdentifierDisplayName::try_new_light(
                    self.locale.clone().into(),
                    lang_id,
                    dn_opts,
                )},
            Style::Long => match self.provider {
                Some(p) => LanguageIdentifierDisplayName::try_new_long_light_with_buffer_provider(
                    p,
                    self.locale.clone().into(),
                    lang_id,
                    dn_opts,
                ),
                None => LanguageIdentifierDisplayName::try_new_long_light(
                    self.locale.clone().into(),
                    lang_id,
                    dn_opts,
                )}};

        match result {
            Ok(display) => Ok(display.as_borrowed().write_to_string().into_owned()),
            Err(e) => fallback_or_error(code, Type::Language, options.fallback, e.into())}
    }

    fn region_name(&self, code: &str, options: &Options) -> Result<String, Error> {
        let region = code.parse().map_err(|_| Error::InvalidCode {
            type_: Type::Region,
            code: code.to_string()})?;

        let result = match options.style {
            Style::Short => match self.provider {
                Some(p) => RegionDisplayName::try_new_short_tiny_with_buffer_provider(
                    p,
                    self.locale.clone().into(),
                    region,
                ),
                None => RegionDisplayName::try_new_short_tiny(self.locale.clone().into(), region)},
            Style::Narrow => match self.provider {
                Some(p) => RegionDisplayName::try_new_tiny_with_buffer_provider(
                    p,
                    self.locale.clone().into(),
                    region,
                ),
                None => RegionDisplayName::try_new_tiny(self.locale.clone().into(), region)},
            Style::Long => match self.provider {
                Some(p) => RegionDisplayName::try_new_light_with_buffer_provider(
                    p,
                    self.locale.clone().into(),
                    region,
                ),
                None => RegionDisplayName::try_new_light(self.locale.clone().into(), region)}};

        match result {
            Ok(display) => Ok(display.write_to_string().into_owned()),
            Err(e) => fallback_or_error(code, Type::Region, options.fallback, e.into())}
    }

    fn script_name(&self, code: &str, options: &Options) -> Result<String, Error> {
        let script = code.parse().map_err(|_| Error::InvalidCode {
            type_: Type::Script,
            code: code.to_string()})?;

        let result = match options.style {
            Style::Short => match self.provider {
                Some(p) => ScriptDisplayName::try_new_short_heavy_with_buffer_provider(
                    p,
                    self.locale.clone().into(),
                    script,
                ),
                None => ScriptDisplayName::try_new_short_heavy(self.locale.clone().into(), script)},
            Style::Narrow => match self.provider {
                Some(p) => ScriptDisplayName::try_new_light_with_buffer_provider(
                    p,
                    self.locale.clone().into(),
                    script,
                ),
                None => ScriptDisplayName::try_new_light(self.locale.clone().into(), script)},
            Style::Long => match self.provider {
                Some(p) => ScriptDisplayName::try_new_heavy_with_buffer_provider(
                    p,
                    self.locale.clone().into(),
                    script,
                ),
                None => ScriptDisplayName::try_new_heavy(self.locale.clone().into(), script)}};

        match result {
            Ok(display) => Ok(display.write_to_string().into_owned()),
            Err(e) => fallback_or_error(code, Type::Script, options.fallback, e.into())}
    }

    fn variant_name(&self, code: &str, options: &Options) -> Result<String, Error> {
        let variant = code.parse().map_err(|_| Error::InvalidCode {
            type_: Type::Variant,
            code: code.to_string()})?;

        let result = match self.provider {
            Some(p) => VariantDisplayName::try_new_heavy_with_buffer_provider(
                p,
                self.locale.clone().into(),
                variant,
            ),
            None => VariantDisplayName::try_new_heavy(self.locale.clone().into(), variant)};

        match result {
            Ok(display) => Ok(display.write_to_string().into_owned()),
            Err(e) => fallback_or_error(code, Type::Variant, options.fallback, e.into())}
    }

    fn currency_name(&self, code: &str, options: &Options) -> Result<String, Error> {
        let code_upper = code.to_uppercase();
        let attrs =
            DataMarkerAttributes::try_from_str(&code_upper).map_err(|_| Error::InvalidCode {
                type_: Type::Currency,
                code: code.to_string()})?;

        let data_locale: DataLocale = self.locale.clone().into();
        let response_opt = match self.provider {
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
            )}
        .allow_identifier_not_found()
        .map_err(Error::Data)?;

        match response_opt {
            Some(response) => Ok(response.payload.get().to_string()),
            None => fallback_or_error(
                code,
                Type::Currency,
                options.fallback,
                Error::NotFound {
                    type_: Type::Currency,
                    code: code.to_string()},
            )}
    }

    fn calendar_name(&self, code: &str, options: &Options) -> Result<String, Error> {
        match calendar_display_name(code) {
            Some(name) => Ok(name),
            None => fallback_or_error(
                code,
                Type::Calendar,
                options.fallback,
                Error::NotFound {
                    type_: Type::Calendar,
                    code: code.to_string()},
            )}
    }

    fn datetime_field_name(&self, code: &str, options: &Options) -> Result<String, Error> {
        match datetime_field_display_name(code) {
            Some(name) => Ok(name),
            None => fallback_or_error(
                code,
                Type::DateTimeField,
                options.fallback,
                Error::NotFound {
                    type_: Type::DateTimeField,
                    code: code.to_string()},
            )}
    }
}

impl<'p> DisplayNamesTrait for DisplayNames<'p> {
    type Error = Error;

    fn of(&self, code: &str, type_: Type, options: Options) -> Result<String, Self::Error> {
        match type_ {
            Type::Language => self.language_name(code, &options),
            Type::Region => self.region_name(code, &options),
            Type::Script => self.script_name(code, &options),
            Type::Variant => self.variant_name(code, &options),
            Type::Currency => self.currency_name(code, &options),
            Type::Calendar => self.calendar_name(code, &options),
            Type::DateTimeField => self.datetime_field_name(code, &options)}
    }
}

/// Returns an English-only display name for a calendar code.
///
/// ICU4X 2 (including `icu::experimental`) does not yet expose a localized API
/// for calendar display names, so this table is intentionally English-only.
/// The `style` and `language_display` options are not applicable here.
fn calendar_display_name(code: &str) -> Option<String> {
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
        _ => return None};
    Some(name.to_string())
}

/// Returns an English-only display name for a dateTimeField code.
///
/// ICU4X 2 (including `icu::experimental`) does not yet expose a localized API
/// for date/time field display names, so this table is intentionally English-only.
/// The `style` and `language_display` options are not applicable here.
fn datetime_field_display_name(code: &str) -> Option<String> {
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
        _ => return None};
    Some(name.to_string())
}

#[cfg(test)]
#[path = "display_names_tests.rs"]
mod display_names_tests;
