use ac_runtime::NonFunctionValue;
use ac_vm::{Context, Error, Function};
use fixed_decimal::{SignDisplay, SignedRoundingMode, UnsignedRoundingMode};
use icu::decimal::input::Decimal;
use icu::decimal::{CompactDecimalFormatter, DecimalFormatter};
use icu::decimal::options::{DecimalFormatterOptions, GroupingStrategy};
use icu::experimental::dimension::currency::CurrencyType;
use icu::experimental::dimension::currency::formatter::CurrencyFormatter;
use icu::experimental::dimension::currency::options::{CurrencyFormatterOptions, CurrencyUsage};
use icu::experimental::dimension::percent::formatter::PercentFormatter;
use writeable::Writeable;

use super::common::{decimal_from_f64, make_provider, opt_bool, opt_int, opt_str, optional_locale, optional_object, require_number, s, DataProvider};
use super::error::IntlError;
use super::provider::IntlProvider;

/// `intl.format_number(n: number, locale?: string, opts?: object) → string`
///
/// Formats a number using locale-aware decimal and thousands separators
/// via ICU4X.
///
/// Options:
/// - `style`: `"decimal"` (default), `"percent"`, or `"currency"`
/// - `currency`: 3-letter ISO 4217 code (required when `style: "currency"`)
/// - `currencyDisplay`: `"symbol"` (default), `"narrowSymbol"`, or `"code"`
/// - `currencySign`: `"standard"` (default) or `"accounting"` (for negative values, uses parentheses)
/// - `notation`: `"standard"` (default) or `"compact"`
/// - `minimumFractionDigits`: minimum fraction digits to display
/// - `maximumFractionDigits`: maximum fraction digits to display
/// - `minimumSignificantDigits`: minimum significant digits
/// - `maximumSignificantDigits`: maximum significant digits
/// - `useGrouping`: `true` (default) or `false`
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
        let locale = optional_locale(args, 1)?;
        let opts = optional_object(args, 2)?;
        let dp = make_provider(&self.provider)?;

        let style = opt_str(opts, "style").unwrap_or("decimal");
        let notation = opt_str(opts, "notation").unwrap_or("standard");

        let formatted = match style {
            "percent" => format_percent(n, &locale, opts, &dp)?,
            "currency" => {
                let currency = opt_str(opts, "currency")
                    .ok_or_else(|| IntlError::InvalidArgument {
                        message: "currency option required for currency style".into(),
                    }.into_vm_error())?;
                let display = opt_str(opts, "currencyDisplay").unwrap_or("symbol");
                format_currency(n, &locale, currency, display, opts, &dp)?
            }
            _ => {
                if notation == "compact" {
                    format_compact(n, &locale, opts, &dp)?
                } else {
                    format_standard(n, &locale, opts, &dp)?
                }
            }
        };

        Ok(s(formatted))
    }
}

fn apply_sign_display(decimal: &mut Decimal, opts: Option<&serde_json::Map<String, serde_json::Value>>) {
    if let Some(sign_display) = opt_str(opts, "signDisplay") {
        let sd = match sign_display {
            "always" => SignDisplay::Always,
            "never" => SignDisplay::Never,
            "exceptZero" => SignDisplay::ExceptZero,
            "negative" => SignDisplay::Negative,
            _ => return,
        };
        decimal.apply_sign_display(sd);
    }
}

fn parse_rounding_mode(opts: Option<&serde_json::Map<String, serde_json::Value>>) -> SignedRoundingMode {
    match opt_str(opts, "roundingMode").unwrap_or("halfEven") {
        "ceil" => SignedRoundingMode::Ceil,
        "floor" => SignedRoundingMode::Floor,
        "expand" => SignedRoundingMode::Unsigned(UnsignedRoundingMode::Expand),
        "trunc" => SignedRoundingMode::Unsigned(UnsignedRoundingMode::Trunc),
        "halfCeil" => SignedRoundingMode::HalfCeil,
        "halfExpand" => SignedRoundingMode::Unsigned(UnsignedRoundingMode::HalfExpand),
        "halfFloor" => SignedRoundingMode::HalfFloor,
        "halfTrunc" => SignedRoundingMode::Unsigned(UnsignedRoundingMode::HalfTrunc),
        _ => SignedRoundingMode::Unsigned(UnsignedRoundingMode::HalfEven),
    }
}

fn apply_fraction_digits(decimal: &mut Decimal, opts: Option<&serde_json::Map<String, serde_json::Value>>) {
    let rounding_mode = parse_rounding_mode(opts);
    if let Some(min) = opt_int(opts, "minimumFractionDigits")
        && min > 0
    {
        decimal.pad_end(-i16::try_from(min).unwrap_or(0));
    }
    if let Some(max) = opt_int(opts, "maximumFractionDigits") {
        let max = i16::try_from(max).unwrap_or(0);
        if max >= 0 {
            decimal.round_with_mode(-max, rounding_mode);
            if let Some(min) = opt_int(opts, "minimumFractionDigits")
                && min > 0
            {
                decimal.pad_end(-i16::try_from(min).unwrap_or(0));
            }
        }
    }
}

fn apply_significant_digits(decimal: &mut Decimal, opts: Option<&serde_json::Map<String, serde_json::Value>>) {
    if let Some(min_sig) = opt_int(opts, "minimumSignificantDigits") {
        let min_sig = i16::try_from(min_sig).unwrap_or(0);
        if min_sig > 0 {
            decimal.pad_start(min_sig);
        }
    }
    if let Some(max_sig) = opt_int(opts, "maximumSignificantDigits") {
        let max_sig = i16::try_from(max_sig).unwrap_or(0);
        if max_sig > 0 {
            decimal.trim_start();
            decimal.pad_start(max_sig);
        }
    }
}

fn apply_all_options(decimal: &mut Decimal, opts: Option<&serde_json::Map<String, serde_json::Value>>) {
    apply_fraction_digits(decimal, opts);
    apply_significant_digits(decimal, opts);
    apply_sign_display(decimal, opts);
}

fn build_formatter_options(opts: Option<&serde_json::Map<String, serde_json::Value>>) -> DecimalFormatterOptions {
    let mut options = DecimalFormatterOptions::default();
    if let Some(false) = opt_bool(opts, "useGrouping") {
        options.grouping_strategy = Some(GroupingStrategy::Never);
    }
    options
}

fn format_standard(n: f64, locale: &icu::locale::Locale, opts: Option<&serde_json::Map<String, serde_json::Value>>, dp: &DataProvider) -> Result<String, Error> {
    let formatter_opts = build_formatter_options(opts);
    let formatter = match dp.as_buffer_provider() {
        Some(p) => DecimalFormatter::try_new_with_buffer_provider(p, locale.into(), formatter_opts),
        None => DecimalFormatter::try_new(locale.into(), formatter_opts),
    }
    .map_err(|e| IntlError::Formatting { message: e.to_string() }.into_vm_error())?;

    let mut decimal = decimal_from_f64(n);
    apply_all_options(&mut decimal, opts);
    let formatted = formatter.format(&decimal);
    Ok(formatted.write_to_string().into_owned())
}

fn format_compact(n: f64, locale: &icu::locale::Locale, opts: Option<&serde_json::Map<String, serde_json::Value>>, dp: &DataProvider) -> Result<String, Error> {
    let formatter = match dp.as_buffer_provider() {
        Some(p) => CompactDecimalFormatter::try_new_short_with_buffer_provider(p, locale.into(), Default::default()),
        None => CompactDecimalFormatter::try_new_short(locale.into(), Default::default()),
    }
    .map_err(|e| IntlError::Formatting { message: e.to_string() }.into_vm_error())?;

    let mut decimal = decimal_from_f64(n);
    apply_sign_display(&mut decimal, opts);
    let formatted = formatter.format(&decimal);
    Ok(formatted.write_to_string().into_owned())
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
