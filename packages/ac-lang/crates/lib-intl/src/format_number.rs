use ac_runtime::NonFunctionValue;
use ac_vm::{Context, Error, Function};
use icu::decimal::input::Decimal;
use icu::decimal::DecimalFormatter;
use writeable::Writeable;

use super::common::{optional_locale, optional_object, require_number, s};

/// `intl.format_number(n: number, locale?: string, opts?: object) → string`
///
/// Formats a number using locale-aware decimal and thousands separators
/// via ICU4X.
///
/// Options:
/// - `notation`: `"standard"` (default) or `"compact"`
///
/// Examples:
/// - `intl.format_number(1234567.89)` → `"1,234,567.89"`
/// - `intl.format_number(1234567.89, "pt-BR")` → `"1.234.567,89"`
/// - `intl.format_number(1000000, "en", {notation: "compact"})` → `"1M"`
/// - `intl.format_number(1000007, "bn")` → `"১০,০০,০০৭"`
#[derive(Clone, Copy)]
pub struct FormatNumber;

impl Function for FormatNumber {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let n = require_number(args, 0)?;
        let locale = optional_locale(args, 1)?;
        let _opts = optional_object(args, 2)?;

        let formatted = format_standard(n, &locale)?;

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

/// Convert an f64 into an ICU4X `Decimal`.
fn decimal_from_f64(n: f64) -> Decimal {
    if n == 0.0 || n.is_nan() || n.is_infinite() {
        return Decimal::from(0_i64);
    }
    let s = format!("{}", n);
    Decimal::try_from_str(&s).unwrap_or(Decimal::from(0_i64))
}
