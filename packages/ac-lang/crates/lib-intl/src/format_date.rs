use ac_runtime::NonFunctionValue;
use ac_vm::{Context, Error, Function};
use icu::datetime::{DateTimeFormatter, fieldsets::{YMD, YMDT, T}};
use icu::datetime::input::{Date, Time, DateTime};
use icu::datetime::preferences::{CalendarAlgorithm, HijriCalendarAlgorithm, HourCycle};
use icu::datetime::options::{SubsecondDigits, TimePrecision};
use writeable::Writeable;

use super::common::{make_provider, opt_int, opt_str, optional_locale, optional_object, require_string, s, DataProvider};
use super::error::IntlError;
use super::provider::IntlProvider;

/// `intl.format_date(date: string, locale?: string, opts?: object) → string`
///
/// Formats a date, time, or datetime string using ICU4X.
///
/// `date` must be an ISO 8601 string: `"YYYY-MM-DD"`, `"HH:MM:SS"`,
/// or `"YYYY-MM-DDTHH:MM:SS"`. Timezone offsets (e.g. `+05:30`, `Z`)
/// are parsed but ignored (ISO calendar only). Fractional seconds
/// (e.g. `.123`, `.123456`) are extracted and used when
/// `fractionalSecondDigits` is set.
///
/// Options:
/// - `style`: `"full"`, `"long"`, `"medium"` (default), or `"short"`
/// - `type`: `"date"` (default), `"datetime"`, or `"time"`
/// - `hourCycle`: `"h11"`, `"h12"`, `"h23"`, or `"h24"`
/// - `calendar`: `"gregory"` (default), `"buddhist"`, `"chinese"`, `"coptic"`, `"ethiopic"`, `"hebrew"`, `"indian"`, `"islamic"`, `"japanese"`, `"persian"`, `"roc"`
/// - `fractionalSecondDigits`: 1, 2, or 3 (number of fractional second digits)
///
/// Examples:
/// - `intl.format_date("2024-05-08")` → `"May 8, 2024"`
/// - `intl.format_date("2024-05-08", "de")` → `"8. Mai 2024"`
/// - `intl.format_date("2024-05-08T14:30:00", "en", {type: "datetime"})` → `"May 8, 2024, 2:30:00 PM"`
/// - `intl.format_date("14:30:00", "en", {type: "time"})` → `"2:30:00 PM"`
#[derive(Clone)]
pub struct FormatDate {
    provider: Option<IntlProvider>,
}

impl FormatDate {
    pub fn new() -> Self {
        Self { provider: None }
    }

    pub fn with_provider(provider: IntlProvider) -> Self {
        Self { provider: Some(provider) }
    }
}

impl Default for FormatDate {
    fn default() -> Self {
        Self::new()
    }
}

impl Function for FormatDate {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let date_str = require_string(args, 0)?;
        let locale = optional_locale(args, 1)?;
        let opts = optional_object(args, 2)?;
        let dp = make_provider(&self.provider)?;

        let style = opt_str(opts, "style").unwrap_or("medium");
        let date_type = opt_str(opts, "type").unwrap_or("date");

        let formatted = match date_type {
            "datetime" => format_datetime(date_str, &locale, style, opts, &dp)?,
            "time" => format_time_only(date_str, &locale, style, opts, &dp)?,
            _ => format_date_only(date_str, &locale, style, opts, &dp)?,
        };

        Ok(s(formatted))
    }
}

fn build_prefs(locale: &icu::locale::Locale, opts: Option<&serde_json::Map<String, serde_json::Value>>) -> icu::datetime::DateTimeFormatterPreferences {
    let mut prefs: icu::datetime::DateTimeFormatterPreferences = locale.into();
    if let Some(hc) = opt_str(opts, "hourCycle") {
        prefs.hour_cycle = match hc {
            "h11" => Some(HourCycle::H11),
            "h12" => Some(HourCycle::H12),
            "h23" => Some(HourCycle::H23),
            "h24" => Some(HourCycle::Clock24),
            _ => None,
        };
    }
    if let Some(cal) = opt_str(opts, "calendar") {
        prefs.calendar_algorithm = match cal {
            "buddhist" => Some(CalendarAlgorithm::Buddhist),
            "chinese" => Some(CalendarAlgorithm::Chinese),
            "coptic" => Some(CalendarAlgorithm::Coptic),
            "dangi" => Some(CalendarAlgorithm::Dangi),
            "ethiopic" => Some(CalendarAlgorithm::Ethiopic),
            "ethioaa" => Some(CalendarAlgorithm::Ethioaa),
            "hebrew" => Some(CalendarAlgorithm::Hebrew),
            "indian" => Some(CalendarAlgorithm::Indian),
            "islamic" => Some(CalendarAlgorithm::Hijri(None)),
            "islamic-civil" => Some(CalendarAlgorithm::Hijri(Some(HijriCalendarAlgorithm::Civil))),
            "islamic-rgsa" => Some(CalendarAlgorithm::Hijri(Some(HijriCalendarAlgorithm::Rgsa))),
            "islamic-tbla" => Some(CalendarAlgorithm::Hijri(Some(HijriCalendarAlgorithm::Tbla))),
            "islamic-umalqura" => Some(CalendarAlgorithm::Hijri(Some(HijriCalendarAlgorithm::Umalqura))),
            "japanese" => Some(CalendarAlgorithm::Japanese),
            "persian" => Some(CalendarAlgorithm::Persian),
            "roc" => Some(CalendarAlgorithm::Roc),
            _ => None,
        };
    }
    prefs
}

fn parse_subsecond_digits(opts: Option<&serde_json::Map<String, serde_json::Value>>) -> Option<SubsecondDigits> {
    match opt_int(opts, "fractionalSecondDigits") {
        Some(1) => Some(SubsecondDigits::S1),
        Some(2) => Some(SubsecondDigits::S2),
        Some(3) => Some(SubsecondDigits::S3),
        _ => None,
    }
}

fn parse_time_precision(opts: Option<&serde_json::Map<String, serde_json::Value>>) -> Option<TimePrecision> {
    parse_subsecond_digits(opts).map(TimePrecision::Subsecond)
}

fn parse_nanos_from_str(s: &str) -> u32 {
    if let Some(idx) = s.find('.') {
        let frac_part = &s[idx + 1..];
        let frac_part = frac_part.split(['+', 'Z', 'z']).next().unwrap_or(frac_part);
        let digits: String = frac_part.chars().take_while(|c| c.is_ascii_digit()).collect();
        if digits.is_empty() {
            return 0;
        }
        let padded = format!("{digits:0<9}");
        padded[..9].parse::<u32>().unwrap_or(0)
    } else {
        0
    }
}

fn format_date_only(date_str: &str, locale: &icu::locale::Locale, style: &str, opts: Option<&serde_json::Map<String, serde_json::Value>>, dp: &DataProvider) -> Result<String, Error> {
    let (year, month, day) = parse_date(date_str)?;
    let date = Date::try_new_iso(year, month, day)
        .map_err(|e| IntlError::InvalidDate { input: e.to_string() }.into_vm_error())?;

    let fieldset = match style {
        "full" | "long" => YMD::long(),
        "short" => YMD::short(),
        _ => YMD::medium(),
    };

    let prefs = build_prefs(locale, opts);
    let formatter = match dp.as_buffer_provider() {
        Some(p) => DateTimeFormatter::try_new_with_buffer_provider(p, prefs, fieldset),
        None => DateTimeFormatter::try_new(prefs, fieldset),
    }
    .map_err(|e| IntlError::Formatting { message: e.to_string() }.into_vm_error())?;

    let formatted = formatter.format(&date);
    Ok(formatted.write_to_string().into_owned())
}

fn format_time_only(time_str: &str, locale: &icu::locale::Locale, style: &str, opts: Option<&serde_json::Map<String, serde_json::Value>>, dp: &DataProvider) -> Result<String, Error> {
    let (hour, minute, second) = parse_time_only(time_str)?;
    let nanos = parse_nanos_from_str(time_str);
    let time = Time::try_new(hour, minute, second, nanos)
        .map_err(|e| IntlError::InvalidDate { input: e.to_string() }.into_vm_error())?;

    let fieldset = match style {
        "full" | "long" => T::hms(),
        "short" => T::hm(),
        _ => T::hms(),
    };
    let fieldset = if let Some(sd) = parse_subsecond_digits(opts) {
        T::hmss(sd)
    } else {
        fieldset
    };

    let prefs = build_prefs(locale, opts);
    let formatter = match dp.as_buffer_provider() {
        Some(p) => DateTimeFormatter::try_new_with_buffer_provider(p, prefs, fieldset),
        None => DateTimeFormatter::try_new(prefs, fieldset),
    }
    .map_err(|e| IntlError::Formatting { message: e.to_string() }.into_vm_error())?;

    let formatted = formatter.format(&time);
    Ok(formatted.write_to_string().into_owned())
}

fn format_datetime(date_str: &str, locale: &icu::locale::Locale, style: &str, opts: Option<&serde_json::Map<String, serde_json::Value>>, dp: &DataProvider) -> Result<String, Error> {
    let (year, month, day, hour, minute, second) = parse_datetime(date_str)?;
    let nanos = parse_nanos_from_str(date_str);
    let date = Date::try_new_iso(year, month, day)
        .map_err(|e| IntlError::InvalidDate { input: e.to_string() }.into_vm_error())?;
    let time = Time::try_new(hour, minute, second, nanos)
        .map_err(|e| IntlError::InvalidDate { input: e.to_string() }.into_vm_error())?;
    let datetime = DateTime { date, time };

    let fieldset = match style {
        "full" | "long" => YMDT::long(),
        "short" => YMDT::short(),
        _ => YMDT::medium(),
    };
    let fieldset = if let Some(tp) = parse_time_precision(opts) {
        fieldset.with_time_precision(tp)
    } else {
        fieldset
    };
    let prefs = build_prefs(locale, opts);
    let formatter = match dp.as_buffer_provider() {
        Some(p) => DateTimeFormatter::try_new_with_buffer_provider(p, prefs, fieldset),
        None => DateTimeFormatter::try_new(prefs, fieldset),
    }
    .map_err(|e| IntlError::Formatting { message: e.to_string() }.into_vm_error())?;

    let formatted = formatter.format(&datetime);
    Ok(formatted.write_to_string().into_owned())
}

fn parse_date(s: &str) -> Result<(i32, u8, u8), Error> {
    let s = s.trim();
    if s.is_empty() {
        return Err(IntlError::InvalidDate { input: s.to_string() }.into_vm_error());
    }
    let (sign, rest) = if let Some(rest) = s.strip_prefix('-') {
        (-1i32, rest)
    } else {
        (1i32, s)
    };
    let parts: Vec<&str> = rest.split('-').collect();
    if parts.len() != 3 {
        return Err(IntlError::InvalidDate { input: s.to_string() }.into_vm_error());
    }
    let year = parts[0].parse::<i32>().map_err(|_| IntlError::InvalidDate { input: format!("invalid year: {}", parts[0]) }.into_vm_error())?;
    let month = parts[1].parse::<u8>().map_err(|_| IntlError::InvalidDate { input: format!("invalid month: {}", parts[1]) }.into_vm_error())?;
    let day = parts[2].parse::<u8>().map_err(|_| IntlError::InvalidDate { input: format!("invalid day: {}", parts[2]) }.into_vm_error())?;
    Ok((sign * year, month, day))
}

fn parse_time_only(s: &str) -> Result<(u8, u8, u8), Error> {
    let s = s.trim();
    let time_part = if s.contains('T') {
        s.split_once('T').map(|(_, t)| t).unwrap_or(s)
    } else {
        s
    };
    parse_time(time_part)
}

fn parse_time(time_part: &str) -> Result<(u8, u8, u8), Error> {
    let time_part = strip_timezone(time_part);
    let time_part = strip_fractional_seconds(time_part);
    let parts: Vec<&str> = time_part.split(':').collect();
    if parts.len() < 2 {
        return Err(IntlError::InvalidDate { input: format!("invalid time: {time_part}") }.into_vm_error());
    }
    let hour = parts[0].parse::<u8>().map_err(|_| IntlError::InvalidDate { input: format!("invalid hour: {}", parts[0]) }.into_vm_error())?;
    let minute = parts[1].parse::<u8>().map_err(|_| IntlError::InvalidDate { input: format!("invalid minute: {}", parts[1]) }.into_vm_error())?;
    let second = parts.get(2).and_then(|s| s.parse::<u8>().ok()).unwrap_or(0);
    Ok((hour, minute, second))
}

fn parse_datetime(s: &str) -> Result<(i32, u8, u8, u8, u8, u8), Error> {
    let s = s.trim();
    let (date_part, time_part) = s.split_once('T')
        .ok_or_else(|| IntlError::InvalidDate { input: format!("invalid datetime (missing T): {s}") }.into_vm_error())?;
    let (year, month, day) = parse_date(date_part)?;
    let (hour, minute, second) = parse_time(time_part)?;
    Ok((year, month, day, hour, minute, second))
}

fn strip_timezone(s: &str) -> &str {
    if let Some(idx) = s.find(['+', 'Z', 'z'])
        && idx > 0
    {
        return &s[..idx];
    }
    let neg_idx = s[1..].find('-');
    if let Some(idx) = neg_idx {
        return &s[..idx + 1];
    }
    s
}

fn strip_fractional_seconds(s: &str) -> &str {
    if let Some(idx) = s.find('.') {
        &s[..idx]
    } else {
        s
    }
}
