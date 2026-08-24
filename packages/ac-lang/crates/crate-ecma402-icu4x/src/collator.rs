use std::cmp::Ordering;
use std::fmt;

use ecma402_traits::collator::{CaseFirst, Collator as CollatorTrait, Sensitivity, Usage};
use icu::collator::{
    Collator as IcuCollator, CollatorPreferences,
    options::{
        AlternateHandling, CaseLevel, CollatorOptions as IcuCollatorOptions, MaxVariable, Strength,
    },
    preferences::{CollationCaseFirst, CollationNumericOrdering, CollationType},
};
use icu::locale::Locale;
use icu_provider::DataError;
use icu_provider_blob::BlobDataProvider;

pub use ecma402_traits::collator::CollatorOptions as Options;

type Comparator = Box<dyn Fn(&str, &str) -> Ordering>;

/// ICU4X-backed ECMA-402 `Intl.Collator`.
pub struct Collator {
    compare_fn: Comparator,
}

/// Errors that can occur when constructing or using a `Collator`.
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

impl Collator {
    fn parse_locale(locale: &str) -> Result<Locale, Error> {
        Locale::try_from_str(locale).map_err(|_| Error::InvalidLocale(locale.to_string()))
    }

    fn make_options(options: &Options) -> IcuCollatorOptions {
        let mut icu_options = IcuCollatorOptions::default();
        icu_options.strength = Some(match options.sensitivity {
            Sensitivity::Base => Strength::Primary,
            Sensitivity::Accent => Strength::Secondary,
            Sensitivity::Case => {
                icu_options.case_level = Some(CaseLevel::On);
                Strength::Primary
            }
            _ => Strength::Tertiary,
        });

        if options.ignore_punctuation {
            icu_options.alternate_handling = Some(AlternateHandling::Shifted);
            icu_options.max_variable = Some(MaxVariable::Punctuation);
        }

        icu_options
    }

    fn make_preferences(options: &Options, locale: &Locale) -> CollatorPreferences {
        let mut prefs = CollatorPreferences::from(locale);

        prefs.case_first = match options.case_first {
            CaseFirst::Upper => Some(CollationCaseFirst::Upper),
            CaseFirst::Lower => Some(CollationCaseFirst::Lower),
            CaseFirst::False => Some(CollationCaseFirst::False),
            _ => None,
        };

        if let Some(numeric) = options.numeric {
            prefs.numeric_ordering = Some(if numeric {
                CollationNumericOrdering::True
            } else {
                CollationNumericOrdering::False
            });
        }

        if options.usage == Usage::Search || options.collation.as_deref() == Some("search") {
            prefs.collation_type = Some(CollationType::Search);
        } else if let Some(col) = options.collation.as_deref() {
            prefs.collation_type = match col {
                "phonebk" => Some(CollationType::Phonebk),
                "stroke" => Some(CollationType::Stroke),
                "trad" => Some(CollationType::Trad),
                "pinyin" => Some(CollationType::Pinyin),
                _ => None,
            };
        }

        prefs
    }

    /// Create a new `Collator` for the given BCP 47 locale tag using compiled CLDR data.
    pub fn new(locale: &str, options: Options) -> Result<Self, Error> {
        let locale: Locale = Self::parse_locale(locale)?;
        let icu_options = Self::make_options(&options);
        let prefs = Self::make_preferences(&options, &locale);

        let collator = IcuCollator::try_new(prefs, icu_options)?;
        Ok(Self {
            compare_fn: Box::new(move |a, b| collator.compare(a, b)),
        })
    }

    /// Create a new `Collator` using an ICU4X blob data provider.
    pub fn with_blob(
        locale: &str,
        options: Options,
        provider: &BlobDataProvider,
    ) -> Result<Self, Error> {
        let locale: Locale = Self::parse_locale(locale)?;
        let icu_options = Self::make_options(&options);
        let prefs = Self::make_preferences(&options, &locale);

        let collator = IcuCollator::try_new_with_buffer_provider(provider, prefs, icu_options)?;
        Ok(Self {
            compare_fn: Box::new(move |a, b| collator.as_borrowed().compare(a, b)),
        })
    }
}

impl CollatorTrait for Collator {
    type Error = Error;

    fn compare(&self, a: &str, b: &str) -> Result<Ordering, Self::Error> {
        Ok((self.compare_fn)(a, b))
    }
}
