use ac_runtime::NonFunctionValue;
use ac_vm::{Context, Error, Function};
use icu::decimal::input::Decimal;
use icu::decimal::DecimalFormatter;
use icu::experimental::dimension::currency::CurrencyType;
use icu::experimental::dimension::currency::formatter::CurrencyFormatter;
use icu::experimental::dimension::percent::formatter::PercentFormatter;
use writeable::Writeable;

use super::common::{opt_str, optional_locale, optional_object, require_number, s};

/// `intl.format_number(n: number, locale?: string, opts?: object) → string`
///
/// Formats a number using locale-aware decimal and thousands separators
/// via ICU4X.
///
/// Options:
/// - `style`: `"decimal"` (default), `"percent"`, or `"currency"`
/// - `currency`: 3-letter ISO 4217 code (required when `style: "currency"`)
/// - `currencyDisplay`: `"symbol"` (default) or `"code"`
///
/// Examples:
/// - `intl.format_number(1234567.89)` → `"1,234,567.89"`
/// - `intl.format_number(1234567.89, "pt-BR")` → `"1.234.567,89"`
/// - `intl.format_number(0.5, "en", {style: "percent"})` → `"50%"`
/// - `intl.format_number(1234.56, "en", {style: "currency", currency: "USD"})` → `"$1,234.56"`
/// - `intl.format_number(1000007, "bn")` → `"১০,০০,০০৭"`
#[derive(Clone, Copy)]
pub struct FormatNumber;

impl Function for FormatNumber {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let n = require_number(args, 0)?;
        let locale = optional_locale(args, 1)?;
        let opts = optional_object(args, 2)?;

        let style = opt_str(opts, "style").unwrap_or("decimal");

        let formatted = match style {
            "percent" => format_percent(n, &locale)?,
            "currency" => {
                let currency = opt_str(opts, "currency")
                    .ok_or_else(|| Error::PrimitiveError("currency option required for currency style".into()))?;
                let display = opt_str(opts, "currencyDisplay").unwrap_or("symbol");
                format_currency(n, &locale, currency, display)?
            }
            _ => format_standard(n, &locale)?,
        };

        Ok(s(formatted))
    }
}

fn format_standard(n: f64, locale: &icu::locale::Locale) -> Result<String, Error> {
    let formatter = DecimalFormatter::try_new(locale.into(), Default::default())
        .map_err(|e| Error::PrimitiveError(format!("formatting error: {e}")))?;

    let decimal = decimal_from_f64(n);
    let formatted = formatter.format(&decimal);
    Ok(formatted.write_to_string().into_owned())
}

fn format_percent(n: f64, locale: &icu::locale::Locale) -> Result<String, Error> {
    let formatter = PercentFormatter::try_new(locale.into(), Default::default())
        .map_err(|e| Error::PrimitiveError(format!("formatting error: {e}")))?;

    let decimal = decimal_from_f64(n * 100.0);
    let formatted = formatter.format(&decimal);
    Ok(formatted.write_to_string().into_owned())
}

fn format_currency(n: f64, locale: &icu::locale::Locale, currency: &str, display: &str) -> Result<String, Error> {
    let currency_code = CurrencyType::try_from_str(currency)
        .map_err(|_| Error::PrimitiveError(format!("invalid currency code: {currency}")))?;

    let formatter = match display {
        "code" => CurrencyFormatter::try_new_code(locale.into(), currency_code, Default::default()),
        _ => CurrencyFormatter::try_new_symbol(locale.into(), currency_code, Default::default()),
    }
    .map_err(|e| Error::PrimitiveError(format!("formatting error: {e}")))?;

    let decimal = decimal_from_f64(n);
    let formatted = formatter.format_fixed_decimal(&decimal);
    Ok(formatted.write_to_string().into_owned())
}

/// Convert an f64 into an ICU4X `Decimal`.
fn decimal_from_f64(n: f64) -> Decimal {
    if n == 0.0 || n.is_nan() || n.is_infinite() {
        return Decimal::from(0_i64);
    }
    let s = format!("{}", n);
    Decimal::try_from_str(&s).unwrap_or(Decimal::from(0_i64))
}
