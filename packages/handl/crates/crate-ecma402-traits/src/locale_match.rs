/// ECMA-402 `Intl.getCanonicalLocales` behavior, backend-agnostic.
pub trait CanonicalLocales {
    /// Error type returned when canonicalizing fails.
    type Error;

    /// Canonicalize `locales`.
    fn canonicalize(locales: &[&str]) -> Result<Vec<String>, Self::Error>;
}

/// ECMA-402 `Intl.supportedLocalesOf` behavior, backend-agnostic.
pub trait SupportedLocalesOf {
    /// Error type returned when filtering fails.
    type Error;

    /// Filter `requested` to only the locales supported by the backend data.
    fn supported_locales_of(&self, requested: &[&str]) -> Result<Vec<String>, Self::Error>;
}
