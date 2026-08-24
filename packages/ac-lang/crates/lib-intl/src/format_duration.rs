use ac_runtime_values::NonFunctionValue;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use ac_runtime_values::{Error};
use ac_runtime_values::{Context, Function};
use icu::experimental::duration::{
    Duration, DurationFormatter, DurationSign, ValidatedDurationFormatterOptions,
    options::{
        BaseStyle, DayStyle, DurationFormatterOptions, FieldDisplay, FractionalDigits, HourStyle,
        MicroSecondStyle, MilliSecondStyle, MinuteStyle, MonthStyle, NanoSecondStyle, SecondStyle,
        WeekStyle, YearStyle,
    },
};
use writeable::Writeable;

use super::common::{
    make_provider, opt_bool, opt_int, opt_str, optional_locale, optional_object, require_object, s,
};
use super::error::IntlError;
use super::provider::IntlProvider;

thread_local! {
    static DURATION_FORMATTERS: RefCell<HashMap<(String, String), Rc<DurationFormatter>>> = RefCell::new(HashMap::new());
}

/// `intl.format_duration(duration: object, locale?: string, opts?: object) → string`
///
/// Formats a duration using ICU4X.
///
/// `duration` is an object with any of: `years`, `months`, `weeks`, `days`,
/// `hours`, `minutes`, `seconds`, `milliseconds`, `microseconds`, `nanoseconds`.
///
/// Options:
/// - `style`: `"long"` (default), `"short"`, `"narrow"`, `"digital"`
/// - Per-unit style: `yearsStyle`, `monthsStyle`, `weeksStyle`, `daysStyle`,
///   `hoursStyle`, `minutesStyle`, `secondsStyle`, `millisecondsStyle`,
///   `microsecondsStyle`, `nanosecondsStyle` — each `"long"`, `"short"`,
///   `"narrow"`, and for time units also `"numeric"` / `"2-digit"`
/// - Per-unit display: `yearsDisplay`, `monthsDisplay`, etc. — `"auto"` (default) or `"always"`
/// - `fractionalDigits`: number of fractional digits for sub-second units
///
/// Examples:
/// - `intl.format_duration({hours: 1, minutes: 30})` → `"1 hour, 30 minutes"`
/// - `intl.format_duration({seconds: 90}, "es")` → `"90 segundos"`
/// - `intl.format_duration({hours: 2, minutes: 5}, "en", {style: "short"})` → `"2 hr, 5 min"`
/// - `intl.format_duration({hours: 2}, "en", {hoursStyle: "long"})` → `"2 hours"`
#[derive(Clone)]
pub struct FormatDuration {
    provider: Option<IntlProvider>,
}

impl FormatDuration {
    pub fn new() -> Self {
        Self { provider: None }
    }

    pub fn with_provider(provider: IntlProvider) -> Self {
        Self {
            provider: Some(provider),
        }
    }
}

impl Default for FormatDuration {
    fn default() -> Self {
        Self::new()
    }
}

impl Function<NonFunctionValue> for FormatDuration {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let obj = require_object(args, 0)?;
        let locale = optional_locale(args, 1)?;
        let opts = optional_object(args, 2)?;
        let dp = make_provider(&self.provider)?;

        let style = opt_str(opts, "style").unwrap_or("long");

        let duration = build_duration(obj);

        if duration.years == 0
            && duration.months == 0
            && duration.weeks == 0
            && duration.days == 0
            && duration.hours == 0
            && duration.minutes == 0
            && duration.seconds == 0
            && duration.milliseconds == 0
            && duration.microseconds == 0
            && duration.nanoseconds == 0
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
        apply_per_unit_options(&mut options, opts);

        let validated = ValidatedDurationFormatterOptions::validate(options).map_err(|e| {
            IntlError::Formatting {
                message: format!("invalid options: {e}"),
            }
            .into_vm_error()
        })?;

        if dp.as_buffer_provider().is_some() {
            return Err(IntlError::Formatting {
                message: "format_duration does not support blob data provider yet (ICU4X experimental limitation)".to_string()}.into_vm_error());
        }

        let key = (locale.to_string(), format!("{:?}", options));
        let formatter =
            DURATION_FORMATTERS.with(|cache| -> Result<Rc<DurationFormatter>, Error> {
                let mut cache = cache.borrow_mut();
                if let Some(f) = cache.get(&key) {
                    Ok(Rc::clone(f))
                } else {
                    let f = DurationFormatter::try_new(locale.into(), validated).map_err(|e| {
                        IntlError::Formatting {
                            message: e.to_string(),
                        }
                        .into_vm_error()
                    })?;
                    let f = Rc::new(f);
                    cache.insert(key, Rc::clone(&f));
                    Ok(f)
                }
            })?;

        let formatted = formatter.format(&duration);
        Ok(s(formatted.write_to_string().into_owned()))
    }
}

fn build_duration(obj: &serde_json::Map<String, serde_json::Value>) -> Duration {
    let get = |key: &str| -> u64 { obj.get(key).and_then(|v| v.as_u64()).unwrap_or(0) };

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

fn parse_field_display(
    opts: Option<&serde_json::Map<String, serde_json::Value>>,
    key: &str,
) -> Option<FieldDisplay> {
    match opt_str(opts, key) {
        Some("always") => Some(FieldDisplay::Always),
        Some("auto") => Some(FieldDisplay::Auto),
        _ => None,
    }
}

fn apply_per_unit_options(
    options: &mut DurationFormatterOptions,
    opts: Option<&serde_json::Map<String, serde_json::Value>>,
) {
    if let Some(s) = opt_str(opts, "yearsStyle") {
        options.year = match s {
            "narrow" => Some(YearStyle::Narrow),
            "short" => Some(YearStyle::Short),
            "long" => Some(YearStyle::Long),
            _ => None,
        };
    }
    options.year_visibility = parse_field_display(opts, "yearsDisplay");

    if let Some(s) = opt_str(opts, "monthsStyle") {
        options.month = match s {
            "narrow" => Some(MonthStyle::Narrow),
            "short" => Some(MonthStyle::Short),
            "long" => Some(MonthStyle::Long),
            _ => None,
        };
    }
    options.month_visibility = parse_field_display(opts, "monthsDisplay");

    if let Some(s) = opt_str(opts, "weeksStyle") {
        options.week = match s {
            "narrow" => Some(WeekStyle::Narrow),
            "short" => Some(WeekStyle::Short),
            "long" => Some(WeekStyle::Long),
            _ => None,
        };
    }
    options.week_visibility = parse_field_display(opts, "weeksDisplay");

    if let Some(s) = opt_str(opts, "daysStyle") {
        options.day = match s {
            "narrow" => Some(DayStyle::Narrow),
            "short" => Some(DayStyle::Short),
            "long" => Some(DayStyle::Long),
            _ => None,
        };
    }
    options.day_visibility = parse_field_display(opts, "daysDisplay");

    if let Some(s) = opt_str(opts, "hoursStyle") {
        options.hour = match s {
            "narrow" => Some(HourStyle::Narrow),
            "short" => Some(HourStyle::Short),
            "long" => Some(HourStyle::Long),
            "numeric" => Some(HourStyle::Numeric),
            "2-digit" => Some(HourStyle::TwoDigit),
            _ => None,
        };
    }
    options.hour_visibility = parse_field_display(opts, "hoursDisplay");

    if let Some(s) = opt_str(opts, "minutesStyle") {
        options.minute = match s {
            "narrow" => Some(MinuteStyle::Narrow),
            "short" => Some(MinuteStyle::Short),
            "long" => Some(MinuteStyle::Long),
            "numeric" => Some(MinuteStyle::Numeric),
            "2-digit" => Some(MinuteStyle::TwoDigit),
            _ => None,
        };
    }
    options.minute_visibility = parse_field_display(opts, "minutesDisplay");

    if let Some(s) = opt_str(opts, "secondsStyle") {
        options.second = match s {
            "narrow" => Some(SecondStyle::Narrow),
            "short" => Some(SecondStyle::Short),
            "long" => Some(SecondStyle::Long),
            "numeric" => Some(SecondStyle::Numeric),
            "2-digit" => Some(SecondStyle::TwoDigit),
            _ => None,
        };
    }
    options.second_visibility = parse_field_display(opts, "secondsDisplay");

    if let Some(s) = opt_str(opts, "millisecondsStyle") {
        options.millisecond = match s {
            "narrow" => Some(MilliSecondStyle::Narrow),
            "short" => Some(MilliSecondStyle::Short),
            "long" => Some(MilliSecondStyle::Long),
            "numeric" => Some(MilliSecondStyle::Numeric),
            _ => None,
        };
    }
    options.millisecond_visibility = parse_field_display(opts, "millisecondsDisplay");

    if let Some(s) = opt_str(opts, "microsecondsStyle") {
        options.microsecond = match s {
            "narrow" => Some(MicroSecondStyle::Narrow),
            "short" => Some(MicroSecondStyle::Short),
            "long" => Some(MicroSecondStyle::Long),
            "numeric" => Some(MicroSecondStyle::Numeric),
            _ => None,
        };
    }
    options.microsecond_visibility = parse_field_display(opts, "microsecondsDisplay");

    if let Some(s) = opt_str(opts, "nanosecondsStyle") {
        options.nanosecond = match s {
            "narrow" => Some(NanoSecondStyle::Narrow),
            "short" => Some(NanoSecondStyle::Short),
            "long" => Some(NanoSecondStyle::Long),
            "numeric" => Some(NanoSecondStyle::Numeric),
            _ => None,
        };
    }
    options.nanosecond_visibility = parse_field_display(opts, "nanosecondsDisplay");

    if let Some(n) = opt_int(opts, "fractionalDigits")
        && (0..=9).contains(&n)
    {
        options.fractional_digits = FractionalDigits::Fixed(n as u8);
    }
}