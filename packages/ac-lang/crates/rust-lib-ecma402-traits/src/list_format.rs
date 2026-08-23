/// The type of a formatted list part, as returned by `format_to_parts`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ListFormatPartType {
    /// A list element.
    Element,
    /// Separator text or other literal.
    Literal,
    /// An unrecognized part type produced by the backend.
    Other(String)}

impl ListFormatPartType {
    /// Returns the ECMA-402 `ListFormat` part type string.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Element => "element",
            Self::Literal => "literal",
            Self::Other(s) => s.as_str()}
    }
}

impl From<&str> for ListFormatPartType {
    fn from(value: &str) -> Self {
        match value {
            "element" => Self::Element,
            "literal" => Self::Literal,
            _ => Self::Other(value.to_string())}
    }
}

/// A single part of a formatted list, as returned by `format_to_parts`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListFormatPart {
    /// The part type.
    pub part_type: ListFormatPartType,
    /// The formatted text for this part.
    pub value: String}

/// The kind of list to format.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum ListType {
    /// `"and"`-style joining, e.g. `"A, B and C"`.
    #[default]
    Conjunction,
    /// `"or"`-style joining, e.g. `"A, B or C"`.
    Disjunction,
    /// Unit-style joining for measurements.
    Unit}

/// The style of list formatting.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum ListStyle {
    /// Standard length.
    #[default]
    Long,
    /// Shorter form.
    Short,
    /// Shortest form.
    Narrow}

/// Options used to construct a `ListFormat` instance.
#[derive(Debug, Clone, Default)]
pub struct ListFormatOptions {
    /// Kind of list to produce.
    pub list_type: ListType,
    /// Length of the produced list.
    pub style: ListStyle}

/// ECMA-402 `Intl.ListFormat` behavior, backend-agnostic.
pub trait ListFormat {
    /// Error type returned when formatting fails.
    type Error;

    /// Format `items` as a locale-aware list string.
    fn format(&self, items: &[&str]) -> Result<String, Self::Error>;

    /// Format `items` and return the structured parts.
    fn format_to_parts(&self, items: &[&str]) -> Result<Vec<ListFormatPart>, Self::Error>;
}
