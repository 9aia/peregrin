use ac_runtime::NonFunctionValue;
use ac_vm::{Context, Error, Function};
use icu::experimental::duration::{
    Duration, DurationFormatter, DurationSign,
    options::{DurationFormatterOptions, BaseStyle},
    ValidatedDurationFormatterOptions,
};
use writeable::Writeable;

use super::common::{opt_bool, opt_str, optional_locale, optional_object, require_object, s};
use super::error::IntlError;

/// `intl.format_duration(duration: object, locale?: string, opts?: object) → string`
///
/// Formats a duration using ICU4X.
///
/// `duration` is an object with any of: `years`, `months`, `weeks`, `days`,
/// `hours`, `minutes`, `seconds`, `milliseconds`, `microseconds`, `nanoseconds`.
///
/// Options:
/// - `style`: `"long"` (default), `"short"`, `"narrow"`, `"digital"`
///
/// Examples:
/// - `intl.format_duration({hours: 1, minutes: 30})` → `"1 hour, 30 minutes"`
/// - `intl.format_duration({seconds: 90}, "es")` → `"90 segundos"`
/// - `intl.format_duration({hours: 2, minutes: 5}, "en", {style: "short"})` → `"2 hr, 5 min"`
#[derive(Clone, Copy)]
pub struct FormatDuration;

impl Function for FormatDuration {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let obj = require_object(args, 0)?;
        let locale = optional_locale(args, 1)?;
        let opts = optional_object(args, 2)?;

        let style = opt_str(opts, "style").unwrap_or("long");

        let duration = build_duration(obj);

        if duration.years == 0 && duration.months == 0 && duration.weeks == 0
            && duration.days == 0 && duration.hours == 0 && duration.minutes == 0
            && duration.seconds == 0 && duration.milliseconds == 0
            && duration.microseconds == 0 && duration.nanoseconds == 0
        {
            return Ok(s("0 seconds"));
        }

        let base = match style {
            "short" => BaseStyle::Short,
            "narrow" => BaseStyle::Narrow,
            "digital" => BaseStyle::Digital,
            _ => BaseStyle::Long,
        };

        let mut options = DurationFormatterOptions::default();
        options.base = base;

        let validated = ValidatedDurationFormatterOptions::validate(options)
            .map_err(|e| IntlError::Formatting { message: format!("invalid options: {e}") }.into_vm_error())?;

        let formatter = DurationFormatter::try_new(locale.into(), validated)
            .map_err(|e| IntlError::Formatting { message: e.to_string() }.into_vm_error())?;

        let formatted = formatter.format(&duration);
        Ok(s(formatted.write_to_string().into_owned()))
    }
}

fn build_duration(obj: &serde_json::Map<String, serde_json::Value>) -> Duration {
    let get = |key: &str| -> u64 {
        obj.get(key).and_then(|v| v.as_u64()).unwrap_or(0)
    };

    let sign = if opt_bool(Some(obj), "negative").unwrap_or(false) {
        DurationSign::Negative
    } else {
        DurationSign::Positive
    };

    Duration {
        sign,
        years: get("years"),
        months: get("months"),
        weeks: get("weeks"),
        days: get("days"),
        hours: get("hours"),
        minutes: get("minutes"),
        seconds: get("seconds"),
        milliseconds: get("milliseconds"),
        microseconds: get("microseconds"),
        nanoseconds: get("nanoseconds"),
    }
}
