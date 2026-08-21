use ac_runtime::NonFunctionValue;
use ac_vm::{Context, Error, Function};
use icu::datetime::{DateTimeFormatter, fieldsets::{YMD, YMDT}};
use icu::datetime::input::{Date, Time, DateTime};
use writeable::Writeable;

use super::common::{opt_str, optional_locale, optional_object, require_string, s};

/// `intl.format_date(date: string, locale?: string, opts?: object) → string`
///
/// Formats a date or datetime string using ICU4X.
///
/// `date` must be an ISO 8601 string: `"YYYY-MM-DD"` or `"YYYY-MM-DDTHH:MM:SS"`.
///
/// Options:
/// - `style`: `"full"`, `"long"`, `"medium"` (default), or `"short"`
/// - `type`: `"date"` (default) or `"datetime"`
///
/// Examples:
/// - `intl.format_date("2024-05-08")` → `"May 8, 2024"`
/// - `intl.format_date("2024-05-08", "de")` → `"8. Mai 2024"`
/// - `intl.format_date("2024-05-08T14:30:00", "en", {type: "datetime"})` → `"May 8, 2024, 2:30:00 PM"`
#[derive(Clone, Copy)]
pub struct FormatDate;

impl Function for FormatDate {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let date_str = require_string(args, 0)?;
        let locale = optional_locale(args, 1)?;
        let opts = optional_object(args, 2)?;

        let style = opt_str(opts, "style").unwrap_or("medium");
        let date_type = opt_str(opts, "type").unwrap_or("date");

        let formatted = if date_type == "datetime" {
            format_datetime(date_str, &locale, style)?
        } else {
            format_date_only(date_str, &locale, style)?
        };

        Ok(s(formatted))
    }
}

fn format_date_only(date_str: &str, locale: &icu::locale::Locale, style: &str) -> Result<String, Error> {
    let (year, month, day) = parse_date(date_str)?;
    let date = Date::try_new_iso(year, month, day)
        .map_err(|e| Error::PrimitiveError(format!("invalid date: {e}")))?;

    let fieldset = match style {
        "full" => YMD::long(),
        "long" => YMD::long(),
        "short" => YMD::short(),
        _ => YMD::medium(),
    };

    let formatter = DateTimeFormatter::try_new(locale.into(), fieldset)
        .map_err(|e| Error::PrimitiveError(format!("formatting error: {e}")))?;

    let formatted = formatter.format(&date);
    Ok(formatted.write_to_string().into_owned())
}

fn format_datetime(date_str: &str, locale: &icu::locale::Locale, style: &str) -> Result<String, Error> {
    let (year, month, day, hour, minute, second) = parse_datetime(date_str)?;
    let date = Date::try_new_iso(year, month, day)
        .map_err(|e| Error::PrimitiveError(format!("invalid date: {e}")))?;
    let time = Time::try_new(hour, minute, second, 0)
        .map_err(|e| Error::PrimitiveError(format!("invalid time: {e}")))?;
    let datetime = DateTime { date, time };

    let fieldset = match style {
        "full" => YMDT::long(),
        "long" => YMDT::long(),
        "short" => YMDT::short(),
        _ => YMDT::medium(),
    };

    let formatter = DateTimeFormatter::try_new(locale.into(), fieldset)
        .map_err(|e| Error::PrimitiveError(format!("formatting error: {e}")))?;

    let formatted = formatter.format(&datetime);
    Ok(formatted.write_to_string().into_owned())
}

fn parse_date(s: &str) -> Result<(i32, u8, u8), Error> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 {
        return Err(Error::PrimitiveError(format!("invalid date format: {s}")));
    }
    let year = parts[0].parse::<i32>().map_err(|_| Error::PrimitiveError(format!("invalid year: {}", parts[0])))?;
    let month = parts[1].parse::<u8>().map_err(|_| Error::PrimitiveError(format!("invalid month: {}", parts[1])))?;
    let day = parts[2].parse::<u8>().map_err(|_| Error::PrimitiveError(format!("invalid day: {}", parts[2])))?;
    Ok((year, month, day))
}

fn parse_datetime(s: &str) -> Result<(i32, u8, u8, u8, u8, u8), Error> {
    let (date_part, time_part) = s.split_once('T')
        .ok_or_else(|| Error::PrimitiveError(format!("invalid datetime format (missing T): {s}")))?;
    let (year, month, day) = parse_date(date_part)?;
    let time_parts: Vec<&str> = time_part.split(':').collect();
    if time_parts.len() < 2 {
        return Err(Error::PrimitiveError(format!("invalid time format: {time_part}")));
    }
    let hour = time_parts[0].parse::<u8>().map_err(|_| Error::PrimitiveError(format!("invalid hour: {}", time_parts[0])))?;
    let minute = time_parts[1].parse::<u8>().map_err(|_| Error::PrimitiveError(format!("invalid minute: {}", time_parts[1])))?;
    let second = time_parts.get(2).and_then(|s| s.parse::<u8>().ok()).unwrap_or(0);
    Ok((year, month, day, hour, minute, second))
}
