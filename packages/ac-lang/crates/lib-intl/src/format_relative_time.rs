use ac_runtime::NonFunctionValue;
use ac_vm::{Context, Error, Function};
use fixed_decimal::Decimal;
use icu::experimental::relativetime::{RelativeTimeFormatter, RelativeTimeFormatterOptions};
use writeable::Writeable;

use super::common::{opt_str, optional_locale, optional_object, require_number, require_string, s};

/// `intl.format_relative_time(value: number, unit: string, locale?: string, opts?: object) → string`
///
/// Formats a relative time using ICU4X.
///
/// `unit` must be one of: `"second"`, `"minute"`, `"hour"`, `"day"`,
/// `"week"`, `"month"`, `"quarter"`, `"year"`.
///
/// Options:
/// - `style`: `"long"` (default), `"short"`, or `"narrow"`
/// - `numeric`: `"always"` (default) or `"auto"` (e.g. "yesterday" instead of "-1 days")
///
/// Examples:
/// - `intl.format_relative_time(5, "second")` → `"in 5 seconds"`
/// - `intl.format_relative_time(-10, "second")` → `"10 seconds ago"`
/// - `intl.format_relative_time(2, "day", "es", {numeric: "auto"})` → `"pasado mañana"`
/// - `intl.format_relative_time(3, "year", "bn")` → `"৩ বছরে"`
#[derive(Clone, Copy)]
pub struct FormatRelativeTime;

impl Function for FormatRelativeTime {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let value = require_number(args, 0)?;
        let unit = require_string(args, 1)?;
        let locale = optional_locale(args, 2)?;
        let opts = optional_object(args, 3)?;

        let style = opt_str(opts, "style").unwrap_or("long");
        let numeric = opt_str(opts, "numeric").unwrap_or("always");

        let mut options = RelativeTimeFormatterOptions::default();
        if numeric == "auto" {
            use icu::experimental::relativetime::options::Numeric;
            options.numeric = Numeric::Auto;
        }

        let formatter = create_formatter(&locale, unit, style, options)?;
        let decimal = decimal_from_f64(value);
        let formatted = formatter.format(decimal);
        Ok(s(formatted.write_to_string().into_owned()))
    }
}

fn create_formatter(
    locale: &icu::locale::Locale,
    unit: &str,
    style: &str,
    options: RelativeTimeFormatterOptions,
) -> Result<RelativeTimeFormatter, Error> {
    let result = match (unit, style) {
        ("second", "long") => RelativeTimeFormatter::try_new_long_second(locale.into(), options),
        ("second", "short") => RelativeTimeFormatter::try_new_short_second(locale.into(), options),
        ("second", "narrow") => RelativeTimeFormatter::try_new_narrow_second(locale.into(), options),
        ("minute", "long") => RelativeTimeFormatter::try_new_long_minute(locale.into(), options),
        ("minute", "short") => RelativeTimeFormatter::try_new_short_minute(locale.into(), options),
        ("minute", "narrow") => RelativeTimeFormatter::try_new_narrow_minute(locale.into(), options),
        ("hour", "long") => RelativeTimeFormatter::try_new_long_hour(locale.into(), options),
        ("hour", "short") => RelativeTimeFormatter::try_new_short_hour(locale.into(), options),
        ("hour", "narrow") => RelativeTimeFormatter::try_new_narrow_hour(locale.into(), options),
        ("day", "long") => RelativeTimeFormatter::try_new_long_day(locale.into(), options),
        ("day", "short") => RelativeTimeFormatter::try_new_short_day(locale.into(), options),
        ("day", "narrow") => RelativeTimeFormatter::try_new_narrow_day(locale.into(), options),
        ("week", "long") => RelativeTimeFormatter::try_new_long_week(locale.into(), options),
        ("week", "short") => RelativeTimeFormatter::try_new_short_week(locale.into(), options),
        ("week", "narrow") => RelativeTimeFormatter::try_new_narrow_week(locale.into(), options),
        ("month", "long") => RelativeTimeFormatter::try_new_long_month(locale.into(), options),
        ("month", "short") => RelativeTimeFormatter::try_new_short_month(locale.into(), options),
        ("month", "narrow") => RelativeTimeFormatter::try_new_narrow_month(locale.into(), options),
        ("quarter", "long") => RelativeTimeFormatter::try_new_long_quarter(locale.into(), options),
        ("quarter", "short") => RelativeTimeFormatter::try_new_short_quarter(locale.into(), options),
        ("quarter", "narrow") => RelativeTimeFormatter::try_new_narrow_quarter(locale.into(), options),
        ("year", "long") => RelativeTimeFormatter::try_new_long_year(locale.into(), options),
        ("year", "short") => RelativeTimeFormatter::try_new_short_year(locale.into(), options),
        ("year", "narrow") => RelativeTimeFormatter::try_new_narrow_year(locale.into(), options),
        _ => return Err(Error::PrimitiveError(format!("invalid unit: {unit}"))),
    };
    result.map_err(|e| Error::PrimitiveError(format!("formatting error: {e}")))
}

fn decimal_from_f64(n: f64) -> Decimal {
    if n == 0.0 || n.is_nan() || n.is_infinite() {
        return Decimal::from(0_i64);
    }
    let s = format!("{}", n);
    Decimal::try_from_str(&s).unwrap_or(Decimal::from(0_i64))
}
