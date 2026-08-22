use ac_runtime::NonFunctionValue;
use ac_vm::{Context, Error, Function};
use icu::experimental::relativetime::{RelativeTimeFormatter, RelativeTimeFormatterOptions};
use writeable::Writeable;

use super::common::{decimal_from_f64, make_provider, opt_str, optional_locale, optional_object, require_number, require_string, s, DataProvider};
use super::error::IntlError;
use super::provider::IntlProvider;

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
#[derive(Clone)]
pub struct FormatRelativeTime {
    provider: Option<IntlProvider>,
}

impl FormatRelativeTime {
    pub fn new() -> Self {
        Self { provider: None }
    }

    pub fn with_provider(provider: IntlProvider) -> Self {
        Self { provider: Some(provider) }
    }
}

impl Default for FormatRelativeTime {
    fn default() -> Self {
        Self::new()
    }
}

impl Function for FormatRelativeTime {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let value = require_number(args, 0)?;
        let unit = require_string(args, 1)?;
        let locale = optional_locale(args, 2)?;
        let opts = optional_object(args, 3)?;
        let dp = make_provider(&self.provider)?;

        let style = opt_str(opts, "style").unwrap_or("long");
        let numeric = opt_str(opts, "numeric").unwrap_or("always");

        let mut options = RelativeTimeFormatterOptions::default();
        if numeric == "auto" {
            use icu::experimental::relativetime::options::Numeric;
            options.numeric = Numeric::Auto;
        }

        let formatter = create_formatter(&locale, unit, style, options, &dp)?;
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
    dp: &DataProvider,
) -> Result<RelativeTimeFormatter, Error> {
    let prefs = locale.into();
    let result = match (unit, style) {
        ("second", "long") => match dp.as_buffer_provider() {
            Some(p) => RelativeTimeFormatter::try_new_long_second_with_buffer_provider(p, prefs, options),
            None => RelativeTimeFormatter::try_new_long_second(prefs, options),
        },
        ("second", "short") => match dp.as_buffer_provider() {
            Some(p) => RelativeTimeFormatter::try_new_short_second_with_buffer_provider(p, prefs, options),
            None => RelativeTimeFormatter::try_new_short_second(prefs, options),
        },
        ("second", "narrow") => match dp.as_buffer_provider() {
            Some(p) => RelativeTimeFormatter::try_new_narrow_second_with_buffer_provider(p, prefs, options),
            None => RelativeTimeFormatter::try_new_narrow_second(prefs, options),
        },
        ("minute", "long") => match dp.as_buffer_provider() {
            Some(p) => RelativeTimeFormatter::try_new_long_minute_with_buffer_provider(p, prefs, options),
            None => RelativeTimeFormatter::try_new_long_minute(prefs, options),
        },
        ("minute", "short") => match dp.as_buffer_provider() {
            Some(p) => RelativeTimeFormatter::try_new_short_minute_with_buffer_provider(p, prefs, options),
            None => RelativeTimeFormatter::try_new_short_minute(prefs, options),
        },
        ("minute", "narrow") => match dp.as_buffer_provider() {
            Some(p) => RelativeTimeFormatter::try_new_narrow_minute_with_buffer_provider(p, prefs, options),
            None => RelativeTimeFormatter::try_new_narrow_minute(prefs, options),
        },
        ("hour", "long") => match dp.as_buffer_provider() {
            Some(p) => RelativeTimeFormatter::try_new_long_hour_with_buffer_provider(p, prefs, options),
            None => RelativeTimeFormatter::try_new_long_hour(prefs, options),
        },
        ("hour", "short") => match dp.as_buffer_provider() {
            Some(p) => RelativeTimeFormatter::try_new_short_hour_with_buffer_provider(p, prefs, options),
            None => RelativeTimeFormatter::try_new_short_hour(prefs, options),
        },
        ("hour", "narrow") => match dp.as_buffer_provider() {
            Some(p) => RelativeTimeFormatter::try_new_narrow_hour_with_buffer_provider(p, prefs, options),
            None => RelativeTimeFormatter::try_new_narrow_hour(prefs, options),
        },
        ("day", "long") => match dp.as_buffer_provider() {
            Some(p) => RelativeTimeFormatter::try_new_long_day_with_buffer_provider(p, prefs, options),
            None => RelativeTimeFormatter::try_new_long_day(prefs, options),
        },
        ("day", "short") => match dp.as_buffer_provider() {
            Some(p) => RelativeTimeFormatter::try_new_short_day_with_buffer_provider(p, prefs, options),
            None => RelativeTimeFormatter::try_new_short_day(prefs, options),
        },
        ("day", "narrow") => match dp.as_buffer_provider() {
            Some(p) => RelativeTimeFormatter::try_new_narrow_day_with_buffer_provider(p, prefs, options),
            None => RelativeTimeFormatter::try_new_narrow_day(prefs, options),
        },
        ("week", "long") => match dp.as_buffer_provider() {
            Some(p) => RelativeTimeFormatter::try_new_long_week_with_buffer_provider(p, prefs, options),
            None => RelativeTimeFormatter::try_new_long_week(prefs, options),
        },
        ("week", "short") => match dp.as_buffer_provider() {
            Some(p) => RelativeTimeFormatter::try_new_short_week_with_buffer_provider(p, prefs, options),
            None => RelativeTimeFormatter::try_new_short_week(prefs, options),
        },
        ("week", "narrow") => match dp.as_buffer_provider() {
            Some(p) => RelativeTimeFormatter::try_new_narrow_week_with_buffer_provider(p, prefs, options),
            None => RelativeTimeFormatter::try_new_narrow_week(prefs, options),
        },
        ("month", "long") => match dp.as_buffer_provider() {
            Some(p) => RelativeTimeFormatter::try_new_long_month_with_buffer_provider(p, prefs, options),
            None => RelativeTimeFormatter::try_new_long_month(prefs, options),
        },
        ("month", "short") => match dp.as_buffer_provider() {
            Some(p) => RelativeTimeFormatter::try_new_short_month_with_buffer_provider(p, prefs, options),
            None => RelativeTimeFormatter::try_new_short_month(prefs, options),
        },
        ("month", "narrow") => match dp.as_buffer_provider() {
            Some(p) => RelativeTimeFormatter::try_new_narrow_month_with_buffer_provider(p, prefs, options),
            None => RelativeTimeFormatter::try_new_narrow_month(prefs, options),
        },
        ("quarter", "long") => match dp.as_buffer_provider() {
            Some(p) => RelativeTimeFormatter::try_new_long_quarter_with_buffer_provider(p, prefs, options),
            None => RelativeTimeFormatter::try_new_long_quarter(prefs, options),
        },
        ("quarter", "short") => match dp.as_buffer_provider() {
            Some(p) => RelativeTimeFormatter::try_new_short_quarter_with_buffer_provider(p, prefs, options),
            None => RelativeTimeFormatter::try_new_short_quarter(prefs, options),
        },
        ("quarter", "narrow") => match dp.as_buffer_provider() {
            Some(p) => RelativeTimeFormatter::try_new_narrow_quarter_with_buffer_provider(p, prefs, options),
            None => RelativeTimeFormatter::try_new_narrow_quarter(prefs, options),
        },
        ("year", "long") => match dp.as_buffer_provider() {
            Some(p) => RelativeTimeFormatter::try_new_long_year_with_buffer_provider(p, prefs, options),
            None => RelativeTimeFormatter::try_new_long_year(prefs, options),
        },
        ("year", "short") => match dp.as_buffer_provider() {
            Some(p) => RelativeTimeFormatter::try_new_short_year_with_buffer_provider(p, prefs, options),
            None => RelativeTimeFormatter::try_new_short_year(prefs, options),
        },
        ("year", "narrow") => match dp.as_buffer_provider() {
            Some(p) => RelativeTimeFormatter::try_new_narrow_year_with_buffer_provider(p, prefs, options),
            None => RelativeTimeFormatter::try_new_narrow_year(prefs, options),
        },
        _ => return Err(IntlError::InvalidUnit { unit: unit.to_string() }.into_vm_error()),
    };
    result.map_err(|e| IntlError::Formatting { message: e.to_string() }.into_vm_error())
}
