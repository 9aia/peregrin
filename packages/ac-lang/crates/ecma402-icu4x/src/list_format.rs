use std::fmt;

use ecma402_traits::list_format::{ListFormat as ListFormatTrait, ListStyle, ListType};
use icu::list::{
    ListFormatter,
    options::{ListFormatterOptions, ListLength},
};
use icu::locale::Locale;
use icu_provider::DataError;
use icu_provider_blob::BlobDataProvider;

pub use ecma402_traits::list_format::ListFormatOptions as Options;

/// ICU4X-backed ECMA-402 `Intl.ListFormat`.
pub struct ListFormat {
    formatter: ListFormatter,
}

/// Errors that can occur when constructing or using a `ListFormat`.
#[derive(Debug)]
pub enum Error {
    InvalidLocale(String),
    Data(DataError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLocale(s) => write!(f, "invalid locale: {s}"),
            Self::Data(e) => write!(f, "data provider error: {e}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<DataError> for Error {
    fn from(e: DataError) -> Self {
        Self::Data(e)
    }
}

impl ListFormat {
    fn parse_locale(locale: &str) -> Result<Locale, Error> {
        Locale::try_from_str(locale).map_err(|_| Error::InvalidLocale(locale.to_string()))
    }

    fn make_options(options: &Options) -> ListFormatterOptions {
        let length = match options.style {
            ListStyle::Short => ListLength::Short,
            ListStyle::Narrow => ListLength::Narrow,
            _ => ListLength::Wide,
        };
        ListFormatterOptions::default().with_length(length)
    }

    /// Create a new `ListFormat` for the given BCP 47 locale tag using compiled CLDR data.
    pub fn new(locale: &str, options: Options) -> Result<Self, Error> {
        let locale: Locale = Self::parse_locale(locale)?;
        let icu_options = Self::make_options(&options);

        let formatter = match options.list_type {
            ListType::Disjunction => ListFormatter::try_new_or(locale.into(), icu_options),
            ListType::Unit => ListFormatter::try_new_unit(locale.into(), icu_options),
            _ => ListFormatter::try_new_and(locale.into(), icu_options),
        }?;

        Ok(Self { formatter })
    }

    /// Create a new `ListFormat` using an ICU4X blob data provider.
    pub fn with_blob(
        locale: &str,
        options: Options,
        provider: &BlobDataProvider,
    ) -> Result<Self, Error> {
        let locale: Locale = Self::parse_locale(locale)?;
        let icu_options = Self::make_options(&options);

        let formatter = match options.list_type {
            ListType::Disjunction => {
                ListFormatter::try_new_or_with_buffer_provider(provider, locale.into(), icu_options)
            }
            ListType::Unit => ListFormatter::try_new_unit_with_buffer_provider(
                provider,
                locale.into(),
                icu_options,
            ),
            _ => ListFormatter::try_new_and_with_buffer_provider(
                provider,
                locale.into(),
                icu_options,
            ),
        }?;

        Ok(Self { formatter })
    }
}

impl ListFormatTrait for ListFormat {
    type Error = Error;

    fn format(&self, items: &[&str]) -> Result<String, Self::Error> {
        if items.is_empty() {
            return Ok(String::new());
        }
        if items.len() == 1 {
            return Ok(items[0].to_string());
        }
        Ok(self.formatter.format_to_string(items.iter().copied()))
    }
}
