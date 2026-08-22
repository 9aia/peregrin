use ac_runtime::NonFunctionValue;
use ac_vm::{Context, Error, Function};
use icu::datetime::{DateTimeFormatter, fieldsets::{YMD, YMDT, T}};
use icu::datetime::input::{Date, Time, DateTime};
use writeable::Writeable;

use super::common::{make_provider, opt_str, optional_locale, optional_object, require_string, s, DataProvider};
use super::error::IntlError;
use super::provider::IntlProvider;

/// `intl.format_date(date: string, locale?: string, opts?: object) → string`
///
/// Formats a date, time, or datetime string using ICU4X.
///
/// `date` must be an ISO 8601 string: `"YYYY-MM-DD"`, `"HH:MM:SS"`,
/// or `"YYYY-MM-DDTHH:MM:SS"`. Timezone offsets (e.g. `+05:30`, `Z`)
/// and fractional seconds are parsed but ignored (ISO calendar only).
///
/// Options:
/// - `style`: `"full"`, `"long"`, `"medium"` (default), or `"short"`
/// - `type`: `"date"` (default), `"datetime"`, or `"time"`
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
            "datetime" => format_datetime(date_str, &locale, style, &dp)?,
            "time" => format_time_only(date_str, &locale, style, &dp)?,
            _ => format_date_only(date_str, &locale, style, &dp)?,
        };

        Ok(s(formatted))
    }
}

fn format_date_only(date_str: &str, locale: &icu::locale::Locale, style: &str, dp: &DataProvider) -> Result<String, Error> {
    let (year, month, day) = parse_date(date_str)?;
    let date = Date::try_new_iso(year, month, day)
        .map_err(|e| IntlError::InvalidDate { input: e.to_string() }.into_vm_error())?;

    let fieldset = match style {
        "full" | "long" => YMD::long(),
        "short" => YMD::short(),
        _ => YMD::medium(),
    };

    let formatter = match dp.as_buffer_provider() {
        Some(p) => DateTimeFormatter::try_new_with_buffer_provider(p, locale.into(), fieldset),
        None => DateTimeFormatter::try_new(locale.into(), fieldset),
    }
    .map_err(|e| IntlError::Formatting { message: e.to_string() }.into_vm_error())?;

    let formatted = formatter.format(&date);
    Ok(formatted.write_to_string().into_owned())
}

fn format_time_only(time_str: &str, locale: &icu::locale::Locale, style: &str, dp: &DataProvider) -> Result<String, Error> {
    let (hour, minute, second) = parse_time_only(time_str)?;
    let time = Time::try_new(hour, minute, second, 0)
        .map_err(|e| IntlError::InvalidDate { input: e.to_string() }.into_vm_error())?;

    let fieldset = match style {
        "full" | "long" => T::hms(),
        "short" => T::hm(),
        _ => T::hms(),
    };

    let formatter = match dp.as_buffer_provider() {
        Some(p) => DateTimeFormatter::try_new_with_buffer_provider(p, locale.into(), fieldset),
        None => DateTimeFormatter::try_new(locale.into(), fieldset),
    }
    .map_err(|e| IntlError::Formatting { message: e.to_string() }.into_vm_error())?;

    let formatted = formatter.format(&time);
    Ok(formatted.write_to_string().into_owned())
}

fn format_datetime(date_str: &str, locale: &icu::locale::Locale, style: &str, dp: &DataProvider) -> Result<String, Error> {
    let (year, month, day, hour, minute, second) = parse_datetime(date_str)?;
    let date = Date::try_new_iso(year, month, day)
        .map_err(|e| IntlError::InvalidDate { input: e.to_string() }.into_vm_error())?;
    let time = Time::try_new(hour, minute, second, 0)
        .map_err(|e| IntlError::InvalidDate { input: e.to_string() }.into_vm_error())?;
    let datetime = DateTime { date, time };

    let fieldset = match style {
        "full" | "long" => YMDT::long(),
        "short" => YMDT::short(),
        _ => YMDT::medium(),
    };

    let formatter = match dp.as_buffer_provider() {
        Some(p) => DateTimeFormatter::try_new_with_buffer_provider(p, locale.into(), fieldset),
        None => DateTimeFormatter::try_new(locale.into(), fieldset),
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
    if let Some(idx) = s.find(['+', 'Z', 'z']) {
        if idx > 0 {
            return &s[..idx];
        }
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
