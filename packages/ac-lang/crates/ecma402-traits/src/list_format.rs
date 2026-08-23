/// The kind of list to format.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum ListType {
    /// `"and"`-style joining, e.g. `"A, B and C"`.
    #[default]
    Conjunction,
    /// `"or"`-style joining, e.g. `"A, B or C"`.
    Disjunction,
    /// Unit-style joining for measurements.
    Unit,
}

/// The style of list formatting.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum ListStyle {
    /// Standard length.
    #[default]
    Long,
    /// Shorter form.
    Short,
    /// Shortest form.
    Narrow,
}

/// Options used to construct a `ListFormat` instance.
#[derive(Debug, Clone, Default)]
pub struct ListFormatOptions {
    /// Kind of list to produce.
    pub list_type: ListType,
    /// Length of the produced list.
    pub style: ListStyle,
}

/// ECMA-402 `Intl.ListFormat` behavior, backend-agnostic.
pub trait ListFormat {
    /// Error type returned when formatting fails.
    type Error;

    /// Format `items` as a locale-aware list string.
    fn format(&self, items: &[&str]) -> Result<String, Self::Error>;
}
