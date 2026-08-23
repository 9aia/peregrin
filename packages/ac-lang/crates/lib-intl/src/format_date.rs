use ac_runtime::NonFunctionValue;
use ac_runtime::{Context, Error, Function};
use icu::datetime::fieldsets::builder::{DateFields, FieldSetBuilder, ZoneStyle};
use icu::datetime::input::{Date, DateTime, Time, ZonedDateTime};
use icu::datetime::options::{Length, SubsecondDigits, TimePrecision, YearStyle};
use icu::datetime::preferences::{CalendarAlgorithm, HijriCalendarAlgorithm, HourCycle};
use icu::datetime::{
    DateTimeFormatter,
    fieldsets::{T, YMD, YMDT},
};
use icu::time::zone::{TimeZone, UtcOffset};
use writeable::Writeable;

use super::common::{
    DataProvider, apply_numbering_system, make_provider, opt_bool, opt_int, opt_str,
    optional_locale, optional_object, require_string, s,
};
use super::error::IntlError;
use super::parts::PartsCollector;
use super::provider::IntlProvider;

/// `intl.format_date(date: string, locale?: string, opts?: object) → string`
///
/// Formats a date, time, or datetime string using ICU4X.
///
/// `date` must be an ISO 8601 string: `"YYYY-MM-DD"`, `"HH:MM:SS"`,
/// or `"YYYY-MM-DDTHH:MM:SS"`. Timezone offsets (e.g. `+05:30`, `Z`)
/// are parsed and used when `timeZoneName` is set in the options.
/// Fractional seconds (e.g. `.123`, `.123456`) are extracted and used
/// when `fractionalSecondDigits` is set.
///
/// Options:
/// - `style`: `"full"`, `"long"`, `"medium"` (default), or `"short"`
/// - `type`: `"date"` (default), `"datetime"`, or `"time"`
/// - `hourCycle`: `"h11"`, `"h12"`, `"h23"`, or `"h24"`
/// - `hour12`: boolean — when `true` uses 12-hour cycle, when `false` uses 24-hour. Ignored if `hourCycle` is set.
/// - `calendar`: `"gregory"` (default), `"buddhist"`, `"chinese"`, `"coptic"`, `"ethiopic"`, `"hebrew"`, `"indian"`, `"islamic"`, `"japanese"`, `"persian"`, `"roc"`
/// - `fractionalSecondDigits`: 1, 2, or 3 (number of fractional second digits)
///
/// ECMA-402 individual field options (when any of these are present, they
/// override `style` and `type`):
/// - `year`: `"numeric"`, `"2-digit"`, `"narrow"`, `"short"`, `"long"`
/// - `month`: `"numeric"`, `"2-digit"`, `"narrow"`, `"short"`, `"long"`
/// - `day`: `"numeric"`, `"2-digit"`
/// - `weekday`: `"narrow"`, `"short"`, `"long"`
/// - `hour`: `"numeric"`, `"2-digit"`
/// - `minute`: `"numeric"`, `"2-digit"`
/// - `second`: `"numeric"`, `"2-digit"`
/// - `era`: `"narrow"`, `"short"`, `"long"`
/// - `timeZoneName`: `"short"`, `"long"`, `"shortOffset"`, `"longOffset"`, `"shortGeneric"`, `"longGeneric"`
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
        Self {
            provider: Some(provider),
        }
    }
}

impl Default for FormatDate {
    fn default() -> Self {
        Self::new()
    }
}

impl Function for FormatDate {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let date_str = require_string(args, 0)?;
        let mut locale = optional_locale(args, 1)?;
        let opts = optional_object(args, 2)?;
        apply_numbering_system(&mut locale, opts);
        let dp = make_provider(&self.provider)?;

        let has_field_bag = has_field_bag_options(opts);

        let formatted = if has_field_bag {
            format_with_field_bag(date_str, &locale, opts, &dp)?
        } else {
            let style = opt_str(opts, "style").unwrap_or("medium");
            let date_type = opt_str(opts, "type").unwrap_or("date");
            match date_type {
                "datetime" => format_datetime(date_str, &locale, style, opts, &dp)?,
                "time" => format_time_only(date_str, &locale, style, opts, &dp)?,
                _ => format_date_only(date_str, &locale, style, opts, &dp)?,
            }
        };

        Ok(s(formatted))
    }
}

// ── format_date_to_parts ─────────────────────────────────────────

/// `intl.format_date_to_parts(date: string, locale?: string, opts?: object) → array`
///
/// Like [`FormatDate`], but returns an array of `{ type, value }` objects
/// instead of a flat string. Uses ICU4X `write_to_parts` APIs to produce
/// structured part annotations (e.g. `"year"`, `"month"`, `"day"`,
/// `"hour"`, `"minute"`, `"second"`, `"literal"`).
#[derive(Clone)]
pub struct FormatDateToParts {
    provider: Option<IntlProvider>,
}

impl FormatDateToParts {
    pub fn new() -> Self {
        Self { provider: None }
    }

    pub fn with_provider(provider: IntlProvider) -> Self {
        Self {
            provider: Some(provider),
        }
    }
}

impl Default for FormatDateToParts {
    fn default() -> Self {
        Self::new()
    }
}

impl Function for FormatDateToParts {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let date_str = require_string(args, 0)?;
        let mut locale = optional_locale(args, 1)?;
        let opts = optional_object(args, 2)?;
        apply_numbering_system(&mut locale, opts);
        let dp = make_provider(&self.provider)?;

        let has_field_bag = has_field_bag_options(opts);

        let mut collector = PartsCollector::new();
        if has_field_bag {
            format_with_field_bag_to_parts(date_str, &locale, opts, &dp, &mut collector)?;
        } else {
            let style = opt_str(opts, "style").unwrap_or("medium");
            let date_type = opt_str(opts, "type").unwrap_or("date");
            match date_type {
                "datetime" => {
                    format_datetime_to_parts(date_str, &locale, style, opts, &dp, &mut collector)?
                }
                "time" => {
                    format_time_only_to_parts(date_str, &locale, style, opts, &dp, &mut collector)?
                }
                _ => {
                    format_date_only_to_parts(date_str, &locale, style, opts, &dp, &mut collector)?
                }
            }
        }

        Ok(collector.into_value())
    }
}

fn has_field_bag_options(opts: Option<&serde_json::Map<String, serde_json::Value>>) -> bool {
    let Some(o) = opts else { return false };
    o.contains_key("year")
        || o.contains_key("month")
        || o.contains_key("day")
        || o.contains_key("weekday")
        || o.contains_key("hour")
        || o.contains_key("minute")
        || o.contains_key("second")
        || o.contains_key("era")
        || o.contains_key("timeZoneName")
}

fn parse_length(style: &str) -> Length {
    match style {
        "full" | "long" => Length::Long,
        "short" => Length::Short,
        _ => Length::Medium,
    }
}

fn determine_date_fields(
    opts: Option<&serde_json::Map<String, serde_json::Value>>,
) -> Option<DateFields> {
    let o = opts?;
    let has_year = o.contains_key("year");
    let has_month = o.contains_key("month");
    let has_day = o.contains_key("day");
    let has_weekday = o.contains_key("weekday");

    match (has_year, has_month, has_day, has_weekday) {
        (true, true, true, true) => Some(DateFields::YMDE),
        (true, true, true, false) => Some(DateFields::YMD),
        (false, true, true, true) => Some(DateFields::MDE),
        (false, true, true, false) => Some(DateFields::MD),
        (false, false, true, true) => Some(DateFields::DE),
        (false, false, true, false) => Some(DateFields::D),
        (false, true, false, true) => Some(DateFields::MDE),
        (false, true, false, false) => Some(DateFields::M),
        (true, true, false, _) => Some(DateFields::YM),
        (true, false, _, _) => Some(DateFields::Y),
        (false, false, false, true) => Some(DateFields::E),
        _ => None,
    }
}

fn determine_time_precision(
    opts: Option<&serde_json::Map<String, serde_json::Value>>,
) -> Option<TimePrecision> {
    let o = opts?;
    if !o.contains_key("hour")
        && !o.contains_key("minute")
        && !o.contains_key("second")
        && !o.contains_key("fractionalSecondDigits")
    {
        return None;
    }
    if let Some(sd) = parse_subsecond_digits(opts) {
        return Some(TimePrecision::Subsecond(sd));
    }
    if o.contains_key("second") {
        Some(TimePrecision::Second)
    } else if o.contains_key("minute") {
        Some(TimePrecision::Minute)
    } else {
        Some(TimePrecision::Hour)
    }
}

fn determine_year_style(
    opts: Option<&serde_json::Map<String, serde_json::Value>>,
) -> Option<YearStyle> {
    let year_opt = opt_str(opts, "year");
    let era_opt = opt_str(opts, "era");
    match (year_opt, era_opt) {
        (Some("2-digit"), _) => Some(YearStyle::Auto),
        (Some(_), Some("narrow" | "short" | "long")) => Some(YearStyle::WithEra),
        (Some(_), _) => Some(YearStyle::Full),
        (None, Some("narrow" | "short" | "long")) => Some(YearStyle::WithEra),
        _ => None,
    }
}

fn determine_zone_style(
    opts: Option<&serde_json::Map<String, serde_json::Value>>,
) -> Option<ZoneStyle> {
    let tz_name = opt_str(opts, "timeZoneName")?;
    match tz_name {
        "short" => Some(ZoneStyle::SpecificShort),
        "long" => Some(ZoneStyle::SpecificLong),
        "shortOffset" => Some(ZoneStyle::LocalizedOffsetShort),
        "longOffset" => Some(ZoneStyle::LocalizedOffsetLong),
        "shortGeneric" => Some(ZoneStyle::GenericShort),
        "longGeneric" => Some(ZoneStyle::GenericLong),
        _ => Some(ZoneStyle::SpecificShort),
    }
}

fn format_with_field_bag(
    date_str: &str,
    locale: &icu::locale::Locale,
    opts: Option<&serde_json::Map<String, serde_json::Value>>,
    dp: &DataProvider,
) -> Result<String, Error> {
    let mut collector = PartsCollector::new();
    format_with_field_bag_to_parts(date_str, locale, opts, dp, &mut collector)?;
    let parts = collector.into_parts();
    Ok(parts.iter().map(|(_, v)| v.as_str()).collect())
}

fn format_with_field_bag_to_parts(
    date_str: &str,
    locale: &icu::locale::Locale,
    opts: Option<&serde_json::Map<String, serde_json::Value>>,
    dp: &DataProvider,
    sink: &mut PartsCollector,
) -> Result<(), Error> {
    let style = opt_str(opts, "style").unwrap_or("medium");
    let length = derive_length_from_fields(opts, style);

    let date_fields = determine_date_fields(opts);
    let time_precision = determine_time_precision(opts);
    let year_style = determine_year_style(opts);
    let zone_style = determine_zone_style(opts);

    let (year, month, day, hour, minute, second) = parse_datetime_flexible(date_str)?;
    let nanos = parse_nanos_from_str(date_str);
    let date = Date::try_new_iso(year, month, day).map_err(|e| {
        IntlError::InvalidDate {
            input: e.to_string(),
        }
        .into_vm_error()
    })?;
    let time = Time::try_new(hour, minute, second, nanos).map_err(|e| {
        IntlError::InvalidDate {
            input: e.to_string(),
        }
        .into_vm_error()
    })?;

    let prefs = build_prefs(locale, opts);

    if let Some(zs) = zone_style {
        let mut builder = FieldSetBuilder::new();
        builder.length = Some(length);
        builder.date_fields = date_fields;
        builder.time_precision = time_precision;
        builder.year_style = year_style;
        builder.zone_style = Some(zs);

        let composite = builder.build_composite().map_err(|e| {
            IntlError::Formatting {
                message: e.to_string(),
            }
            .into_vm_error()
        })?;

        let formatter = match dp.as_buffer_provider() {
            Some(p) => DateTimeFormatter::try_new_with_buffer_provider(p, prefs, composite),
            None => DateTimeFormatter::try_new(prefs, composite),
        }
        .map_err(|e| {
            IntlError::Formatting {
                message: e.to_string(),
            }
            .into_vm_error()
        })?;

        let offset = parse_timezone_offset(date_str).unwrap_or(UtcOffset::zero());
        let zone = TimeZone::UNKNOWN
            .with_offset(Some(offset))
            .at_date_time(DateTime { date, time });
        let zoned = ZonedDateTime { date, time, zone };
        let formatted = formatter.format(&zoned);
        formatted.write_to_parts(sink).map_err(|e| {
            IntlError::Formatting {
                message: e.to_string(),
            }
            .into_vm_error()
        })?;
    } else {
        let mut builder = FieldSetBuilder::new();
        builder.length = Some(length);
        builder.date_fields = date_fields;
        builder.time_precision = time_precision;
        builder.year_style = year_style;

        let composite = builder.build_composite_datetime().map_err(|e| {
            IntlError::Formatting {
                message: e.to_string(),
            }
            .into_vm_error()
        })?;

        let formatter = match dp.as_buffer_provider() {
            Some(p) => DateTimeFormatter::try_new_with_buffer_provider(p, prefs, composite),
            None => DateTimeFormatter::try_new(prefs, composite),
        }
        .map_err(|e| {
            IntlError::Formatting {
                message: e.to_string(),
            }
            .into_vm_error()
        })?;

        let datetime = DateTime { date, time };
        let formatted = formatter.format(&datetime);
        formatted.write_to_parts(sink).map_err(|e| {
            IntlError::Formatting {
                message: e.to_string(),
            }
            .into_vm_error()
        })?;
    }
    Ok(())
}

fn derive_length_from_fields(
    opts: Option<&serde_json::Map<String, serde_json::Value>>,
    style: &str,
) -> Length {
    let Some(_) = opts else {
        return parse_length(style);
    };
    let named_fields = [
        opt_str(opts, "month"),
        opt_str(opts, "weekday"),
        opt_str(opts, "era"),
    ];
    if named_fields
        .iter()
        .any(|v| matches!(v, Some("long") | Some("full")))
    {
        Length::Long
    } else if named_fields.iter().any(|v| matches!(v, Some("short"))) {
        Length::Medium
    } else if named_fields.iter().any(|v| matches!(v, Some("narrow"))) {
        Length::Short
    } else {
        parse_length(style)
    }
}

fn parse_datetime_flexible(s: &str) -> Result<(i32, u8, u8, u8, u8, u8), Error> {
    let s = s.trim();
    if s.contains('T') {
        parse_datetime(s)
    } else if s.contains('-') && !s.contains(':') {
        let (year, month, day) = parse_date(s)?;
        Ok((year, month, day, 0, 0, 0))
    } else if s.contains(':') {
        let (hour, minute, second) = parse_time_only(s)?;
        Ok((1970, 1, 1, hour, minute, second))
    } else {
        Err(IntlError::InvalidDate {
            input: s.to_string(),
        }
        .into_vm_error())
    }
}

fn build_prefs(
    locale: &icu::locale::Locale,
    opts: Option<&serde_json::Map<String, serde_json::Value>>,
) -> icu::datetime::DateTimeFormatterPreferences {
    let mut prefs: icu::datetime::DateTimeFormatterPreferences = locale.into();
    if let Some(hc) = opt_str(opts, "hourCycle") {
        prefs.hour_cycle = match hc {
            "h11" => Some(HourCycle::H11),
            "h12" => Some(HourCycle::H12),
            "h23" => Some(HourCycle::H23),
            "h24" => Some(HourCycle::Clock24),
            _ => None,
        };
    } else if let Some(hour12) = opt_bool(opts, "hour12") {
        prefs.hour_cycle = if hour12 {
            Some(HourCycle::H12)
        } else {
            Some(HourCycle::H23)
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
            "islamic-civil" => Some(CalendarAlgorithm::Hijri(Some(
                HijriCalendarAlgorithm::Civil,
            ))),
            "islamic-rgsa" => Some(CalendarAlgorithm::Hijri(Some(HijriCalendarAlgorithm::Rgsa))),
            "islamic-tbla" => Some(CalendarAlgorithm::Hijri(Some(HijriCalendarAlgorithm::Tbla))),
            "islamic-umalqura" => Some(CalendarAlgorithm::Hijri(Some(
                HijriCalendarAlgorithm::Umalqura,
            ))),
            "japanese" => Some(CalendarAlgorithm::Japanese),
            "persian" => Some(CalendarAlgorithm::Persian),
            "roc" => Some(CalendarAlgorithm::Roc),
            _ => None,
        };
    }
    prefs
}

fn parse_subsecond_digits(
    opts: Option<&serde_json::Map<String, serde_json::Value>>,
) -> Option<SubsecondDigits> {
    match opt_int(opts, "fractionalSecondDigits") {
        Some(1) => Some(SubsecondDigits::S1),
        Some(2) => Some(SubsecondDigits::S2),
        Some(3) => Some(SubsecondDigits::S3),
        _ => None,
    }
}

fn parse_time_precision(
    opts: Option<&serde_json::Map<String, serde_json::Value>>,
) -> Option<TimePrecision> {
    parse_subsecond_digits(opts).map(TimePrecision::Subsecond)
}

fn parse_nanos_from_str(s: &str) -> u32 {
    if let Some(idx) = s.find('.') {
        let frac_part = &s[idx + 1..];
        let frac_part = frac_part.split(['+', 'Z', 'z']).next().unwrap_or(frac_part);
        let digits: String = frac_part
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if digits.is_empty() {
            return 0;
        }
        let padded = format!("{digits:0<9}");
        padded[..9].parse::<u32>().unwrap_or(0)
    } else {
        0
    }
}

fn format_date_only(
    date_str: &str,
    locale: &icu::locale::Locale,
    style: &str,
    opts: Option<&serde_json::Map<String, serde_json::Value>>,
    dp: &DataProvider,
) -> Result<String, Error> {
    let (year, month, day) = parse_date(date_str)?;
    let date = Date::try_new_iso(year, month, day).map_err(|e| {
        IntlError::InvalidDate {
            input: e.to_string(),
        }
        .into_vm_error()
    })?;

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
    .map_err(|e| {
        IntlError::Formatting {
            message: e.to_string(),
        }
        .into_vm_error()
    })?;

    let formatted = formatter.format(&date);
    Ok(formatted.write_to_string().into_owned())
}

fn format_date_only_to_parts(
    date_str: &str,
    locale: &icu::locale::Locale,
    style: &str,
    opts: Option<&serde_json::Map<String, serde_json::Value>>,
    dp: &DataProvider,
    sink: &mut PartsCollector,
) -> Result<(), Error> {
    let (year, month, day) = parse_date(date_str)?;
    let date = Date::try_new_iso(year, month, day).map_err(|e| {
        IntlError::InvalidDate {
            input: e.to_string(),
        }
        .into_vm_error()
    })?;

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
    .map_err(|e| {
        IntlError::Formatting {
            message: e.to_string(),
        }
        .into_vm_error()
    })?;

    let formatted = formatter.format(&date);
    formatted.write_to_parts(sink).map_err(|e| {
        IntlError::Formatting {
            message: e.to_string(),
        }
        .into_vm_error()
    })
}

fn format_time_only(
    time_str: &str,
    locale: &icu::locale::Locale,
    style: &str,
    opts: Option<&serde_json::Map<String, serde_json::Value>>,
    dp: &DataProvider,
) -> Result<String, Error> {
    let (hour, minute, second) = parse_time_only(time_str)?;
    let nanos = parse_nanos_from_str(time_str);
    let time = Time::try_new(hour, minute, second, nanos).map_err(|e| {
        IntlError::InvalidDate {
            input: e.to_string(),
        }
        .into_vm_error()
    })?;

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
    .map_err(|e| {
        IntlError::Formatting {
            message: e.to_string(),
        }
        .into_vm_error()
    })?;

    let formatted = formatter.format(&time);
    Ok(formatted.write_to_string().into_owned())
}

fn format_time_only_to_parts(
    time_str: &str,
    locale: &icu::locale::Locale,
    style: &str,
    opts: Option<&serde_json::Map<String, serde_json::Value>>,
    dp: &DataProvider,
    sink: &mut PartsCollector,
) -> Result<(), Error> {
    let (hour, minute, second) = parse_time_only(time_str)?;
    let nanos = parse_nanos_from_str(time_str);
    let time = Time::try_new(hour, minute, second, nanos).map_err(|e| {
        IntlError::InvalidDate {
            input: e.to_string(),
        }
        .into_vm_error()
    })?;

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
    .map_err(|e| {
        IntlError::Formatting {
            message: e.to_string(),
        }
        .into_vm_error()
    })?;

    let formatted = formatter.format(&time);
    formatted.write_to_parts(sink).map_err(|e| {
        IntlError::Formatting {
            message: e.to_string(),
        }
        .into_vm_error()
    })
}

fn format_datetime(
    date_str: &str,
    locale: &icu::locale::Locale,
    style: &str,
    opts: Option<&serde_json::Map<String, serde_json::Value>>,
    dp: &DataProvider,
) -> Result<String, Error> {
    let (year, month, day, hour, minute, second) = parse_datetime(date_str)?;
    let nanos = parse_nanos_from_str(date_str);
    let date = Date::try_new_iso(year, month, day).map_err(|e| {
        IntlError::InvalidDate {
            input: e.to_string(),
        }
        .into_vm_error()
    })?;
    let time = Time::try_new(hour, minute, second, nanos).map_err(|e| {
        IntlError::InvalidDate {
            input: e.to_string(),
        }
        .into_vm_error()
    })?;
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
    .map_err(|e| {
        IntlError::Formatting {
            message: e.to_string(),
        }
        .into_vm_error()
    })?;

    let formatted = formatter.format(&datetime);
    Ok(formatted.write_to_string().into_owned())
}

fn format_datetime_to_parts(
    date_str: &str,
    locale: &icu::locale::Locale,
    style: &str,
    opts: Option<&serde_json::Map<String, serde_json::Value>>,
    dp: &DataProvider,
    sink: &mut PartsCollector,
) -> Result<(), Error> {
    let (year, month, day, hour, minute, second) = parse_datetime(date_str)?;
    let nanos = parse_nanos_from_str(date_str);
    let date = Date::try_new_iso(year, month, day).map_err(|e| {
        IntlError::InvalidDate {
            input: e.to_string(),
        }
        .into_vm_error()
    })?;
    let time = Time::try_new(hour, minute, second, nanos).map_err(|e| {
        IntlError::InvalidDate {
            input: e.to_string(),
        }
        .into_vm_error()
    })?;
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
    .map_err(|e| {
        IntlError::Formatting {
            message: e.to_string(),
        }
        .into_vm_error()
    })?;

    let formatted = formatter.format(&datetime);
    formatted.write_to_parts(sink).map_err(|e| {
        IntlError::Formatting {
            message: e.to_string(),
        }
        .into_vm_error()
    })
}

fn parse_date(s: &str) -> Result<(i32, u8, u8), Error> {
    let s = s.trim();
    if s.is_empty() {
        return Err(IntlError::InvalidDate {
            input: s.to_string(),
        }
        .into_vm_error());
    }
    let (sign, rest) = if let Some(rest) = s.strip_prefix('-') {
        (-1i32, rest)
    } else {
        (1i32, s)
    };
    let parts: Vec<&str> = rest.split('-').collect();
    if parts.len() != 3 {
        return Err(IntlError::InvalidDate {
            input: s.to_string(),
        }
        .into_vm_error());
    }
    let year = parts[0].parse::<i32>().map_err(|_| {
        IntlError::InvalidDate {
            input: format!("invalid year: {}", parts[0]),
        }
        .into_vm_error()
    })?;
    let month = parts[1].parse::<u8>().map_err(|_| {
        IntlError::InvalidDate {
            input: format!("invalid month: {}", parts[1]),
        }
        .into_vm_error()
    })?;
    let day = parts[2].parse::<u8>().map_err(|_| {
        IntlError::InvalidDate {
            input: format!("invalid day: {}", parts[2]),
        }
        .into_vm_error()
    })?;
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
        return Err(IntlError::InvalidDate {
            input: format!("invalid time: {time_part}"),
        }
        .into_vm_error());
    }
    let hour = parts[0].parse::<u8>().map_err(|_| {
        IntlError::InvalidDate {
            input: format!("invalid hour: {}", parts[0]),
        }
        .into_vm_error()
    })?;
    let minute = parts[1].parse::<u8>().map_err(|_| {
        IntlError::InvalidDate {
            input: format!("invalid minute: {}", parts[1]),
        }
        .into_vm_error()
    })?;
    let second = parts.get(2).and_then(|s| s.parse::<u8>().ok()).unwrap_or(0);
    Ok((hour, minute, second))
}

fn parse_datetime(s: &str) -> Result<(i32, u8, u8, u8, u8, u8), Error> {
    let s = s.trim();
    let (date_part, time_part) = s.split_once('T').ok_or_else(|| {
        IntlError::InvalidDate {
            input: format!("invalid datetime (missing T): {s}"),
        }
        .into_vm_error()
    })?;
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

fn parse_timezone_offset(s: &str) -> Option<UtcOffset> {
    let s = s.trim();
    if !s.contains('T') {
        return None;
    }
    let time_part = s.split_once('T').map(|(_, t)| t).unwrap_or(s);
    let after_dot = time_part
        .find('.')
        .map(|i| &time_part[i..])
        .unwrap_or(time_part);
    let after_frac = after_dot.split(['+', 'Z', 'z']).next().unwrap_or(after_dot);
    let end_of_frac = after_frac.len();
    let time_start = s.len() - time_part.len();
    let pos = time_start + end_of_frac;
    if pos >= s.len() {
        return None;
    }
    let candidate = &s[pos..];
    if candidate.starts_with('+')
        || candidate.starts_with('-')
        || candidate.starts_with('Z')
        || candidate.starts_with('z')
    {
        UtcOffset::try_from_str(candidate).ok()
    } else {
        None
    }
}
