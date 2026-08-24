/// The kind of plural rule to apply.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum PluralRuleType {
    /// Cardinal numbers (the default), e.g. `"one apple"`.
    #[default]
    Cardinal,
    /// Ordinal numbers, e.g. `"1st"`.
    Ordinal,
}

/// Options used to construct a `PluralRules` instance.
#[derive(Debug, Clone, Default)]
pub struct PluralRulesOptions {
    /// Cardinal or ordinal selection.
    pub rule_type: PluralRuleType,
}

/// A CLDR plural category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PluralCategory {
    /// Plural category for quantity 0.
    Zero,
    /// Plural category for quantity 1.
    One,
    /// Plural category for quantity 2.
    Two,
    /// Plural category for a few items.
    Few,
    /// Plural category for many items.
    Many,
    /// Catch-all plural category.
    Other,
}

impl PluralCategory {
    /// Returns the category name used by ECMA-402 (`"zero"`, `"one"`, etc.).
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Zero => "zero",
            Self::One => "one",
            Self::Two => "two",
            Self::Few => "few",
            Self::Many => "many",
            Self::Other => "other",
        }
    }
}

/// ECMA-402 `Intl.PluralRules` behavior, backend-agnostic.
pub trait PluralRules {
    /// Error type returned when selecting a plural category fails.
    type Error;

    /// Select the plural category for `n` according to the configured
    /// locale and rule type.
    fn select(&self, n: f64) -> Result<PluralCategory, Self::Error>;
}