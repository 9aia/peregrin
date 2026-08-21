use crate::data::{lookup_locale, LocaleData};
use crate::error::IntlError;

/// BCP 47 language tag (minimal: language + optional region).
///
/// All locale data is compile-time static, so `Locale` holds `&'static str`
/// slices and is `Copy` with zero heap allocation.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Locale {
    lang: &'static str,
    region: Option<&'static str>,
}

impl Locale {
    /// The fallback locale: `en`.
    pub const fn fallback() -> Self {
        Locale { lang: "en", region: None }
    }

    /// English locale constant.
    pub const fn en() -> Self {
        Locale { lang: "en", region: None }
    }

    /// Parse a BCP 47 tag like `"en"`, `"pt-BR"`, `"ar"`.
    ///
    /// Looks up the language in the static locale table. If the language is
    /// known, returns a `Locale` with `&'static str` slices. If unknown,
    /// returns `Err(UnsupportedLocale)`.
    ///
    /// Region is preserved if present but does not affect the static lookup
    /// (region-specific data falls back to language-level data).
    pub fn parse(input: &str) -> Result<Self, IntlError> {
        let (lang_part, region_part) = match input.split_once('-') {
            Some((l, r)) => (l, Some(r)),
            None => (input, None),
        };

        let lang_lower = lang_part.to_ascii_lowercase();
        let lang_static = lookup_locale(&lang_lower).ok_or(IntlError::UnsupportedLocale {
            locale: input.to_string(),
        })?;

        Ok(Locale {
            lang: lang_static.lang,
            region: region_part.map(|r| {
                let upper = r.to_ascii_uppercase();
                if lang_static.region.find(&upper).is_some() || !upper.is_empty() {
                    upper.leak()
                } else {
                    lang_static.lang
                }
            }),
        })
    }

    pub fn language(&self) -> &'static str {
        self.lang
    }

    pub fn region(&self) -> Option<&'static str> {
        self.region
    }

    /// Returns the locale data for this locale, falling back to English.
    pub fn data(&self) -> &'static LocaleData {
        lookup_locale(self.lang).unwrap_or_else(|| lookup_locale("en").unwrap())
    }
}

impl Default for Locale {
    fn default() -> Self {
        Self::fallback()
    }
}

impl std::fmt::Display for Locale {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.region {
            Some(r) => write!(f, "{}-{}", self.lang, r),
            None => write!(f, "{}", self.lang),
        }
    }
}
