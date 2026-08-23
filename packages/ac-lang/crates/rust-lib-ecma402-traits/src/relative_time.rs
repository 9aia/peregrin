use crate::number_format::NumberFormatPartType;

/// Unit of time for relative formatting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RelativeTimeUnit {
    /// Seconds.
    Second,
    /// Minutes.
    Minute,
    /// Hours.
    Hour,
    /// Days.
    Day,
    /// Weeks.
    Week,
    /// Months.
    Month,
    /// Quarters.
    Quarter,
    /// Years.
    Year}

/// Style of relative time formatting.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum RelativeTimeStyle {
    /// Standard length.
    #[default]
    Long,
    /// Shorter form.
    Short,
    /// Shortest form.
    Narrow}

/// Numeric presentation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum RelativeTimeNumeric {
    /// Always include the numeric value, e.g. `"1 day ago"`.
    #[default]
    Always,
    /// Use natural language when possible, e.g. `"yesterday"`.
    Auto}

/// Options used to construct a `RelativeTimeFormat` instance.
#[derive(Debug, Clone)]
pub struct RelativeTimeOptions {
    /// Unit to use for `value`.
    pub unit: RelativeTimeUnit,
    /// Length of the output.
    pub style: RelativeTimeStyle,
    /// Whether to include the numeric value.
    pub numeric: RelativeTimeNumeric}

impl RelativeTimeOptions {
    /// Creates options for the requested `unit` with default style and
    /// numeric presentation.
    pub fn new(unit: RelativeTimeUnit) -> Self {
        Self {
            unit,
            style: RelativeTimeStyle::default(),
            numeric: RelativeTimeNumeric::default()}
    }
}

/// A single part of a formatted relative time, as returned by `format_to_parts`.
///
/// The numeric portion reuses `NumberFormatPartType` to align with the ECMA-402
/// PartitionRelativeTimePattern algorithm, which produces number parts followed
/// by `literal` parts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelativeTimeFormatPart {
    /// The part type.
    pub part_type: NumberFormatPartType,
    /// The formatted text for this part.
    pub value: String}

/// ECMA-402 `Intl.RelativeTimeFormat` behavior, backend-agnostic.
pub trait RelativeTimeFormat {
    /// Error type returned when formatting fails.
    type Error;

    /// Format `value` as a relative-time string.
    fn format(&self, value: f64) -> Result<String, Self::Error>;

    /// Format `value` and return the structured parts.
    fn format_to_parts(&self, value: f64) -> Result<Vec<RelativeTimeFormatPart>, Self::Error>;
}
