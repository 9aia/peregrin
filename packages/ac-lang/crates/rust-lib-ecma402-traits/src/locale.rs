/// Hour cycle override for `Intl.Locale` and `Intl.DateTimeFormat`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HourCycle {
    /// 11-hour cycle.
    H11,
    /// 12-hour cycle.
    H12,
    /// 23-hour cycle.
    H23,
    /// 24-hour cycle.
    H24,
}

impl HourCycle {
    /// Returns the Unicode hour cycle subtag string (`"h11"`, `"h12"`, etc.).
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::H11 => "h11",
            Self::H12 => "h12",
            Self::H23 => "h23",
            Self::H24 => "h24",
        }
    }
}

impl std::str::FromStr for HourCycle {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "h11" => Ok(Self::H11),
            "h12" => Ok(Self::H12),
            "h23" => Ok(Self::H23),
            "h24" => Ok(Self::H24),
            _ => Err(()),
        }
    }
}

/// Optional subtag overrides for `Intl.Locale`.
#[derive(Debug, Clone, Default)]
pub struct LocaleOverrides {
    /// Override the language subtag, e.g. `"en"`.
    pub language: Option<String>,
    /// Override the script subtag, e.g. `"Latn"`.
    pub script: Option<String>,
    /// Override the region subtag, e.g. `"US"`.
    pub region: Option<String>,
    /// Override the calendar subtag, e.g. `"buddhist"`.
    pub calendar: Option<String>,
    /// Override the numbering system subtag, e.g. `"latn"`.
    pub numbering_system: Option<String>,
    /// Override the hour cycle, e.g. `HourCycle::H23`.
    pub hour_cycle: Option<HourCycle>,
}

/// Canonicalized locale information returned by `Intl.Locale`.
#[derive(Debug, Clone)]
pub struct LocaleInfo {
    /// Full canonicalized locale tag, e.g. `"en-Latn-US"`.
    pub base_name: String,
    /// Language subtag.
    pub language: String,
    /// Script subtag, if any.
    pub script: Option<String>,
    /// Region subtag, if any.
    pub region: Option<String>,
    /// Calendar subtag, if any.
    pub calendar: Option<String>,
    /// Numbering system subtag, if any.
    pub numbering_system: Option<String>,
    /// Hour cycle, if any.
    pub hour_cycle: Option<HourCycle>,
}

/// ECMA-402 `Intl.Locale` behavior, backend-agnostic.
pub trait Locale {
    /// Error type returned when parsing or canonicalizing fails.
    type Error;

    /// Parse `tag` and apply `overrides`, returning the resulting locale information.
    fn parse(tag: &str, overrides: LocaleOverrides) -> Result<LocaleInfo, Self::Error>;
}
