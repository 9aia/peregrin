use std::cmp::Ordering;

/// How strictly the comparison must match.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Sensitivity {
    /// Distinguish all differences, including variant and case.
    #[default]
    Variant,
    /// Compare only base letters, ignoring diacritics, case, and variants.
    Base,
    /// Distinguish accented forms, but ignore case and variant.
    Accent,
    /// Distinguish case, but ignore accent and variant differences.
    Case,
}

/// Case-first ordering preference.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum CaseFirst {
    /// Use the locale's default case ordering.
    #[default]
    Default,
    /// Sort uppercase before lowercase.
    Upper,
    /// Sort lowercase before uppercase.
    Lower,
    /// Do not change case ordering.
    False,
}

/// Intended use for the collator.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Usage {
    /// General string sorting.
    #[default]
    Sort,
    /// Searching for matches.
    Search,
}

/// Options used to construct a `Collator` instance.
#[derive(Debug, Clone, Default)]
pub struct CollatorOptions {
    /// Level of difference that triggers a non-equal result.
    pub sensitivity: Sensitivity,
    /// Whether uppercase or lowercase letters come first.
    pub case_first: CaseFirst,
    /// `true` to treat groups of digits as numbers.
    pub numeric: Option<bool>,
    /// `true` to ignore punctuation.
    pub ignore_punctuation: bool,
    /// Whether the collator is for sorting or searching.
    pub usage: Usage,
    /// Requested Unicode collation identifier, if any.
    pub collation: Option<String>,
}

/// ECMA-402 `Intl.Collator` behavior, backend-agnostic.
pub trait Collator {
    /// Error type returned when comparison fails.
    type Error;

    /// Compare `a` and `b`, returning their ordering.
    fn compare(&self, a: &str, b: &str) -> Result<Ordering, Self::Error>;
}
