use std::fmt;

use ecma402_traits::locale_match::{
    CanonicalLocales as CanonicalLocalesTrait, SupportedLocalesOf as SupportedLocalesOfTrait,
};
use icu::decimal::provider::DecimalSymbolsV1;
use icu::locale::Locale;
use icu_provider::{DataError, DataMarker, IterableDataProvider, IterableDynamicDataProvider};
use icu_provider_blob::BlobDataProvider;

/// An ICU4X baked data provider that also exposes the locale inventory for
/// `DecimalSymbolsV1`. This is used to build the compiled-data supported locale
/// list at runtime instead of maintaining a hardcoded list.
struct CompiledDecimalProvider;

// Bring the generated macros into scope; the private `__impl_*` macros are
// needed by the public `impl_*` macros during expansion.
use icu_decimal_data::*;

make_provider!(CompiledDecimalProvider);
impl_decimal_symbols_v1!(CompiledDecimalProvider, ITER);

fn compiled_supported_locales() -> Result<Vec<String>, Error> {
    let ids = IterableDataProvider::<DecimalSymbolsV1>::iter_ids(&CompiledDecimalProvider)?;
    Ok(ids
        .into_iter()
        .map(|id| id.locale.to_string())
        .filter(|l| !l.starts_with("und"))
        .collect())
}

/// ICU4X-backed ECMA-402 `Intl.getCanonicalLocales`.
pub struct GetCanonicalLocales;

/// ICU4X-backed ECMA-402 `Intl.supportedLocalesOf`.
pub struct SupportedLocalesOf<'p> {
    provider: Option<&'p BlobDataProvider>,
}

/// Errors that can occur when matching locales.
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

impl GetCanonicalLocales {
    /// Canonicalize `locales`.
    pub fn canonicalize(locales: &[&str]) -> Result<Vec<String>, Error> {
        <Self as CanonicalLocalesTrait>::canonicalize(locales)
    }
}

impl CanonicalLocalesTrait for GetCanonicalLocales {
    type Error = Error;

    fn canonicalize(locales: &[&str]) -> Result<Vec<String>, Self::Error> {
        locales
            .iter()
            .map(|l| {
                l.parse::<Locale>()
                    .map(|loc| loc.to_string())
                    .map_err(|e| Error::InvalidLocale(format!("{l}: {e}")))
            })
            .collect()
    }
}

impl<'p> SupportedLocalesOf<'p> {
    /// Create a `SupportedLocalesOf` using the compiled CLDR data language list.
    pub fn new() -> Self {
        Self { provider: None }
    }

    /// Create a `SupportedLocalesOf` using an ICU4X blob data provider.
    pub fn with_blob(provider: &'p BlobDataProvider) -> Self {
        Self {
            provider: Some(provider),
        }
    }

    /// Return all locales supported by the backing data.
    pub fn all_supported(&self) -> Result<Vec<String>, Error> {
        match self.provider {
            Some(p) => {
                let ids = p.iter_ids_for_marker(DecimalSymbolsV1::INFO)?;
                Ok(ids
                    .into_iter()
                    .map(|id| id.locale.to_string())
                    .filter(|l| l != "und")
                    .collect())
            }
            None => compiled_supported_locales(),
        }
    }
}

impl<'p> Default for SupportedLocalesOf<'p> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'p> SupportedLocalesOfTrait for SupportedLocalesOf<'p> {
    type Error = Error;

    fn supported_locales_of(&self, requested: &[&str]) -> Result<Vec<String>, Self::Error> {
        let supported = self.all_supported()?;

        Ok(requested
            .iter()
            .filter(|r| {
                if let Ok(loc) = r.parse::<Locale>() {
                    let lang = loc.id.language.as_str();
                    supported.iter().any(|s| {
                        s == *r
                            || s == lang
                            || s.parse::<Locale>()
                                .is_ok_and(|sup| sup.id.language.as_str() == lang)
                    })
                } else {
                    supported.iter().any(|s| s == *r)
                }
            })
            .map(|s| s.to_string())
            .collect())
    }
}

#[cfg(test)]
#[path = "locale_match_tests.rs"]
mod tests;
