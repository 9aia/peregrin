use std::fmt;

use ecma402_traits::number_format::NumberFormatOptions;
use ecma402_traits::number_format::NumberFormat as _;
use ecma402_traits::relative_time::{
    RelativeTimeFormat as RelativeTimeFormatTrait, RelativeTimeFormatPart, RelativeTimeNumeric,
    RelativeTimeStyle, RelativeTimeUnit,
};
use fixed_decimal::{Decimal, FloatPrecision};
use icu::experimental::relativetime::options::Numeric;
use icu::experimental::relativetime::{RelativeTimeFormatter, RelativeTimeFormatterOptions};
use icu::locale::Locale;
use icu_provider::DataError;
use icu_provider_blob::BlobDataProvider;
use writeable::Writeable;

use crate::number_format::NumberFormat;
use icu4x_extra::relative_time::split_parts;

pub use ecma402_traits::relative_time::RelativeTimeOptions as Options;

/// ICU4X-backed ECMA-402 `Intl.RelativeTimeFormat`.
pub struct RelativeTimeFormat<'a> {
    formatter: RelativeTimeFormatter,
    number_format: NumberFormat<'a>,
}

/// Errors that can occur when constructing or using a `RelativeTimeFormat`.
#[derive(Debug)]
pub enum Error {
    InvalidLocale(String),
    Data(DataError),
    Formatting(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLocale(s) => write!(f, "invalid locale: {s}"),
            Self::Data(e) => write!(f, "data provider error: {e}"),
            Self::Formatting(s) => write!(f, "formatting error: {s}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<DataError> for Error {
    fn from(e: DataError) -> Self {
        Self::Data(e)
    }
}

impl From<crate::number_format::Error> for Error {
    fn from(e: crate::number_format::Error) -> Self {
        match e {
            crate::number_format::Error::Data(de) => Self::Data(de),
            crate::number_format::Error::InvalidLocale(s) => Self::InvalidLocale(s),
            _ => Self::Formatting(e.to_string()),
        }
    }
}

impl<'a> RelativeTimeFormat<'a> {
    fn parse_locale(locale: &str) -> Result<Locale, Error> {
        Locale::try_from_str(locale).map_err(|_| Error::InvalidLocale(locale.to_string()))
    }

    fn make_options(options: &Options) -> RelativeTimeFormatterOptions {
        let mut icu_options = RelativeTimeFormatterOptions::default();
        if options.numeric == RelativeTimeNumeric::Auto {
            icu_options.numeric = Numeric::Auto;
        }
        icu_options
    }

    fn make_formatter(
        locale: &Locale,
        options: &Options,
        provider: Option<&BlobDataProvider>,
    ) -> Result<RelativeTimeFormatter, Error> {
        macro_rules! try_construct {
            ($compiled:path, $blob:path) => {
                match provider {
                    Some(p) => $blob(p, locale.into(), Self::make_options(options)),
                    None => $compiled(locale.into(), Self::make_options(options)),
                }
            };
        }

        use RelativeTimeStyle::{Long, Narrow, Short};
        use RelativeTimeUnit::{Day, Hour, Minute, Month, Quarter, Second, Week, Year};

        let result = match (options.unit, options.style) {
            (Second, Long) => try_construct!(
                RelativeTimeFormatter::try_new_long_second,
                RelativeTimeFormatter::try_new_long_second_with_buffer_provider
            ),
            (Second, Short) => try_construct!(
                RelativeTimeFormatter::try_new_short_second,
                RelativeTimeFormatter::try_new_short_second_with_buffer_provider
            ),
            (Second, Narrow) => try_construct!(
                RelativeTimeFormatter::try_new_narrow_second,
                RelativeTimeFormatter::try_new_narrow_second_with_buffer_provider
            ),
            (Minute, Long) => try_construct!(
                RelativeTimeFormatter::try_new_long_minute,
                RelativeTimeFormatter::try_new_long_minute_with_buffer_provider
            ),
            (Minute, Short) => try_construct!(
                RelativeTimeFormatter::try_new_short_minute,
                RelativeTimeFormatter::try_new_short_minute_with_buffer_provider
            ),
            (Minute, Narrow) => try_construct!(
                RelativeTimeFormatter::try_new_narrow_minute,
                RelativeTimeFormatter::try_new_narrow_minute_with_buffer_provider
            ),
            (Hour, Long) => try_construct!(
                RelativeTimeFormatter::try_new_long_hour,
                RelativeTimeFormatter::try_new_long_hour_with_buffer_provider
            ),
            (Hour, Short) => try_construct!(
                RelativeTimeFormatter::try_new_short_hour,
                RelativeTimeFormatter::try_new_short_hour_with_buffer_provider
            ),
            (Hour, Narrow) => try_construct!(
                RelativeTimeFormatter::try_new_narrow_hour,
                RelativeTimeFormatter::try_new_narrow_hour_with_buffer_provider
            ),
            (Day, Long) => try_construct!(
                RelativeTimeFormatter::try_new_long_day,
                RelativeTimeFormatter::try_new_long_day_with_buffer_provider
            ),
            (Day, Short) => try_construct!(
                RelativeTimeFormatter::try_new_short_day,
                RelativeTimeFormatter::try_new_short_day_with_buffer_provider
            ),
            (Day, Narrow) => try_construct!(
                RelativeTimeFormatter::try_new_narrow_day,
                RelativeTimeFormatter::try_new_narrow_day_with_buffer_provider
            ),
            (Week, Long) => try_construct!(
                RelativeTimeFormatter::try_new_long_week,
                RelativeTimeFormatter::try_new_long_week_with_buffer_provider
            ),
            (Week, Short) => try_construct!(
                RelativeTimeFormatter::try_new_short_week,
                RelativeTimeFormatter::try_new_short_week_with_buffer_provider
            ),
            (Week, Narrow) => try_construct!(
                RelativeTimeFormatter::try_new_narrow_week,
                RelativeTimeFormatter::try_new_narrow_week_with_buffer_provider
            ),
            (Month, Long) => try_construct!(
                RelativeTimeFormatter::try_new_long_month,
                RelativeTimeFormatter::try_new_long_month_with_buffer_provider
            ),
            (Month, Short) => try_construct!(
                RelativeTimeFormatter::try_new_short_month,
                RelativeTimeFormatter::try_new_short_month_with_buffer_provider
            ),
            (Month, Narrow) => try_construct!(
                RelativeTimeFormatter::try_new_narrow_month,
                RelativeTimeFormatter::try_new_narrow_month_with_buffer_provider
            ),
            (Quarter, Long) => try_construct!(
                RelativeTimeFormatter::try_new_long_quarter,
                RelativeTimeFormatter::try_new_long_quarter_with_buffer_provider
            ),
            (Quarter, Short) => try_construct!(
                RelativeTimeFormatter::try_new_short_quarter,
                RelativeTimeFormatter::try_new_short_quarter_with_buffer_provider
            ),
            (Quarter, Narrow) => try_construct!(
                RelativeTimeFormatter::try_new_narrow_quarter,
                RelativeTimeFormatter::try_new_narrow_quarter_with_buffer_provider
            ),
            (Year, Long) => try_construct!(
                RelativeTimeFormatter::try_new_long_year,
                RelativeTimeFormatter::try_new_long_year_with_buffer_provider
            ),
            (Year, Short) => try_construct!(
                RelativeTimeFormatter::try_new_short_year,
                RelativeTimeFormatter::try_new_short_year_with_buffer_provider
            ),
            (Year, Narrow) => try_construct!(
                RelativeTimeFormatter::try_new_narrow_year,
                RelativeTimeFormatter::try_new_narrow_year_with_buffer_provider
            ),
        };

        result.map_err(Error::from)
    }

    /// Create a new `RelativeTimeFormat` for the given BCP 47 locale tag using compiled CLDR data.
    pub fn new(locale: &str, options: Options) -> Result<RelativeTimeFormat<'static>, Error> {
        let locale: Locale = Self::parse_locale(locale)?;
        let formatter = Self::make_formatter(&locale, &options, None)?;
        let locale_tag = locale.to_string();
        let number_format = NumberFormat::new(&locale_tag, NumberFormatOptions::default())?;
        Ok(RelativeTimeFormat {
            formatter,
            number_format,
        })
    }

    /// Create a new `RelativeTimeFormat` using an ICU4X blob data provider.
    pub fn with_blob(
        locale: &str,
        options: Options,
        provider: &'a BlobDataProvider,
    ) -> Result<Self, Error> {
        let locale: Locale = Self::parse_locale(locale)?;
        let formatter = Self::make_formatter(&locale, &options, Some(provider))?;
        let locale_tag = locale.to_string();
        let number_format =
            NumberFormat::with_blob(&locale_tag, NumberFormatOptions::default(), provider)?;
        Ok(Self {
            formatter,
            number_format,
        })
    }
}

impl<'a> RelativeTimeFormatTrait for RelativeTimeFormat<'a> {
    type Error = Error;

    fn format(&self, value: f64) -> Result<String, Self::Error> {
        let decimal = if value == 0.0 || value.is_nan() || value.is_infinite() {
            Decimal::from(0_i64)
        } else {
            Decimal::try_from_f64(value, FloatPrecision::RoundTrip).unwrap_or(Decimal::from(0_i64))
        };

        let formatted = self.formatter.format(decimal);
        Ok(formatted.write_to_string().into_owned())
    }

    fn format_to_parts(&self, value: f64) -> Result<Vec<RelativeTimeFormatPart>, Self::Error> {
        let formatted = self.format(value)?;
        let number_parts = self.number_format.format_to_parts(value)?;
        Ok(split_parts(&formatted, &number_parts))
    }
}
