/// Number format style.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum NumberFormatStyle {
    /// Decimal formatting.
    #[default]
    Decimal,
    /// Currency formatting.
    Currency,
    /// Percent formatting.
    Percent,
    /// Unit formatting.
    Unit}

impl NumberFormatStyle {
    /// Returns the ECMA-402 `style` string (`"decimal"`, `"currency"`, etc.).
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Decimal => "decimal",
            Self::Currency => "currency",
            Self::Percent => "percent",
            Self::Unit => "unit"}
    }
}

/// How to display a currency value.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum CurrencyDisplay {
    /// Use a localized currency symbol.
    #[default]
    Symbol,
    /// Use a narrow currency symbol.
    NarrowSymbol,
    /// Use the ISO 4217 currency code.
    Code,
    /// Use a localized currency name.
    Name}

impl CurrencyDisplay {
    /// Returns the ECMA-402 `currencyDisplay` string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Symbol => "symbol",
            Self::NarrowSymbol => "narrowSymbol",
            Self::Code => "code",
            Self::Name => "name"}
    }
}

/// Currency sign convention.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum CurrencySign {
    /// Standard sign display.
    #[default]
    Standard,
    /// Accounting-style sign display.
    Accounting}

impl CurrencySign {
    /// Returns the ECMA-402 `currencySign` string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::Accounting => "accounting"}
    }
}

/// Number notation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Notation {
    /// Standard notation.
    #[default]
    Standard,
    /// Scientific notation.
    Scientific,
    /// Engineering notation.
    Engineering,
    /// Compact notation.
    Compact}

impl Notation {
    /// Returns the ECMA-402 `notation` string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::Scientific => "scientific",
            Self::Engineering => "engineering",
            Self::Compact => "compact"}
    }
}

/// Compact display style.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum CompactDisplay {
    /// Short form, e.g. `"10K"`.
    #[default]
    Short,
    /// Long form, e.g. `"10 thousand"`.
    Long}

impl CompactDisplay {
    /// Returns the ECMA-402 `compactDisplay` string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Short => "short",
            Self::Long => "long"}
    }
}

/// Grouping-separator behavior.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum UseGrouping {
    /// Use grouping separators (legacy boolean `true`).
    #[default]
    True,
    /// Do not use grouping separators (legacy boolean `false`).
    False,
    /// Use grouping separators according to locale conventions.
    Auto,
    /// Always use grouping separators.
    Always,
    /// Use grouping only when there are at least 2 digits before the decimal.
    Min2,
    /// Never use grouping separators.
    Never}

impl UseGrouping {
    /// Returns the ECMA-402 `useGrouping` string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::True => "true",
            Self::False => "false",
            Self::Auto => "auto",
            Self::Always => "always",
            Self::Min2 => "min2",
            Self::Never => "never"}
    }
}

/// Unit display style.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum UnitDisplay {
    /// Short form.
    #[default]
    Short,
    /// Narrow form.
    Narrow,
    /// Long form.
    Long}

impl UnitDisplay {
    /// Returns the ECMA-402 `unitDisplay` string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Short => "short",
            Self::Narrow => "narrow",
            Self::Long => "long"}
    }
}

/// Sign display behavior.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum SignDisplay {
    /// Show the sign for negative numbers only (default).
    #[default]
    Auto,
    /// Always show the sign.
    Always,
    /// Show the sign for positive and negative, but not zero.
    ExceptZero,
    /// Show the sign for negative numbers, including negative zero.
    Negative,
    /// Never show the sign.
    Never}

impl SignDisplay {
    /// Returns the ECMA-402 `signDisplay` string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Always => "always",
            Self::ExceptZero => "exceptZero",
            Self::Negative => "negative",
            Self::Never => "never"}
    }
}

/// Rounding mode for number formatting.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum RoundingMode {
    /// Round towards positive infinity.
    Ceil,
    /// Round towards negative infinity.
    Floor,
    /// Round away from zero.
    Expand,
    /// Round towards zero.
    Trunc,
    /// Round halves towards positive infinity.
    HalfCeil,
    /// Round halves towards negative infinity.
    HalfFloor,
    /// Round halves away from zero.
    HalfExpand,
    /// Round halves towards zero.
    HalfTrunc,
    /// Round halves to the nearest even number (default).
    #[default]
    HalfEven}

impl RoundingMode {
    /// Returns the ECMA-402 `roundingMode` string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ceil => "ceil",
            Self::Floor => "floor",
            Self::Expand => "expand",
            Self::Trunc => "trunc",
            Self::HalfCeil => "halfCeil",
            Self::HalfFloor => "halfFloor",
            Self::HalfExpand => "halfExpand",
            Self::HalfTrunc => "halfTrunc",
            Self::HalfEven => "halfEven"}
    }
}

/// The type of a formatted number part, as returned by `format_to_parts`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NumberFormatPartType {
    /// Integer digits.
    Integer,
    /// Fraction digits.
    Fraction,
    /// Decimal separator.
    Decimal,
    /// Grouping separator.
    Group,
    /// Minus sign.
    MinusSign,
    /// Plus sign.
    PlusSign,
    /// Percent sign.
    PercentSign,
    /// Currency string.
    Currency,
    /// Exponent integer digits.
    ExponentInteger,
    /// Minus sign in the exponent.
    ExponentMinusSign,
    /// Plus sign in the exponent.
    ExponentPlusSign,
    /// Separator between mantissa and exponent.
    ExponentSeparator,
    /// Not-a-number.
    Nan,
    /// Infinity.
    Infinity,
    /// Compact display marker.
    Compact,
    /// Unit string.
    Unit,
    /// Any literal text, including prefixes and suffixes.
    Literal,
    /// An unrecognized part type produced by the backend.
    Other(String)}

impl NumberFormatPartType {
    /// Returns the ECMA-402 `NumberFormat` part type string.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Integer => "integer",
            Self::Fraction => "fraction",
            Self::Decimal => "decimal",
            Self::Group => "group",
            Self::MinusSign => "minusSign",
            Self::PlusSign => "plusSign",
            Self::PercentSign => "percentSign",
            Self::Currency => "currency",
            Self::ExponentInteger => "exponentInteger",
            Self::ExponentMinusSign => "exponentMinusSign",
            Self::ExponentPlusSign => "exponentPlusSign",
            Self::ExponentSeparator => "exponentSeparator",
            Self::Nan => "nan",
            Self::Infinity => "infinity",
            Self::Compact => "compact",
            Self::Unit => "unit",
            Self::Literal => "literal",
            Self::Other(s) => s.as_str()}
    }
}

impl From<&str> for NumberFormatPartType {
    fn from(value: &str) -> Self {
        match value {
            "integer" => Self::Integer,
            "fraction" => Self::Fraction,
            "decimal" => Self::Decimal,
            "group" => Self::Group,
            "minusSign" => Self::MinusSign,
            "plusSign" => Self::PlusSign,
            "percentSign" => Self::PercentSign,
            "currency" => Self::Currency,
            "exponentInteger" => Self::ExponentInteger,
            "exponentMinusSign" => Self::ExponentMinusSign,
            "exponentPlusSign" => Self::ExponentPlusSign,
            "exponentSeparator" => Self::ExponentSeparator,
            "nan" => Self::Nan,
            "infinity" => Self::Infinity,
            "compact" => Self::Compact,
            "unit" => Self::Unit,
            "literal" => Self::Literal,
            _ => Self::Other(value.to_string())}
    }
}

/// A single part of a formatted number, as returned by `format_to_parts`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NumberFormatPart {
    /// The part type.
    pub part_type: NumberFormatPartType,
    /// The formatted text for this part.
    pub value: String}

/// Options used to construct a `NumberFormat` instance.
///
/// Strings are only used for open-ended values such as `currency`, `unit`,
/// and `numbering_system`. Options with a closed set of ECMA-402 values use
/// dedicated enums.
#[derive(Debug, Clone)]
pub struct NumberFormatOptions {
    /// Number style.
    pub style: NumberFormatStyle,
    /// ISO 4217 currency code when `style` is `NumberFormatStyle::Currency`.
    pub currency: Option<String>,
    /// How to display the currency.
    pub currency_display: CurrencyDisplay,
    /// Accounting sign convention.
    pub currency_sign: CurrencySign,
    /// Number notation.
    pub notation: Notation,
    /// Compact display style.
    pub compact_display: CompactDisplay,
    /// Minimum integer digits. `None` means use the implementation default.
    pub minimum_integer_digits: Option<i64>,
    /// Minimum fraction digits. `None` means use the implementation default.
    pub minimum_fraction_digits: Option<i64>,
    /// Maximum fraction digits. `None` means use the implementation default.
    pub maximum_fraction_digits: Option<i64>,
    /// Minimum significant digits. `None` means use the implementation default.
    pub minimum_significant_digits: Option<i64>,
    /// Maximum significant digits. `None` means use the implementation default.
    pub maximum_significant_digits: Option<i64>,
    /// Whether to use grouping separators.
    pub use_grouping: UseGrouping,
    /// Unit identifier when `style` is `NumberFormatStyle::Unit`.
    pub unit: Option<String>,
    /// Unit display style.
    pub unit_display: UnitDisplay,
    /// Sign display behavior.
    pub sign_display: SignDisplay,
    /// Rounding mode.
    pub rounding_mode: RoundingMode,
    /// Numbering system override.
    pub numbering_system: Option<String>}

impl Default for NumberFormatOptions {
    fn default() -> Self {
        Self {
            style: NumberFormatStyle::default(),
            currency: None,
            currency_display: CurrencyDisplay::default(),
            currency_sign: CurrencySign::default(),
            notation: Notation::default(),
            compact_display: CompactDisplay::default(),
            minimum_integer_digits: Some(1),
            minimum_fraction_digits: None,
            maximum_fraction_digits: None,
            minimum_significant_digits: None,
            maximum_significant_digits: None,
            use_grouping: UseGrouping::default(),
            unit: None,
            unit_display: UnitDisplay::default(),
            sign_display: SignDisplay::default(),
            rounding_mode: RoundingMode::default(),
            numbering_system: None}
    }
}

/// ECMA-402 `Intl.NumberFormat` behavior, backend-agnostic.
pub trait NumberFormat {
    /// Error type returned when formatting fails.
    type Error;

    /// Format `value` as a locale-aware number string.
    fn format(&self, value: f64) -> Result<String, Self::Error>;

    /// Format `value` and return the structured parts.
    fn format_to_parts(&self, value: f64) -> Result<Vec<NumberFormatPart>, Self::Error>;
}
