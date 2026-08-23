use std::fmt::{self, Write};

use ecma402_traits::number_format::NumberFormat as NumberFormatTrait;

pub use ecma402_traits::number_format::{
    CompactDisplay, CurrencyDisplay, CurrencySign, Notation, NumberFormatOptions, NumberFormatPart,
    NumberFormatPartType, NumberFormatStyle, RoundingMode, SignDisplay, UnitDisplay, UseGrouping,
};
use fixed_decimal::{
    Decimal, FloatPrecision, SignDisplay as FixedSignDisplay, SignedRoundingMode,
    UnsignedRoundingMode,
};
use icu::decimal::options::{CompactDecimalFormatterOptions, DecimalFormatterOptions, GroupingStrategy};
use icu::decimal::{CompactDecimalFormatter, DecimalFormatter};
use icu::experimental::dimension::currency::CurrencyType;
use icu::experimental::dimension::currency::formatter::CurrencyFormatter;
use icu::experimental::dimension::currency::options::{CurrencyFormatterOptions, CurrencyUsage};
use icu::experimental::dimension::percent::formatter::PercentFormatter;
use icu::experimental::dimension::units::categorized_formatter::{
    CategorizedFormatter, CategorizedUnitsFormatterPreferences,
};
use icu::experimental::dimension::units::options::{UnitsFormatterOptions, Width};
use icu::experimental::measure::category::{Area, Duration, Length, Mass, Volume};
use icu::experimental::measure::measureunit::MeasureUnit;
use icu::locale::Locale;
use icu_provider::DataError;
use icu_provider_blob::BlobDataProvider;
use writeable::{Part, PartsWrite, Writeable};

/// ICU4X-backed ECMA-402 `Intl.NumberFormat`.
pub struct NumberFormat<'a> {
    locale: Locale,
    options: NumberFormatOptions,
    provider: DataProvider<'a>,
}

enum DataProvider<'a> {
    Compiled,
    Blob(&'a BlobDataProvider),
}

impl<'a> DataProvider<'a> {
    fn as_buffer_provider(&self) -> Option<&'a BlobDataProvider> {
        match self {
            Self::Compiled => None,
            Self::Blob(p) => Some(p),
        }
    }
}

/// Errors that can occur when constructing or using a `NumberFormat`.
#[derive(Debug)]
pub enum Error {
    InvalidLocale(String),
    Data(DataError),
    InvalidArgument(String),
    InvalidCurrency(String),
    InvalidUnit(String),
    Formatting(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLocale(s) => write!(f, "invalid locale: {s}"),
            Self::Data(e) => write!(f, "data provider error: {e}"),
            Self::InvalidArgument(s) => write!(f, "invalid argument: {s}"),
            Self::InvalidCurrency(s) => write!(f, "invalid currency code: {s}"),
            Self::InvalidUnit(s) => write!(f, "invalid unit: {s}"),
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

impl<'a> NumberFormat<'a> {
    /// Create a new `NumberFormat` for the given BCP 47 locale tag using compiled CLDR data.
    pub fn new(locale: &str, options: NumberFormatOptions) -> Result<NumberFormat<'static>, Error> {
        let mut locale = parse_locale(locale)?;
        apply_numbering_system(&mut locale, &options);

        Ok(NumberFormat {
            locale,
            options,
            provider: DataProvider::Compiled,
        })
    }

    /// Create a new `NumberFormat` using an ICU4X blob data provider.
    pub fn with_blob(
        locale: &str,
        options: NumberFormatOptions,
        provider: &'a BlobDataProvider,
    ) -> Result<Self, Error> {
        let mut locale = parse_locale(locale)?;
        apply_numbering_system(&mut locale, &options);

        Ok(NumberFormat {
            locale,
            options,
            provider: DataProvider::Blob(provider),
        })
    }

    fn decimal_formatter(&self) -> Result<DecimalFormatter, Error> {
        let formatter_opts = build_formatter_options(&self.options);
        match self.provider.as_buffer_provider() {
            Some(p) => DecimalFormatter::try_new_with_buffer_provider(
                p,
                self.locale.clone().into(),
                formatter_opts,
            ),
            None => DecimalFormatter::try_new(self.locale.clone().into(), formatter_opts),
        }
        .map_err(Error::Data)
    }

    fn format_prepared_decimal(&self, decimal: &Decimal) -> Result<String, Error> {
        let formatter = self.decimal_formatter()?;
        let formatted = formatter.format(decimal);
        Ok(formatted.write_to_string().into_owned())
    }

    fn format_prepared_decimal_to_parts(
        &self,
        decimal: &Decimal,
        sink: &mut PartsCollector,
    ) -> Result<(), Error> {
        let formatter = self.decimal_formatter()?;
        let formatted = formatter.format(decimal);
        formatted
            .write_to_parts(sink)
            .map_err(|e| Error::Formatting(e.to_string()))
    }

    fn format_exponent(&self, exponent: i32) -> Result<String, Error> {
        let formatter = self.decimal_formatter()?;
        let mut decimal = Decimal::from(exponent);
        decimal.apply_sign_display(FixedSignDisplay::ExceptZero);
        Ok(formatter.format(&decimal).write_to_string().into_owned())
    }

    fn format_exponent_to_parts(
        &self,
        exponent: i32,
        sink: &mut PartsCollector,
    ) -> Result<(), Error> {
        let formatter = self.decimal_formatter()?;
        let mut decimal = Decimal::from(exponent);
        decimal.apply_sign_display(FixedSignDisplay::ExceptZero);
        let formatted = formatter.format(&decimal);
        formatted
            .write_to_parts(sink)
            .map_err(|e| Error::Formatting(e.to_string()))
    }

    fn format_standard(&self, n: f64) -> Result<String, Error> {
        let mut decimal = decimal_from_f64(n);
        apply_digit_options(&mut decimal, &self.options);
        self.format_prepared_decimal(&decimal)
    }

    fn format_compact(&self, n: f64) -> Result<String, Error> {
        let compact_opts: CompactDecimalFormatterOptions =
            build_formatter_options(&self.options).into();
        let formatter = if self.options.compact_display == CompactDisplay::Long {
            match self.provider.as_buffer_provider() {
                Some(p) => CompactDecimalFormatter::try_new_long_with_buffer_provider(
                    p,
                    self.locale.clone().into(),
                    compact_opts,
                ),
                None => CompactDecimalFormatter::try_new_long(
                    self.locale.clone().into(),
                    compact_opts,
                ),
            }
        } else {
            match self.provider.as_buffer_provider() {
                Some(p) => CompactDecimalFormatter::try_new_short_with_buffer_provider(
                    p,
                    self.locale.clone().into(),
                    compact_opts,
                ),
                None => CompactDecimalFormatter::try_new_short(
                    self.locale.clone().into(),
                    compact_opts,
                ),
            }
        }
        .map_err(Error::Data)?;

        let mut decimal = decimal_from_f64(n);
        apply_sign_display(&mut decimal, &self.options);
        let formatted = formatter.format(&decimal);
        Ok(formatted.write_to_string().into_owned())
    }

    fn format_scientific(&self, n: f64) -> Result<String, Error> {
        if n == 0.0 || n.is_nan() || n.is_infinite() {
            return self.format_standard(n);
        }
        let mut decimal = decimal_from_f64(n);
        let exponent = decimal.nonzero_magnitude_start() as i32;
        decimal.multiply_pow10(-exponent as i16);
        apply_fraction_digits(&mut decimal, &self.options);
        apply_significant_digits(&mut decimal, &self.options);
        apply_sign_display(&mut decimal, &self.options);
        let mantissa_str = self.format_prepared_decimal(&decimal)?;
        let exp_str = self.format_exponent(exponent)?;
        Ok(format!("{mantissa_str}×10{exp_str}"))
    }

    fn format_engineering(&self, n: f64) -> Result<String, Error> {
        if n == 0.0 || n.is_nan() || n.is_infinite() {
            return self.format_standard(n);
        }
        let mut decimal = decimal_from_f64(n);
        let raw_exp = decimal.nonzero_magnitude_start() as i32;
        let exponent = (raw_exp / 3) * 3;
        decimal.multiply_pow10(-exponent as i16);
        apply_fraction_digits(&mut decimal, &self.options);
        apply_significant_digits(&mut decimal, &self.options);
        apply_sign_display(&mut decimal, &self.options);
        let mantissa_str = self.format_prepared_decimal(&decimal)?;
        let exp_str = self.format_exponent(exponent)?;
        Ok(format!("{mantissa_str}×10{exp_str}"))
    }

    fn format_percent(&self, n: f64) -> Result<String, Error> {
        let formatter = match self.provider.as_buffer_provider() {
            Some(p) => PercentFormatter::try_new_with_buffer_provider(
                p,
                self.locale.clone().into(),
                Default::default(),
            ),
            None => PercentFormatter::try_new(self.locale.clone().into(), Default::default()),
        }
        .map_err(Error::Data)?;

        let mut decimal = decimal_from_f64(n);
        decimal.multiply_pow10(2);
        decimal.trim_start();
        apply_digit_options(&mut decimal, &self.options);
        let formatted = formatter.format(&decimal);
        Ok(formatted.write_to_string().into_owned())
    }

    fn format_currency(&self, n: f64) -> Result<String, Error> {
        let currency = self.options.currency.as_deref().unwrap_or("");
        let display = self.options.currency_display;

        let currency_code = CurrencyType::try_from_str(currency)
            .map_err(|_| Error::InvalidCurrency(currency.to_string()))?;

        let usage = if self.options.currency_sign == CurrencySign::Accounting {
            CurrencyUsage::Accounting
        } else {
            CurrencyUsage::Standard
        };
        let mut currency_opts = CurrencyFormatterOptions::default();
        currency_opts.usage = usage;

        let formatter = match (display, self.provider.as_buffer_provider()) {
            (CurrencyDisplay::Code, Some(p)) => {
                CurrencyFormatter::try_new_code_with_buffer_provider(
                    p,
                    self.locale.clone().into(),
                    currency_code,
                    currency_opts,
                )
            }
            (CurrencyDisplay::Code, None) => CurrencyFormatter::try_new_code(
                self.locale.clone().into(),
                currency_code,
                currency_opts,
            ),
            (CurrencyDisplay::Name, Some(p)) => {
                CurrencyFormatter::try_new_name_with_buffer_provider(
                    p,
                    self.locale.clone().into(),
                    currency_code,
                )
            }
            (CurrencyDisplay::Name, None) => {
                CurrencyFormatter::try_new_name(self.locale.clone().into(), currency_code)
            }
            (CurrencyDisplay::NarrowSymbol, Some(p)) => {
                CurrencyFormatter::try_new_symbol_narrow_with_buffer_provider(
                    p,
                    self.locale.clone().into(),
                    currency_code,
                    currency_opts,
                )
            }
            (CurrencyDisplay::NarrowSymbol, None) => CurrencyFormatter::try_new_symbol_narrow(
                self.locale.clone().into(),
                currency_code,
                currency_opts,
            ),
            (CurrencyDisplay::Symbol, Some(p)) => {
                CurrencyFormatter::try_new_symbol_with_buffer_provider(
                    p,
                    self.locale.clone().into(),
                    currency_code,
                    currency_opts,
                )
            }
            (CurrencyDisplay::Symbol, None) => CurrencyFormatter::try_new_symbol(
                self.locale.clone().into(),
                currency_code,
                currency_opts,
            ),
        }
        .map_err(Error::Data)?;

        let mut decimal = decimal_from_f64(n);
        apply_fraction_digits(&mut decimal, &self.options);
        apply_sign_display(&mut decimal, &self.options);
        let formatted = formatter.format_fixed_decimal(&decimal);
        Ok(formatted.write_to_string().into_owned())
    }

    fn format_unit(&self, n: f64) -> Result<String, Error> {
        let unit_str = self.options.unit.as_deref().unwrap_or("");
        if unit_str.is_empty() {
            return Err(Error::InvalidArgument(
                "unit option required for unit style".into(),
            ));
        }

        let _ = MeasureUnit::try_from_str(unit_str)
            .map_err(|_| Error::InvalidUnit(unit_str.to_string()))?;

        let width = match self.options.unit_display {
            UnitDisplay::Long => Width::Long,
            UnitDisplay::Narrow => Width::Narrow,
            UnitDisplay::Short => Width::Short,
        };
        let mut unit_opts = UnitsFormatterOptions::default();
        unit_opts.width = width;

        let mut decimal = decimal_from_f64(n);
        apply_digit_options(&mut decimal, &self.options);

        let id: &'static str = Box::leak(unit_str.to_string().into_boxed_str());
        let prefs: CategorizedUnitsFormatterPreferences = self.locale.clone().into();

        macro_rules! try_unit {
            ($constructor:expr, $category:ident) => {{
                let mut categorized = $constructor;
                categorized.unit.id = Some(id);
                if let Ok(fmt) = CategorizedFormatter::<$category>::try_new_outlier(
                    prefs,
                    categorized,
                    unit_opts,
                ) {
                    let formatted = fmt.format_fixed_decimal(&decimal);
                    return Ok(formatted.write_to_string().into_owned());
                }
            }};
        }

        try_unit!(Length::meter(), Length);
        try_unit!(Mass::gram(), Mass);
        try_unit!(Area::square_meter(), Area);
        try_unit!(Volume::cubic_meter(), Volume);
        try_unit!(Duration::second(), Duration);

        let number = self.format_prepared_decimal(&decimal)?;
        let separator = match self.options.unit_display {
            UnitDisplay::Narrow => "",
            _ => " ",
        };
        Ok(format!("{number}{separator}{unit_str}"))
    }

    fn format_standard_to_parts(&self, n: f64, sink: &mut PartsCollector) -> Result<(), Error> {
        let mut decimal = decimal_from_f64(n);
        apply_digit_options(&mut decimal, &self.options);
        self.format_prepared_decimal_to_parts(&decimal, sink)
    }

    fn format_compact_to_parts(&self, n: f64, sink: &mut PartsCollector) -> Result<(), Error> {
        let compact_opts: CompactDecimalFormatterOptions =
            build_formatter_options(&self.options).into();
        let formatter = if self.options.compact_display == CompactDisplay::Long {
            match self.provider.as_buffer_provider() {
                Some(p) => CompactDecimalFormatter::try_new_long_with_buffer_provider(
                    p,
                    self.locale.clone().into(),
                    compact_opts,
                ),
                None => CompactDecimalFormatter::try_new_long(
                    self.locale.clone().into(),
                    compact_opts,
                ),
            }
        } else {
            match self.provider.as_buffer_provider() {
                Some(p) => CompactDecimalFormatter::try_new_short_with_buffer_provider(
                    p,
                    self.locale.clone().into(),
                    compact_opts,
                ),
                None => CompactDecimalFormatter::try_new_short(
                    self.locale.clone().into(),
                    compact_opts,
                ),
            }
        }
        .map_err(Error::Data)?;

        let mut decimal = decimal_from_f64(n);
        apply_sign_display(&mut decimal, &self.options);
        let formatted = formatter.format(&decimal);
        sink.with_part(
            Part {
                category: "decimal",
                value: "compact",
            },
            |w| formatted.write_to_parts(w),
        )
        .map_err(|e| Error::Formatting(e.to_string()))
    }

    fn format_scientific_to_parts(&self, n: f64, sink: &mut PartsCollector) -> Result<(), Error> {
        if n == 0.0 || n.is_nan() || n.is_infinite() {
            return self.format_standard_to_parts(n, sink);
        }
        let mut decimal = decimal_from_f64(n);
        let exponent = decimal.nonzero_magnitude_start() as i32;
        decimal.multiply_pow10(-exponent as i16);
        apply_fraction_digits(&mut decimal, &self.options);
        apply_significant_digits(&mut decimal, &self.options);
        apply_sign_display(&mut decimal, &self.options);
        self.format_prepared_decimal_to_parts(&decimal, sink)?;

        sink.with_part(
            Part {
                category: "decimal",
                value: "exponentSeparator",
            },
            |w| w.write_str("×10"),
        )
        .map_err(|e| Error::Formatting(e.to_string()))?;

        self.format_exponent_to_parts(exponent, sink)
    }

    fn format_engineering_to_parts(&self, n: f64, sink: &mut PartsCollector) -> Result<(), Error> {
        if n == 0.0 || n.is_nan() || n.is_infinite() {
            return self.format_standard_to_parts(n, sink);
        }
        let mut decimal = decimal_from_f64(n);
        let raw_exp = decimal.nonzero_magnitude_start() as i32;
        let exponent = (raw_exp / 3) * 3;
        decimal.multiply_pow10(-exponent as i16);
        apply_fraction_digits(&mut decimal, &self.options);
        apply_significant_digits(&mut decimal, &self.options);
        apply_sign_display(&mut decimal, &self.options);
        self.format_prepared_decimal_to_parts(&decimal, sink)?;

        sink.with_part(
            Part {
                category: "decimal",
                value: "exponentSeparator",
            },
            |w| w.write_str("×10"),
        )
        .map_err(|e| Error::Formatting(e.to_string()))?;

        self.format_exponent_to_parts(exponent, sink)
    }

    fn format_percent_to_parts(&self, n: f64, sink: &mut PartsCollector) -> Result<(), Error> {
        let formatter = match self.provider.as_buffer_provider() {
            Some(p) => PercentFormatter::try_new_with_buffer_provider(
                p,
                self.locale.clone().into(),
                Default::default(),
            ),
            None => PercentFormatter::try_new(self.locale.clone().into(), Default::default()),
        }
        .map_err(Error::Data)?;

        let mut decimal = decimal_from_f64(n);
        decimal.multiply_pow10(2);
        decimal.trim_start();
        apply_digit_options(&mut decimal, &self.options);
        let formatted = formatter.format(&decimal);
        formatted
            .write_to_parts(sink)
            .map_err(|e| Error::Formatting(e.to_string()))
    }

    fn format_currency_to_parts(&self, n: f64, sink: &mut PartsCollector) -> Result<(), Error> {
        let currency = self.options.currency.as_deref().unwrap_or("");
        let display = self.options.currency_display;

        let currency_code = CurrencyType::try_from_str(currency)
            .map_err(|_| Error::InvalidCurrency(currency.to_string()))?;

        let usage = if self.options.currency_sign == CurrencySign::Accounting {
            CurrencyUsage::Accounting
        } else {
            CurrencyUsage::Standard
        };
        let mut currency_opts = CurrencyFormatterOptions::default();
        currency_opts.usage = usage;

        let formatter = match (display, self.provider.as_buffer_provider()) {
            (CurrencyDisplay::Code, Some(p)) => {
                CurrencyFormatter::try_new_code_with_buffer_provider(
                    p,
                    self.locale.clone().into(),
                    currency_code,
                    currency_opts,
                )
            }
            (CurrencyDisplay::Code, None) => CurrencyFormatter::try_new_code(
                self.locale.clone().into(),
                currency_code,
                currency_opts,
            ),
            (CurrencyDisplay::Name, Some(p)) => {
                CurrencyFormatter::try_new_name_with_buffer_provider(
                    p,
                    self.locale.clone().into(),
                    currency_code,
                )
            }
            (CurrencyDisplay::Name, None) => {
                CurrencyFormatter::try_new_name(self.locale.clone().into(), currency_code)
            }
            (CurrencyDisplay::NarrowSymbol, Some(p)) => {
                CurrencyFormatter::try_new_symbol_narrow_with_buffer_provider(
                    p,
                    self.locale.clone().into(),
                    currency_code,
                    currency_opts,
                )
            }
            (CurrencyDisplay::NarrowSymbol, None) => CurrencyFormatter::try_new_symbol_narrow(
                self.locale.clone().into(),
                currency_code,
                currency_opts,
            ),
            (CurrencyDisplay::Symbol, Some(p)) => {
                CurrencyFormatter::try_new_symbol_with_buffer_provider(
                    p,
                    self.locale.clone().into(),
                    currency_code,
                    currency_opts,
                )
            }
            (CurrencyDisplay::Symbol, None) => CurrencyFormatter::try_new_symbol(
                self.locale.clone().into(),
                currency_code,
                currency_opts,
            ),
        }
        .map_err(Error::Data)?;

        let mut decimal = decimal_from_f64(n);
        apply_fraction_digits(&mut decimal, &self.options);
        apply_sign_display(&mut decimal, &self.options);
        let formatted = formatter.format_fixed_decimal(&decimal);
        formatted
            .write_to_parts(sink)
            .map_err(|e| Error::Formatting(e.to_string()))
    }

    fn format_unit_to_parts(&self, n: f64, sink: &mut PartsCollector) -> Result<(), Error> {
        let unit_str = self.options.unit.as_deref().unwrap_or("");
        if unit_str.is_empty() {
            return Err(Error::InvalidArgument(
                "unit option required for unit style".into(),
            ));
        }

        let _ = MeasureUnit::try_from_str(unit_str)
            .map_err(|_| Error::InvalidUnit(unit_str.to_string()))?;

        let width = match self.options.unit_display {
            UnitDisplay::Long => Width::Long,
            UnitDisplay::Narrow => Width::Narrow,
            UnitDisplay::Short => Width::Short,
        };
        let mut unit_opts = UnitsFormatterOptions::default();
        unit_opts.width = width;

        let mut decimal = decimal_from_f64(n);
        apply_digit_options(&mut decimal, &self.options);

        let id: &'static str = Box::leak(unit_str.to_string().into_boxed_str());
        let prefs: CategorizedUnitsFormatterPreferences = self.locale.clone().into();

        macro_rules! try_unit_parts {
            ($constructor:expr, $category:ident) => {{
                let mut categorized = $constructor;
                categorized.unit.id = Some(id);
                if let Ok(fmt) = CategorizedFormatter::<$category>::try_new_outlier(
                    prefs,
                    categorized,
                    unit_opts,
                ) {
                    let formatted = fmt.format_fixed_decimal(&decimal);
                    return formatted
                        .write_to_parts(sink)
                        .map_err(|e| Error::Formatting(e.to_string()));
                }
            }};
        }

        try_unit_parts!(Length::meter(), Length);
        try_unit_parts!(Mass::gram(), Mass);
        try_unit_parts!(Area::square_meter(), Area);
        try_unit_parts!(Volume::cubic_meter(), Volume);
        try_unit_parts!(Duration::second(), Duration);

        self.format_prepared_decimal_to_parts(&decimal, sink)?;
        if self.options.unit_display != UnitDisplay::Narrow {
            sink.with_part(
                Part {
                    category: "decimal",
                    value: "literal",
                },
                |w| w.write_str(" "),
            )
            .map_err(|e| Error::Formatting(e.to_string()))?;
        }
        sink.with_part(
            Part {
                category: "decimal",
                value: "unit",
            },
            |w| w.write_str(unit_str),
        )
        .map_err(|e| Error::Formatting(e.to_string()))
    }
}

impl<'a> NumberFormatTrait for NumberFormat<'a> {
    type Error = Error;

    fn format(&self, value: f64) -> Result<String, Self::Error> {
        match self.options.style {
            NumberFormatStyle::Percent => self.format_percent(value),
            NumberFormatStyle::Unit => self.format_unit(value),
            NumberFormatStyle::Currency => {
                if self.options.currency.is_none() {
                    return Err(Error::InvalidArgument(
                        "currency option required for currency style".into(),
                    ));
                }
                self.format_currency(value)
            }
            NumberFormatStyle::Decimal => match self.options.notation {
                Notation::Compact => self.format_compact(value),
                Notation::Scientific => self.format_scientific(value),
                Notation::Engineering => self.format_engineering(value),
                Notation::Standard => self.format_standard(value),
            },
        }
    }

    fn format_to_parts(&self, value: f64) -> Result<Vec<NumberFormatPart>, Self::Error> {
        let mut sink = PartsCollector::new();
        match self.options.style {
            NumberFormatStyle::Percent => self.format_percent_to_parts(value, &mut sink)?,
            NumberFormatStyle::Unit => self.format_unit_to_parts(value, &mut sink)?,
            NumberFormatStyle::Currency => {
                if self.options.currency.is_none() {
                    return Err(Error::InvalidArgument(
                        "currency option required for currency style".into(),
                    ));
                }
                self.format_currency_to_parts(value, &mut sink)?;
            }
            NumberFormatStyle::Decimal => match self.options.notation {
                Notation::Compact => self.format_compact_to_parts(value, &mut sink)?,
                Notation::Scientific => self.format_scientific_to_parts(value, &mut sink)?,
                Notation::Engineering => self.format_engineering_to_parts(value, &mut sink)?,
                Notation::Standard => self.format_standard_to_parts(value, &mut sink)?,
            },
        }
        Ok(sink.into_parts())
    }
}

fn parse_locale(locale: &str) -> Result<Locale, Error> {
    Locale::try_from_str(locale).map_err(|_| Error::InvalidLocale(locale.to_string()))
}

fn apply_numbering_system(locale: &mut Locale, options: &NumberFormatOptions) {
    if let Some(numbering_system) = &options.numbering_system
        && !numbering_system.is_empty()
        && let Ok(value) = icu::locale::extensions::unicode::Value::try_from_str(numbering_system)
    {
        locale
            .extensions
            .unicode
            .keywords
            .set(icu::locale::extensions::unicode::key!("nu"), value);
    }
}

fn build_formatter_options(options: &NumberFormatOptions) -> DecimalFormatterOptions {
    let mut opts = DecimalFormatterOptions::default();
    opts.grouping_strategy = Some(match options.use_grouping {
        UseGrouping::Min2 => GroupingStrategy::Min2,
        UseGrouping::Auto => GroupingStrategy::Auto,
        UseGrouping::Always => GroupingStrategy::Always,
        UseGrouping::False | UseGrouping::Never => GroupingStrategy::Never,
        UseGrouping::True => GroupingStrategy::Auto,
    });
    opts
}

fn decimal_from_f64(n: f64) -> Decimal {
    if n == 0.0 || n.is_nan() || n.is_infinite() {
        return Decimal::from(0_i64);
    }
    Decimal::try_from_f64(n, FloatPrecision::RoundTrip).unwrap_or(Decimal::from(0_i64))
}

fn parse_rounding_mode(options: &NumberFormatOptions) -> SignedRoundingMode {
    match options.rounding_mode {
        RoundingMode::Ceil => SignedRoundingMode::Ceil,
        RoundingMode::Floor => SignedRoundingMode::Floor,
        RoundingMode::Expand => SignedRoundingMode::Unsigned(UnsignedRoundingMode::Expand),
        RoundingMode::Trunc => SignedRoundingMode::Unsigned(UnsignedRoundingMode::Trunc),
        RoundingMode::HalfCeil => SignedRoundingMode::HalfCeil,
        RoundingMode::HalfExpand => SignedRoundingMode::Unsigned(UnsignedRoundingMode::HalfExpand),
        RoundingMode::HalfFloor => SignedRoundingMode::HalfFloor,
        RoundingMode::HalfTrunc => SignedRoundingMode::Unsigned(UnsignedRoundingMode::HalfTrunc),
        RoundingMode::HalfEven => SignedRoundingMode::Unsigned(UnsignedRoundingMode::HalfEven),
    }
}

fn apply_minimum_integer_digits(decimal: &mut Decimal, options: &NumberFormatOptions) {
    if let Some(n) = options.minimum_integer_digits
        && n > 0
    {
        decimal.pad_start(i16::try_from(n).unwrap_or(0));
    }
}

fn apply_fraction_digits(decimal: &mut Decimal, options: &NumberFormatOptions) {
    let rounding_mode = parse_rounding_mode(options);
    if let Some(min) = options.minimum_fraction_digits {
        decimal.pad_end(-i16::try_from(min).unwrap_or(0));
    }
    if let Some(max) = options.maximum_fraction_digits {
        let max = i16::try_from(max).unwrap_or(0);
        decimal.round_with_mode(-max, rounding_mode);
        if let Some(min) = options.minimum_fraction_digits {
            decimal.pad_end(-i16::try_from(min).unwrap_or(0));
        }
    }
}

fn apply_significant_digits(decimal: &mut Decimal, options: &NumberFormatOptions) {
    let rounding_mode = parse_rounding_mode(options);
    if let Some(max) = options.maximum_significant_digits
        && max > 0
    {
        let magnitude = decimal.nonzero_magnitude_start();
        let position = magnitude - (i16::try_from(max).unwrap_or(0) - 1);
        decimal.round_with_mode(position, rounding_mode);
    }
    if let Some(min) = options.minimum_significant_digits
        && min > 0
    {
        let magnitude = decimal.nonzero_magnitude_start();
        let position = magnitude - (i16::try_from(min).unwrap_or(0) - 1);
        decimal.pad_end(position);
    }
}

fn apply_sign_display(decimal: &mut Decimal, options: &NumberFormatOptions) {
    let sd = match options.sign_display {
        SignDisplay::Always => FixedSignDisplay::Always,
        SignDisplay::Never => FixedSignDisplay::Never,
        SignDisplay::ExceptZero => FixedSignDisplay::ExceptZero,
        SignDisplay::Negative => FixedSignDisplay::Negative,
        SignDisplay::Auto => return,
    };
    decimal.apply_sign_display(sd);
}

fn apply_digit_options(decimal: &mut Decimal, options: &NumberFormatOptions) {
    apply_minimum_integer_digits(decimal, options);
    apply_fraction_digits(decimal, options);
    apply_significant_digits(decimal, options);
    apply_sign_display(decimal, options);
}

/// A `PartsWrite` implementation that collects formatted output into
/// `NumberFormatPart` values.
struct PartsCollector {
    parts: Vec<NumberFormatPart>,
    buf: String,
    current_part: Part,
}

impl PartsCollector {
    fn new() -> Self {
        Self {
            parts: Vec::new(),
            buf: String::new(),
            current_part: Part {
                category: "",
                value: "literal",
            },
        }
    }

    fn flush(&mut self) {
        if !self.buf.is_empty() {
            let text = std::mem::take(&mut self.buf);
            self.parts.push(NumberFormatPart {
                part_type: NumberFormatPartType::from(self.current_part.value),
                value: text,
            });
        }
    }

    fn into_parts(mut self) -> Vec<NumberFormatPart> {
        self.flush();
        self.parts
    }
}

impl fmt::Write for PartsCollector {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.buf.push_str(s);
        Ok(())
    }

    fn write_char(&mut self, c: char) -> fmt::Result {
        self.buf.push(c);
        Ok(())
    }
}

impl PartsWrite for PartsCollector {
    type SubPartsWrite = PartsCollector;

    fn with_part(
        &mut self,
        part: Part,
        mut f: impl FnMut(&mut Self::SubPartsWrite) -> fmt::Result,
    ) -> fmt::Result {
        self.flush();
        let prev_part = self.current_part;
        self.current_part = part;
        let result = f(self);
        self.flush();
        self.current_part = prev_part;
        result
    }
}

#[cfg(test)]
#[path = "number_format_tests.rs"]
mod number_format_tests;
