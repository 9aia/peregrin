use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use ac_runtime::NonFunctionValue;
use ac_vm::{Context, Error, Function};
use icu::decimal::{CompactDecimalFormatter, DecimalFormatter};
use icu::decimal::options::{DecimalFormatterOptions, GroupingStrategy};
use icu::experimental::dimension::currency::CurrencyType;
use icu::experimental::dimension::currency::formatter::CurrencyFormatter;
use icu::experimental::dimension::currency::options::{CurrencyFormatterOptions, CurrencyUsage};
use icu::experimental::dimension::percent::formatter::PercentFormatter;
use icu::experimental::dimension::units::categorized_formatter::{CategorizedFormatter, CategorizedUnitsFormatterPreferences};
use icu::experimental::dimension::units::options::{UnitsFormatterOptions, Width};
use icu::experimental::measure::category::{Area, Duration, Length, Mass, Volume};
use writeable::Writeable;

use super::common::{apply_digit_options, apply_fraction_digits, apply_numbering_system, apply_sign_display, decimal_from_f64, make_provider, opt_str, optional_locale, optional_object, require_number, s, DataProvider};
use super::error::IntlError;
use super::parts::PartsCollector;
use super::provider::IntlProvider;

thread_local! {
    static DECIMAL_FORMATTERS: RefCell<HashMap<(String, String), Rc<DecimalFormatter>>> = RefCell::new(HashMap::new());
}

/// `intl.format_number(n: number, locale?: string, opts?: object) → string`
///
/// Formats a number using locale-aware decimal and thousands separators
/// via ICU4X.
///
/// Options:
/// - `style`: `"decimal"` (default), `"percent"`, `"currency"`, or `"unit"`
/// - `currency`: 3-letter ISO 4217 code (required when `style: "currency"`)
/// - `currencyDisplay`: `"symbol"` (default), `"narrowSymbol"`, `"code"`, or `"name"`
/// - `currencySign`: `"standard"` (default) or `"accounting"` (for negative values, uses parentheses)
/// - `notation`: `"standard"` (default), `"compact"`, `"scientific"`, or `"engineering"`
/// - `compactDisplay`: `"short"` (default) or `"long"` (for compact notation)
/// - `minimumIntegerDigits`: minimum integer digits to display (pads with leading zeros)
/// - `minimumFractionDigits`: minimum fraction digits to display
/// - `maximumFractionDigits`: maximum fraction digits to display
/// - `minimumSignificantDigits`: minimum significant digits
/// - `maximumSignificantDigits`: maximum significant digits
/// - `useGrouping`: `true` (default), `false`, `"auto"`, `"always"`, `"min2"`
/// - `unit`: sanctioned unit identifier (e.g. `"meter"`, `"kilometer-per-hour"`) — required when `style: "unit"`
/// - `unitDisplay`: `"short"` (default), `"narrow"`, or `"long"` (for `style: "unit"`)
/// - `signDisplay`: `"auto"` (default), `"always"`, `"never"`, `"exceptZero"`, `"negative"`
/// - `roundingMode`: `"halfEven"` (default), `"ceil"`, `"floor"`, `"expand"`, `"trunc"`, `"halfCeil"`, `"halfExpand"`, `"halfFloor"`, `"halfTrunc"`
///
/// Examples:
/// - `intl.format_number(1234567.89)` → `"1,234,567.89"`
/// - `intl.format_number(1234567.89, "pt-BR")` → `"1.234.567,89"`
/// - `intl.format_number(0.5, "en", {style: "percent"})` → `"50%"`
/// - `intl.format_number(1234.56, "en", {style: "currency", currency: "USD"})` → `"$1,234.56"`
/// - `intl.format_number(1000007, "bn")` → `"১০,০০,০০৭"`
/// - `intl.format_number(1000000, "en", {notation: "compact"})` → `"1M"`
#[derive(Clone)]
pub struct FormatNumber {
    provider: Option<IntlProvider>,
}

impl FormatNumber {
    pub fn new() -> Self {
        Self { provider: None }
    }

    pub fn with_provider(provider: IntlProvider) -> Self {
        Self { provider: Some(provider) }
    }
}

impl Default for FormatNumber {
    fn default() -> Self {
        Self::new()
    }
}

impl Function for FormatNumber {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let n = require_number(args, 0)?;
        let mut locale = optional_locale(args, 1)?;
        let opts = optional_object(args, 2)?;
        apply_numbering_system(&mut locale, opts);
        let dp = make_provider(&self.provider)?;

        let style = opt_str(opts, "style").unwrap_or("decimal");
        let notation = opt_str(opts, "notation").unwrap_or("standard");

        let formatted = match style {
            "percent" => format_percent(n, &locale, opts, &dp)?,
            "unit" => format_unit(n, &locale, opts, &dp)?,
            "currency" => {
                let currency = opt_str(opts, "currency")
                    .ok_or_else(|| IntlError::InvalidArgument {
                        message: "currency option required for currency style".into(),
                    }.into_vm_error())?;
                let display = opt_str(opts, "currencyDisplay").unwrap_or("symbol");
                format_currency(n, &locale, currency, display, opts, &dp)?
            }
            _ => match notation {
                "compact" => {
                    let compact_display = opt_str(opts, "compactDisplay").unwrap_or("short");
                    format_compact(n, &locale, opts, compact_display, &dp)?
                }
                "scientific" => format_scientific(n, &locale, opts, &dp)?,
                "engineering" => format_engineering(n, &locale, opts, &dp)?,
                _ => format_standard(n, &locale, opts, &dp)?,
            },
        };

        Ok(s(formatted))
    }
}

fn build_formatter_options(opts: Option<&serde_json::Map<String, serde_json::Value>>) -> DecimalFormatterOptions {
    let mut options = DecimalFormatterOptions::default();
    if let Some(o) = opts
        && let Some(v) = o.get("useGrouping") {
            if let Some(b) = v.as_bool() {
                options.grouping_strategy = if b {
                    Some(GroupingStrategy::Auto)
                } else {
                    Some(GroupingStrategy::Never)
                };
            } else if let Some(s) = v.as_str() {
                options.grouping_strategy = match s {
                    "min2" => Some(GroupingStrategy::Min2),
                    "auto" => Some(GroupingStrategy::Auto),
                    "always" => Some(GroupingStrategy::Always),
                    "false" | "never" => Some(GroupingStrategy::Never),
                    _ => Some(GroupingStrategy::Auto),
                };
            }
        }
    options
}

fn decimal_formatter(
    locale: &icu::locale::Locale,
    opts: Option<&serde_json::Map<String, serde_json::Value>>,
    dp: &DataProvider,
) -> Result<Rc<DecimalFormatter>, Error> {
    let formatter_opts = build_formatter_options(opts);
    if let Some(p) = dp.as_buffer_provider() {
        let f = DecimalFormatter::try_new_with_buffer_provider(p, locale.into(), formatter_opts)
            .map_err(|e| IntlError::Formatting { message: e.to_string() }.into_vm_error())?;
        return Ok(Rc::new(f));
    }

    let key = (locale.to_string(), format!("{:?}", formatter_opts));
    DECIMAL_FORMATTERS.with(|cache| -> Result<Rc<DecimalFormatter>, Error> {
        let mut cache = cache.borrow_mut();
        if let Some(f) = cache.get(&key) {
            Ok(Rc::clone(f))
        } else {
            let f = DecimalFormatter::try_new(locale.into(), formatter_opts)
                .map_err(|e| IntlError::Formatting { message: e.to_string() }.into_vm_error())?;
            let f = Rc::new(f);
            cache.insert(key, Rc::clone(&f));
            Ok(f)
        }
    })
}

fn format_standard(n: f64, locale: &icu::locale::Locale, opts: Option<&serde_json::Map<String, serde_json::Value>>, dp: &DataProvider) -> Result<String, Error> {
    let formatter = decimal_formatter(locale, opts, dp)?;

    let mut decimal = decimal_from_f64(n);
    apply_digit_options(&mut decimal, opts);
    let formatted = formatter.format(&decimal);
    Ok(formatted.write_to_string().into_owned())
}

fn format_compact(n: f64, locale: &icu::locale::Locale, opts: Option<&serde_json::Map<String, serde_json::Value>>, compact_display: &str, dp: &DataProvider) -> Result<String, Error> {
    let formatter = if compact_display == "long" {
        match dp.as_buffer_provider() {
            Some(p) => CompactDecimalFormatter::try_new_long_with_buffer_provider(p, locale.into(), Default::default()),
            None => CompactDecimalFormatter::try_new_long(locale.into(), Default::default()),
        }
    } else {
        match dp.as_buffer_provider() {
            Some(p) => CompactDecimalFormatter::try_new_short_with_buffer_provider(p, locale.into(), Default::default()),
            None => CompactDecimalFormatter::try_new_short(locale.into(), Default::default()),
        }
    }
    .map_err(|e| IntlError::Formatting { message: e.to_string() }.into_vm_error())?;

    let mut decimal = decimal_from_f64(n);
    apply_sign_display(&mut decimal, opts);
    let formatted = formatter.format(&decimal);
    Ok(formatted.write_to_string().into_owned())
}

fn format_decimal_with_options(
    decimal: &fixed_decimal::Decimal,
    locale: &icu::locale::Locale,
    opts: Option<&serde_json::Map<String, serde_json::Value>>,
    dp: &DataProvider,
) -> Result<String, Error> {
    let formatter = decimal_formatter(locale, opts, dp)?;
    let formatted = formatter.format(decimal);
    Ok(formatted.write_to_string().into_owned())
}

fn format_scientific(n: f64, locale: &icu::locale::Locale, opts: Option<&serde_json::Map<String, serde_json::Value>>, dp: &DataProvider) -> Result<String, Error> {
    if n == 0.0 || n.is_nan() || n.is_infinite() {
        return format_standard(n, locale, opts, dp);
    }
    let mut decimal = decimal_from_f64(n);
    apply_sign_display(&mut decimal, opts);
    let exponent = decimal.nonzero_magnitude_start() as i32;
    decimal.multiply_pow10(-exponent as i16);
    let mantissa_str = format_decimal_with_options(&decimal, locale, opts, dp)?;
    let exp_str = format_exponent(exponent, locale, opts, dp)?;
    Ok(format!("{mantissa_str}×10{exp_str}"))
}

fn format_engineering(n: f64, locale: &icu::locale::Locale, opts: Option<&serde_json::Map<String, serde_json::Value>>, dp: &DataProvider) -> Result<String, Error> {
    if n == 0.0 || n.is_nan() || n.is_infinite() {
        return format_standard(n, locale, opts, dp);
    }
    let mut decimal = decimal_from_f64(n);
    apply_sign_display(&mut decimal, opts);
    let raw_exp = decimal.nonzero_magnitude_start() as i32;
    let exponent = (raw_exp / 3) * 3;
    decimal.multiply_pow10(-exponent as i16);
    let mantissa_str = format_decimal_with_options(&decimal, locale, opts, dp)?;
    let exp_str = format_exponent(exponent, locale, opts, dp)?;
    Ok(format!("{mantissa_str}×10{exp_str}"))
}

fn format_exponent(exponent: i32, locale: &icu::locale::Locale, opts: Option<&serde_json::Map<String, serde_json::Value>>, dp: &DataProvider) -> Result<String, Error> {
    let formatter = decimal_formatter(locale, opts, dp)?;
    let mut decimal = fixed_decimal::Decimal::from(exponent);
    decimal.apply_sign_display(fixed_decimal::SignDisplay::ExceptZero);
    Ok(formatter.format(&decimal).write_to_string().into_owned())
}

fn format_percent(n: f64, locale: &icu::locale::Locale, opts: Option<&serde_json::Map<String, serde_json::Value>>, dp: &DataProvider) -> Result<String, Error> {
    let formatter = match dp.as_buffer_provider() {
        Some(p) => PercentFormatter::try_new_with_buffer_provider(p, locale.into(), Default::default()),
        None => PercentFormatter::try_new(locale.into(), Default::default()),
    }
    .map_err(|e| IntlError::Formatting { message: e.to_string() }.into_vm_error())?;

    let mut decimal = decimal_from_f64(n);
    decimal.multiply_pow10(2);
    decimal.trim_start();
    apply_fraction_digits(&mut decimal, opts);
    apply_sign_display(&mut decimal, opts);
    let formatted = formatter.format(&decimal);
    Ok(formatted.write_to_string().into_owned())
}

fn format_currency(n: f64, locale: &icu::locale::Locale, currency: &str, display: &str, opts: Option<&serde_json::Map<String, serde_json::Value>>, dp: &DataProvider) -> Result<String, Error> {
    let currency_code = CurrencyType::try_from_str(currency)
        .map_err(|_| IntlError::InvalidCurrency { code: currency.to_string() }.into_vm_error())?;

    let usage = match opt_str(opts, "currencySign") {
        Some("accounting") => CurrencyUsage::Accounting,
        _ => CurrencyUsage::Standard,
    };
    let mut currency_opts = CurrencyFormatterOptions::default();
    currency_opts.usage = usage;

    let formatter = match (display, dp.as_buffer_provider()) {
        ("code", Some(p)) => CurrencyFormatter::try_new_code_with_buffer_provider(p, locale.into(), currency_code, currency_opts),
        ("code", None) => CurrencyFormatter::try_new_code(locale.into(), currency_code, currency_opts),
        ("name", Some(p)) => CurrencyFormatter::try_new_name_with_buffer_provider(p, locale.into(), currency_code),
        ("name", None) => CurrencyFormatter::try_new_name(locale.into(), currency_code),
        ("narrowSymbol", Some(p)) => CurrencyFormatter::try_new_symbol_narrow_with_buffer_provider(p, locale.into(), currency_code, currency_opts),
        ("narrowSymbol", None) => CurrencyFormatter::try_new_symbol_narrow(locale.into(), currency_code, currency_opts),
        (_, Some(p)) => CurrencyFormatter::try_new_symbol_with_buffer_provider(p, locale.into(), currency_code, currency_opts),
        (_, None) => CurrencyFormatter::try_new_symbol(locale.into(), currency_code, currency_opts),
    }
    .map_err(|e| IntlError::Formatting { message: e.to_string() }.into_vm_error())?;

    let mut decimal = decimal_from_f64(n);
    apply_fraction_digits(&mut decimal, opts);
    apply_sign_display(&mut decimal, opts);
    let formatted = formatter.format_fixed_decimal(&decimal);
    Ok(formatted.write_to_string().into_owned())
}

/// Format a number with a unit (e.g. "5 meters", "30 km/h").
/// Uses ICU4X CategorizedFormatter with the available category constructors.
fn format_unit(n: f64, locale: &icu::locale::Locale, opts: Option<&serde_json::Map<String, serde_json::Value>>, _dp: &DataProvider) -> Result<String, Error> {
    let unit_str = opt_str(opts, "unit")
        .ok_or_else(|| IntlError::InvalidArgument {
            message: "unit option required for unit style".into(),
        }.into_vm_error())?;

    let unit_display = opt_str(opts, "unitDisplay").unwrap_or("short");
    let width = match unit_display {
        "long" => Width::Long,
        "narrow" => Width::Narrow,
        _ => Width::Short,
    };
    let mut unit_opts = UnitsFormatterOptions::default();
    unit_opts.width = width;

    let mut decimal = decimal_from_f64(n);
    apply_sign_display(&mut decimal, opts);

    let prefs: CategorizedUnitsFormatterPreferences = locale.into();

    // Try each available category constructor. The CategorizedFormatter API requires
    // a compile-time category type and a CategorizedMeasureUnit, which can only be
    // created via category-specific constructors (the _category field is pub(crate)).
    // We match the unit string to the appropriate constructor.
    macro_rules! try_unit {
        ($constructor:expr, $category:ident) => {{
            let categorized_unit = $constructor;
            let formatter = CategorizedFormatter::<$category>::try_new_outlier(
                prefs,
                categorized_unit,
                unit_opts,
            );
            if let Ok(fmt) = formatter {
                let formatted = fmt.format_fixed_decimal(&decimal);
                return Ok(formatted.write_to_string().into_owned());
            }
        }};
    }

    // Length units
    if unit_str == "meter" { try_unit!(Length::meter(), Length) }
    // Mass units
    match unit_str {
        "gram" => try_unit!(Mass::gram(), Mass),
        "kilogram" => try_unit!(Mass::kilogram(), Mass),
        _ => {}
    }
    // Area units
    if unit_str == "square-meter" { try_unit!(Area::square_meter(), Area) }
    // Volume units
    match unit_str {
        "liter" => try_unit!(Volume::liter(), Volume),
        "cubic-meter" => try_unit!(Volume::cubic_meter(), Volume),
        _ => {}
    }
    // Duration units
    match unit_str {
        "millisecond" => try_unit!(Duration::millisecond(), Duration),
        "microsecond" => try_unit!(Duration::microsecond(), Duration),
        "nanosecond" => try_unit!(Duration::nanosecond(), Duration),
        "second" => try_unit!(Duration::second(), Duration),
        "minute" => try_unit!(Duration::minute(), Duration),
        "hour" => try_unit!(Duration::hour(), Duration),
        "day" => try_unit!(Duration::day(), Duration),
        "week" => try_unit!(Duration::week(), Duration),
        "month" => try_unit!(Duration::month(), Duration),
        "year" => try_unit!(Duration::year(), Duration),
        _ => {}
    }

    Err(IntlError::InvalidUnit { unit: unit_str.to_string() }.into_vm_error())
}

// ── format_number_to_parts ───────────────────────────────────────

/// `intl.format_number_to_parts(n: number, locale?: string, opts?: object) → array`
///
/// Like [`FormatNumber`], but returns an array of `{ type, value }` objects
/// instead of a flat string. Uses ICU4X `write_to_parts` APIs to produce
/// structured part annotations (e.g. `"integer"`, `"group"`, `"decimal"`,
/// `"fraction"`, `"minusSign"`, `"plusSign"`).
#[derive(Clone)]
pub struct FormatNumberToParts {
    provider: Option<IntlProvider>,
}

impl FormatNumberToParts {
    pub fn new() -> Self {
        Self { provider: None }
    }

    pub fn with_provider(provider: IntlProvider) -> Self {
        Self { provider: Some(provider) }
    }
}

impl Default for FormatNumberToParts {
    fn default() -> Self {
        Self::new()
    }
}

impl Function for FormatNumberToParts {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let n = require_number(args, 0)?;
        let mut locale = optional_locale(args, 1)?;
        let opts = optional_object(args, 2)?;
        apply_numbering_system(&mut locale, opts);
        let dp = make_provider(&self.provider)?;

        let style = opt_str(opts, "style").unwrap_or("decimal");
        let notation = opt_str(opts, "notation").unwrap_or("standard");

        let mut collector = PartsCollector::new();
        match style {
            "percent" => format_percent_to_parts(n, &locale, opts, &dp, &mut collector)?,
            "unit" => format_unit_to_parts(n, &locale, opts, &dp, &mut collector)?,
            "currency" => {
                let currency = opt_str(opts, "currency")
                    .ok_or_else(|| IntlError::InvalidArgument {
                        message: "currency option required for currency style".into(),
                    }.into_vm_error())?;
                let display = opt_str(opts, "currencyDisplay").unwrap_or("symbol");
                format_currency_to_parts(n, &locale, currency, display, opts, &dp, &mut collector)?;
            }
            _ => match notation {
                "compact" => {
                    let compact_display = opt_str(opts, "compactDisplay").unwrap_or("short");
                    format_compact_to_parts(n, &locale, opts, compact_display, &dp, &mut collector)?;
                }
                "scientific" => format_scientific_to_parts(n, &locale, opts, &dp, &mut collector)?,
                "engineering" => format_engineering_to_parts(n, &locale, opts, &dp, &mut collector)?,
                _ => format_standard_to_parts(n, &locale, opts, &dp, &mut collector)?,
            },
        }

        Ok(collector.into_value())
    }
}

fn format_decimal_to_parts(
    decimal: &fixed_decimal::Decimal,
    locale: &icu::locale::Locale,
    opts: Option<&serde_json::Map<String, serde_json::Value>>,
    dp: &DataProvider,
    sink: &mut PartsCollector,
) -> Result<(), Error> {
    let formatter = decimal_formatter(locale, opts, dp)?;
    let formatted = formatter.format(decimal);
    formatted
        .write_to_parts(sink)
        .map_err(|e| IntlError::Formatting { message: e.to_string() }.into_vm_error())
}

fn format_standard_to_parts(
    n: f64,
    locale: &icu::locale::Locale,
    opts: Option<&serde_json::Map<String, serde_json::Value>>,
    dp: &DataProvider,
    sink: &mut PartsCollector,
) -> Result<(), Error> {
    let formatter = decimal_formatter(locale, opts, dp)?;

    let mut decimal = decimal_from_f64(n);
    apply_digit_options(&mut decimal, opts);
    let formatted = formatter.format(&decimal);
    formatted
        .write_to_parts(sink)
        .map_err(|e| IntlError::Formatting { message: e.to_string() }.into_vm_error())
}

fn format_compact_to_parts(
    n: f64,
    locale: &icu::locale::Locale,
    opts: Option<&serde_json::Map<String, serde_json::Value>>,
    compact_display: &str,
    dp: &DataProvider,
    sink: &mut PartsCollector,
) -> Result<(), Error> {
    let formatter = if compact_display == "long" {
        match dp.as_buffer_provider() {
            Some(p) => CompactDecimalFormatter::try_new_long_with_buffer_provider(p, locale.into(), Default::default()),
            None => CompactDecimalFormatter::try_new_long(locale.into(), Default::default()),
        }
    } else {
        match dp.as_buffer_provider() {
            Some(p) => CompactDecimalFormatter::try_new_short_with_buffer_provider(p, locale.into(), Default::default()),
            None => CompactDecimalFormatter::try_new_short(locale.into(), Default::default()),
        }
    }
    .map_err(|e| IntlError::Formatting { message: e.to_string() }.into_vm_error())?;

    let mut decimal = decimal_from_f64(n);
    apply_sign_display(&mut decimal, opts);
    let formatted = formatter.format(&decimal);
    formatted
        .write_to_parts(sink)
        .map_err(|e| IntlError::Formatting { message: e.to_string() }.into_vm_error())
}

fn format_scientific_to_parts(
    n: f64,
    locale: &icu::locale::Locale,
    opts: Option<&serde_json::Map<String, serde_json::Value>>,
    dp: &DataProvider,
    sink: &mut PartsCollector,
) -> Result<(), Error> {
    if n == 0.0 || n.is_nan() || n.is_infinite() {
        return format_standard_to_parts(n, locale, opts, dp, sink);
    }
    let mut decimal = decimal_from_f64(n);
    apply_sign_display(&mut decimal, opts);
    let exponent = decimal.nonzero_magnitude_start() as i32;
    decimal.multiply_pow10(-exponent as i16);
    format_decimal_to_parts(&decimal, locale, opts, dp, sink)?;
    use std::fmt::Write;
    write!(sink, "×10").map_err(|e| IntlError::Formatting { message: e.to_string() }.into_vm_error())?;
    let mut exp_sink = PartsCollector::new();
    format_exponent_to_parts(exponent, locale, opts, dp, &mut exp_sink)?;
    for (part_type, value) in exp_sink.into_parts() {
        if part_type == "literal" {
            sink.write_str(&value).map_err(|e| IntlError::Formatting { message: e.to_string() }.into_vm_error())?;
        }
    }
    Ok(())
}

fn format_engineering_to_parts(
    n: f64,
    locale: &icu::locale::Locale,
    opts: Option<&serde_json::Map<String, serde_json::Value>>,
    dp: &DataProvider,
    sink: &mut PartsCollector,
) -> Result<(), Error> {
    if n == 0.0 || n.is_nan() || n.is_infinite() {
        return format_standard_to_parts(n, locale, opts, dp, sink);
    }
    let mut decimal = decimal_from_f64(n);
    apply_sign_display(&mut decimal, opts);
    let raw_exp = decimal.nonzero_magnitude_start() as i32;
    let exponent = (raw_exp / 3) * 3;
    decimal.multiply_pow10(-exponent as i16);
    format_decimal_to_parts(&decimal, locale, opts, dp, sink)?;
    use std::fmt::Write;
    write!(sink, "×10").map_err(|e| IntlError::Formatting { message: e.to_string() }.into_vm_error())?;
    let mut exp_sink = PartsCollector::new();
    format_exponent_to_parts(exponent, locale, opts, dp, &mut exp_sink)?;
    for (part_type, value) in exp_sink.into_parts() {
        if part_type == "literal" {
            sink.write_str(&value).map_err(|e| IntlError::Formatting { message: e.to_string() }.into_vm_error())?;
        }
    }
    Ok(())
}

fn format_exponent_to_parts(
    exponent: i32,
    locale: &icu::locale::Locale,
    opts: Option<&serde_json::Map<String, serde_json::Value>>,
    dp: &DataProvider,
    sink: &mut PartsCollector,
) -> Result<(), Error> {
    let formatter = decimal_formatter(locale, opts, dp)?;
    let mut decimal = fixed_decimal::Decimal::from(exponent);
    decimal.apply_sign_display(fixed_decimal::SignDisplay::ExceptZero);
    let formatted = formatter.format(&decimal);
    formatted
        .write_to_parts(sink)
        .map_err(|e| IntlError::Formatting { message: e.to_string() }.into_vm_error())
}

fn format_percent_to_parts(
    n: f64,
    locale: &icu::locale::Locale,
    opts: Option<&serde_json::Map<String, serde_json::Value>>,
    dp: &DataProvider,
    sink: &mut PartsCollector,
) -> Result<(), Error> {
    let formatter = match dp.as_buffer_provider() {
        Some(p) => PercentFormatter::try_new_with_buffer_provider(p, locale.into(), Default::default()),
        None => PercentFormatter::try_new(locale.into(), Default::default()),
    }
    .map_err(|e| IntlError::Formatting { message: e.to_string() }.into_vm_error())?;

    let mut decimal = decimal_from_f64(n);
    decimal.multiply_pow10(2);
    decimal.trim_start();
    apply_fraction_digits(&mut decimal, opts);
    apply_sign_display(&mut decimal, opts);
    let formatted = formatter.format(&decimal);
    formatted
        .write_to_parts(sink)
        .map_err(|e| IntlError::Formatting { message: e.to_string() }.into_vm_error())
}

fn format_currency_to_parts(
    n: f64,
    locale: &icu::locale::Locale,
    currency: &str,
    display: &str,
    opts: Option<&serde_json::Map<String, serde_json::Value>>,
    dp: &DataProvider,
    sink: &mut PartsCollector,
) -> Result<(), Error> {
    let currency_code = CurrencyType::try_from_str(currency)
        .map_err(|_| IntlError::InvalidCurrency { code: currency.to_string() }.into_vm_error())?;

    let usage = match opt_str(opts, "currencySign") {
        Some("accounting") => CurrencyUsage::Accounting,
        _ => CurrencyUsage::Standard,
    };
    let mut currency_opts = CurrencyFormatterOptions::default();
    currency_opts.usage = usage;

    let formatter = match (display, dp.as_buffer_provider()) {
        ("code", Some(p)) => CurrencyFormatter::try_new_code_with_buffer_provider(p, locale.into(), currency_code, currency_opts),
        ("code", None) => CurrencyFormatter::try_new_code(locale.into(), currency_code, currency_opts),
        ("name", Some(p)) => CurrencyFormatter::try_new_name_with_buffer_provider(p, locale.into(), currency_code),
        ("name", None) => CurrencyFormatter::try_new_name(locale.into(), currency_code),
        ("narrowSymbol", Some(p)) => CurrencyFormatter::try_new_symbol_narrow_with_buffer_provider(p, locale.into(), currency_code, currency_opts),
        ("narrowSymbol", None) => CurrencyFormatter::try_new_symbol_narrow(locale.into(), currency_code, currency_opts),
        (_, Some(p)) => CurrencyFormatter::try_new_symbol_with_buffer_provider(p, locale.into(), currency_code, currency_opts),
        (_, None) => CurrencyFormatter::try_new_symbol(locale.into(), currency_code, currency_opts),
    }
    .map_err(|e| IntlError::Formatting { message: e.to_string() }.into_vm_error())?;

    let mut decimal = decimal_from_f64(n);
    apply_fraction_digits(&mut decimal, opts);
    apply_sign_display(&mut decimal, opts);
    let formatted = formatter.format_fixed_decimal(&decimal);
    formatted
        .write_to_parts(sink)
        .map_err(|e| IntlError::Formatting { message: e.to_string() }.into_vm_error())
}

fn format_unit_to_parts(
    n: f64,
    locale: &icu::locale::Locale,
    opts: Option<&serde_json::Map<String, serde_json::Value>>,
    _dp: &DataProvider,
    sink: &mut PartsCollector,
) -> Result<(), Error> {
    let unit_str = opt_str(opts, "unit")
        .ok_or_else(|| IntlError::InvalidArgument {
            message: "unit option required for unit style".into(),
        }.into_vm_error())?;

    let unit_display = opt_str(opts, "unitDisplay").unwrap_or("short");
    let width = match unit_display {
        "long" => Width::Long,
        "narrow" => Width::Narrow,
        _ => Width::Short,
    };
    let mut unit_opts = UnitsFormatterOptions::default();
    unit_opts.width = width;

    let mut decimal = decimal_from_f64(n);
    apply_sign_display(&mut decimal, opts);

    let prefs: CategorizedUnitsFormatterPreferences = locale.into();

    macro_rules! try_unit_parts {
        ($constructor:expr, $category:ident) => {{
            let categorized_unit = $constructor;
            let formatter = CategorizedFormatter::<$category>::try_new_outlier(
                prefs,
                categorized_unit,
                unit_opts,
            );
            if let Ok(fmt) = formatter {
                let formatted = fmt.format_fixed_decimal(&decimal);
                formatted
                    .write_to_parts(sink)
                    .map_err(|e| IntlError::Formatting { message: e.to_string() }.into_vm_error())?;
                return Ok(());
            }
        }};
    }

    if unit_str == "meter" { try_unit_parts!(Length::meter(), Length) }
    match unit_str {
        "gram" => try_unit_parts!(Mass::gram(), Mass),
        "kilogram" => try_unit_parts!(Mass::kilogram(), Mass),
        _ => {}
    }
    if unit_str == "square-meter" { try_unit_parts!(Area::square_meter(), Area) }
    match unit_str {
        "liter" => try_unit_parts!(Volume::liter(), Volume),
        "cubic-meter" => try_unit_parts!(Volume::cubic_meter(), Volume),
        _ => {}
    }
    match unit_str {
        "millisecond" => try_unit_parts!(Duration::millisecond(), Duration),
        "microsecond" => try_unit_parts!(Duration::microsecond(), Duration),
        "nanosecond" => try_unit_parts!(Duration::nanosecond(), Duration),
        "second" => try_unit_parts!(Duration::second(), Duration),
        "minute" => try_unit_parts!(Duration::minute(), Duration),
        "hour" => try_unit_parts!(Duration::hour(), Duration),
        "day" => try_unit_parts!(Duration::day(), Duration),
        "week" => try_unit_parts!(Duration::week(), Duration),
        "month" => try_unit_parts!(Duration::month(), Duration),
        "year" => try_unit_parts!(Duration::year(), Duration),
        _ => {}
    }

    Err(IntlError::InvalidUnit { unit: unit_str.to_string() }.into_vm_error())
}
