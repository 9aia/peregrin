use std::fmt;

use ecma402_traits::plural_rules::{PluralCategory, PluralRuleType};
use fixed_decimal::{Decimal, FloatPrecision};
use icu::locale::Locale;
use icu::plurals::{
    PluralCategory as IcuPluralCategory, PluralRuleType as IcuPluralRuleType,
    PluralRules as IcuPluralRules, PluralRulesOptions,
};
use icu_provider::DataError;
use icu_provider_blob::BlobDataProvider;

pub use ecma402_traits::plural_rules::{
    PluralRules as PluralRulesTrait, PluralRulesOptions as Options,
};

/// ICU4X-backed ECMA-402 `Intl.PluralRules`.
pub struct PluralRules {
    rules: IcuPluralRules,
}

/// Errors that can occur when constructing or using a `PluralRules`.
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

impl PluralRules {
    /// Create a new `PluralRules` for the given BCP 47 locale tag using compiled CLDR data.
    pub fn new(locale: &str, options: Options) -> Result<Self, Error> {
        let locale: Locale = Self::parse_locale(locale)?;

        let rule_type = match options.rule_type {
            PluralRuleType::Cardinal => IcuPluralRuleType::Cardinal,
            PluralRuleType::Ordinal => IcuPluralRuleType::Ordinal,
        };
        let icu_options = PluralRulesOptions::default().with_type(rule_type);

        let rules = IcuPluralRules::try_new(locale.into(), icu_options)?;
        Ok(Self { rules })
    }

    /// Create a new `PluralRules` using an ICU4X blob data provider.
    pub fn with_blob(
        locale: &str,
        options: Options,
        provider: &BlobDataProvider,
    ) -> Result<Self, Error> {
        let locale: Locale = Self::parse_locale(locale)?;

        let rule_type = match options.rule_type {
            PluralRuleType::Cardinal => IcuPluralRuleType::Cardinal,
            PluralRuleType::Ordinal => IcuPluralRuleType::Ordinal,
        };
        let icu_options = PluralRulesOptions::default().with_type(rule_type);

        let rules =
            IcuPluralRules::try_new_with_buffer_provider(provider, locale.into(), icu_options)?;
        Ok(Self { rules })
    }

    /// Select the plural category for a pre-formatted decimal.
    pub fn select_decimal(&self, decimal: &Decimal) -> Result<PluralCategory, Error> {
        Ok(map_category(self.rules.category_for(decimal)))
    }

    fn parse_locale(locale: &str) -> Result<Locale, Error> {
        Locale::try_from_str(locale).map_err(|_| Error::InvalidLocale(locale.to_string()))
    }
}

impl PluralRulesTrait for PluralRules {
    type Error = Error;

    fn select(&self, n: f64) -> Result<PluralCategory, Self::Error> {
        let decimal = if n == 0.0 || n.is_nan() || n.is_infinite() {
            Decimal::from(0_i64)
        } else {
            Decimal::try_from_f64(n, FloatPrecision::RoundTrip).unwrap_or(Decimal::from(0_i64))
        };

        self.select_decimal(&decimal)
    }
}

fn map_category(category: IcuPluralCategory) -> PluralCategory {
    match category {
        IcuPluralCategory::Zero => PluralCategory::Zero,
        IcuPluralCategory::One => PluralCategory::One,
        IcuPluralCategory::Two => PluralCategory::Two,
        IcuPluralCategory::Few => PluralCategory::Few,
        IcuPluralCategory::Many => PluralCategory::Many,
        IcuPluralCategory::Other => PluralCategory::Other,
    }
}