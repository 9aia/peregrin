/// The kind of code to display.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Type {
    /// Human-readable language name, e.g. `"English"`.
    #[default]
    Language,
    /// Human-readable region name, e.g. `"United States"`.
    Region,
    /// Human-readable script name, e.g. `"Latin"`.
    Script,
    /// Human-readable variant name.
    Variant,
    /// Human-readable currency name.
    Currency,
    /// Human-readable calendar name.
    Calendar,
    /// Human-readable date/time field name.
    DateTimeField,
}

impl Type {
    /// Returns the ECMA-402 `type` string (`"language"`, `"region"`, etc.).
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Language => "language",
            Self::Region => "region",
            Self::Script => "script",
            Self::Variant => "variant",
            Self::Currency => "currency",
            Self::Calendar => "calendar",
            Self::DateTimeField => "dateTimeField",
        }
    }
}

/// The style to use for the display name.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Style {
    /// Full name.
    #[default]
    Long,
    /// Short name or abbreviation.
    Short,
    /// Shortest form.
    Narrow,
}

/// The fallback behavior when a display name is not found.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Fallback {
    /// Return the original code.
    #[default]
    Code,
    /// Return an empty string.
    None,
}

/// The language display kind, used for language codes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum LanguageDisplay {
    /// Dialect-based naming.
    #[default]
    Dialect,
    /// Standard-based naming.
    Standard,
}

/// Options used to construct or query a `DisplayNames` instance.
#[derive(Debug, Clone, Default)]
pub struct DisplayNamesOptions {
    /// Desired length of the returned name.
    pub style: Style,
    /// Behavior when no name is available.
    pub fallback: Fallback,
    /// Whether to use dialect or standard naming for languages.
    pub language_display: LanguageDisplay,
}

/// ECMA-402 `Intl.DisplayNames` behavior, backend-agnostic.
pub trait DisplayNames {
    /// Error type returned when resolving a name fails.
    type Error;

    /// Returns the display name for `code` of `type_` according to `options`
    /// and the instance's locale.
    fn of(
        &self,
        code: &str,
        type_: Type,
        options: DisplayNamesOptions,
    ) -> Result<String, Self::Error>;
}