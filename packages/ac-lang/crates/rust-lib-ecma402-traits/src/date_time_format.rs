/// The type of a formatted date/time part, as returned by `format_to_parts`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DateTimeFormatPartType {
    /// Day of the month.
    Day,
    /// Day period, e.g. `"AM"` / `"PM"`.
    DayPeriod,
    /// Era.
    Era,
    /// Fractional seconds.
    FractionalSecond,
    /// Hour.
    Hour,
    /// Minute.
    Minute,
    /// Month.
    Month,
    /// Year related to another calendar.
    RelatedYear,
    /// Second.
    Second,
    /// Time zone name.
    TimeZoneName,
    /// Day of the week.
    Weekday,
    /// Year.
    Year,
    /// Any literal text, including separators and whitespace.
    Literal,
    /// An unrecognized part type produced by the backend.
    Other(String)}

impl DateTimeFormatPartType {
    /// Returns the ECMA-402 `DateTimeFormat` part type string.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Day => "day",
            Self::DayPeriod => "dayPeriod",
            Self::Era => "era",
            Self::FractionalSecond => "fractionalSecond",
            Self::Hour => "hour",
            Self::Minute => "minute",
            Self::Month => "month",
            Self::RelatedYear => "relatedYear",
            Self::Second => "second",
            Self::TimeZoneName => "timeZoneName",
            Self::Weekday => "weekday",
            Self::Year => "year",
            Self::Literal => "literal",
            Self::Other(s) => s.as_str()}
    }
}

impl From<&str> for DateTimeFormatPartType {
    fn from(value: &str) -> Self {
        match value {
            "day" => Self::Day,
            "dayPeriod" => Self::DayPeriod,
            "era" => Self::Era,
            "fractionalSecond" => Self::FractionalSecond,
            "hour" => Self::Hour,
            "minute" => Self::Minute,
            "month" => Self::Month,
            "relatedYear" => Self::RelatedYear,
            "second" => Self::Second,
            "timeZoneName" => Self::TimeZoneName,
            "weekday" => Self::Weekday,
            "year" => Self::Year,
            "literal" => Self::Literal,
            _ => Self::Other(value.to_string())}
    }
}

/// A single part of a formatted date, as returned by `format_to_parts`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DateTimeFormatPart {
    /// The part type.
    pub part_type: DateTimeFormatPartType,
    /// The formatted text for this part.
    pub value: String}

/// Overall date/time style.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum DateTimeStyle {
    /// Full style, e.g. `"Monday, January 1, 2024"`.
    Full,
    /// Long style.
    Long,
    /// Medium style (default).
    #[default]
    Medium,
    /// Short style.
    Short}

impl DateTimeStyle {
    /// Returns the ECMA-402 style string (`"full"`, `"long"`, etc.).
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Long => "long",
            Self::Medium => "medium",
            Self::Short => "short"}
    }
}

/// What kind of value is being formatted.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum DateType {
    /// Format a date only (default).
    #[default]
    Date,
    /// Format a time only.
    Time,
    /// Format a date and time.
    DateTime}

impl DateType {
    /// Returns the ECMA-402 type string (`"date"`, `"time"`, `"datetime"`).
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Date => "date",
            Self::Time => "time",
            Self::DateTime => "datetime"}
    }
}

pub use crate::locale::HourCycle;

/// Field-bag display style for a date or time component.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DateTimeFieldStyle {
    /// Numeric form, e.g. `"1"`.
    Numeric,
    /// Two-digit form, e.g. `"01"`.
    TwoDigit,
    /// Long form, e.g. `"January"`.
    Long,
    /// Short form, e.g. `"Jan"`.
    Short,
    /// Narrow form, e.g. `"J"`.
    Narrow}

impl DateTimeFieldStyle {
    /// Returns the ECMA-402 field value string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Numeric => "numeric",
            Self::TwoDigit => "2-digit",
            Self::Long => "long",
            Self::Short => "short",
            Self::Narrow => "narrow"}
    }
}

/// Time zone name style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TimeZoneNameStyle {
    /// Short form, e.g. `"PST"`.
    Short,
    /// Long form, e.g. `"Pacific Standard Time"`.
    Long,
    /// Short localized offset, e.g. `"GMT-8"`.
    ShortOffset,
    /// Long localized offset, e.g. `"GMT-08:00"`.
    LongOffset,
    /// Short generic form.
    ShortGeneric,
    /// Long generic form.
    LongGeneric}

impl TimeZoneNameStyle {
    /// Returns the ECMA-402 `timeZoneName` string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Short => "short",
            Self::Long => "long",
            Self::ShortOffset => "shortOffset",
            Self::LongOffset => "longOffset",
            Self::ShortGeneric => "shortGeneric",
            Self::LongGeneric => "longGeneric"}
    }
}

/// Options used to construct a `DateTimeFormat` instance.
///
/// Mirrors ECMA-402 `Intl.DateTimeFormat` options. Options with a closed set
/// of values use dedicated enums; open-ended values such as `calendar`,
/// `numbering_system`, and `time_zone` remain `Option<String>`.
#[derive(Debug, Clone)]
pub struct DateTimeFormatOptions {
    /// Overall style for date-only formatting.
    pub date_style: Option<DateTimeStyle>,
    /// Overall style for time-only/datetime formatting.
    pub time_style: Option<DateTimeStyle>,
    /// What kind of value is being formatted.
    pub date_type: DateType,
    /// Calendar override, e.g. `"buddhist"`, `"japanese"`, `"hebrew"`.
    pub calendar: Option<String>,
    /// Numbering system Unicode extension value.
    pub numbering_system: Option<String>,
    /// Whether to use 12-hour time. `hour_cycle` takes precedence.
    pub hour12: Option<bool>,
    /// Hour cycle.
    pub hour_cycle: Option<HourCycle>,
    /// Time zone identifier for zone-aware formatting.
    pub time_zone: Option<String>,
    /// Number of fractional second digits to show: `1`, `2`, or `3`.
    pub fractional_second_digits: Option<i64>,
    /// Field-bag style for `year`.
    pub year: Option<DateTimeFieldStyle>,
    /// Field-bag style for `month`.
    pub month: Option<DateTimeFieldStyle>,
    /// Field-bag style for `day`.
    pub day: Option<DateTimeFieldStyle>,
    /// Field-bag style for `weekday`.
    pub weekday: Option<DateTimeFieldStyle>,
    /// Field-bag style for `era`.
    pub era: Option<DateTimeFieldStyle>,
    /// Field-bag style for `hour`.
    pub hour: Option<DateTimeFieldStyle>,
    /// Field-bag style for `minute`.
    pub minute: Option<DateTimeFieldStyle>,
    /// Field-bag style for `second`.
    pub second: Option<DateTimeFieldStyle>,
    /// Field-bag style for `timeZoneName`.
    pub time_zone_name: Option<TimeZoneNameStyle>,
    /// Field-bag style for `dayPeriod`.
    pub day_period: Option<DateTimeFieldStyle>}

impl Default for DateTimeFormatOptions {
    fn default() -> Self {
        Self {
            date_style: None,
            time_style: None,
            date_type: DateType::Date,
            calendar: None,
            numbering_system: None,
            hour12: None,
            hour_cycle: None,
            time_zone: None,
            fractional_second_digits: None,
            year: None,
            month: None,
            day: None,
            weekday: None,
            era: None,
            hour: None,
            minute: None,
            second: None,
            time_zone_name: None,
            day_period: None}
    }
}

/// ECMA-402 `Intl.DateTimeFormat` behavior, backend-agnostic.
pub trait DateTimeFormat {
    /// Error type returned when formatting fails.
    type Error;

    /// Format `value` (an ISO 8601 or numeric Unix-epoch string) as a
    /// locale-aware date/time string.
    fn format(&self, value: &str) -> Result<String, Self::Error>;

    /// Format `value` and return the structured parts.
    fn format_to_parts(&self, value: &str) -> Result<Vec<DateTimeFormatPart>, Self::Error>;
}
