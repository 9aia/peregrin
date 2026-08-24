use std::fmt;

use ecma402_traits::date_time_format::DateTimeFormat as DateTimeFormatTrait;
pub use ecma402_traits::date_time_format::{
    DateTimeFieldStyle, DateTimeFormatOptions, DateTimeFormatPart, DateTimeFormatPartType,
    DateTimeStyle, DateType, HourCycle, TimeZoneNameStyle,
};
use icu::calendar::cal::Iso;
use icu::datetime::fieldsets::builder::{DateFields, FieldSetBuilder, ZoneStyle};
use icu::datetime::input::{Date, DateTime, Time, ZonedDateTime};
use icu::datetime::options::{Length, SubsecondDigits, TimePrecision, YearStyle};
use icu::datetime::preferences::{
    CalendarAlgorithm, HijriCalendarAlgorithm, HourCycle as IcuHourCycle,
};
use icu::datetime::{DateTimeFormatter, fieldsets::T, fieldsets::YMD, fieldsets::YMDT};
use icu::locale::Locale;
use icu::time::zone::{TimeZone, UtcOffset};
use icu_provider::DataError;
use icu_provider_blob::BlobDataProvider;
use writeable::{Part, PartsWrite, Writeable};

/// ICU4X-backed ECMA-402 `Intl.DateTimeFormat`.
pub struct DateTimeFormat<'a> {
    locale: Locale,
    options: DateTimeFormatOptions,
    provider: DataProvider<'a>,
}

enum DataProvider<'a> {
    Compiled,
    Blob(&'a BlobDataProvider),
}

impl<'a> DataProvider<'a> {
    fn as_buffer_provider(&self) -> Option<&'a BlobDataProvider> {
        match self {
            Self::Compiled => None,
            Self::Blob(p) => Some(p),
        }
    }
}

/// Errors that can occur when constructing or using a `DateTimeFormat`.
#[derive(Debug)]
pub enum Error {
    InvalidLocale(String),
    InvalidDate(String),
    Data(DataError),
    Formatting(String),
    InvalidArgument(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLocale(s) => write!(f, "invalid locale: {s}"),
            Self::InvalidDate(s) => write!(f, "invalid date: {s}"),
            Self::Data(e) => write!(f, "data provider error: {e}"),
            Self::Formatting(s) => write!(f, "formatting error: {s}"),
            Self::InvalidArgument(s) => write!(f, "invalid argument: {s}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<DataError> for Error {
    fn from(e: DataError) -> Self {
        Self::Data(e)
    }
}

impl<'a> DateTimeFormat<'a> {
    /// Create a new `DateTimeFormat` for the given BCP 47 locale tag using compiled CLDR data.
    pub fn new(
        locale: &str,
        options: DateTimeFormatOptions,
    ) -> Result<DateTimeFormat<'static>, Error> {
        let mut locale = parse_locale(locale)?;
        apply_numbering_system(&mut locale, &options);

        Ok(DateTimeFormat {
            locale,
            options,
            provider: DataProvider::Compiled,
        })
    }

    /// Create a new `DateTimeFormat` using an ICU4X blob data provider.
    pub fn with_blob(
        locale: &str,
        options: DateTimeFormatOptions,
        provider: &'a BlobDataProvider,
    ) -> Result<Self, Error> {
        let mut locale = parse_locale(locale)?;
        apply_numbering_system(&mut locale, &options);

        Ok(DateTimeFormat {
            locale,
            options,
            provider: DataProvider::Blob(provider),
        })
    }

    fn date_style(&self) -> DateTimeStyle {
        self.options.date_style.unwrap_or(DateTimeStyle::Medium)
    }

    fn time_style(&self) -> DateTimeStyle {
        self.options.time_style.unwrap_or(DateTimeStyle::Medium)
    }

    fn prefs(&self) -> icu::datetime::DateTimeFormatterPreferences {
        build_prefs(&self.locale, &self.options)
    }

    fn time_precision(&self) -> Option<TimePrecision> {
        parse_time_precision(&self.options)
    }

    fn format_value(&self, value: &str) -> Result<String, Error> {
        let parsed = parse_input(value, &self.options)?;
        let mut sink = PartsCollector::new();
        self.format_parsed_to_parts(&parsed, &mut sink)?;
        Ok(sink.into_string())
    }

    fn format_value_to_parts(&self, value: &str) -> Result<Vec<DateTimeFormatPart>, Error> {
        let parsed = parse_input(value, &self.options)?;
        let mut sink = PartsCollector::new();
        self.format_parsed_to_parts(&parsed, &mut sink)?;
        Ok(sink.into_parts())
    }

    fn format_parsed_to_parts(
        &self,
        parsed: &Parsed,
        sink: &mut PartsCollector,
    ) -> Result<(), Error> {
        if has_field_bag(&self.options) {
            format_with_field_bag_to_parts(parsed, self, sink)
        } else {
            match self.options.date_type {
                DateType::DateTime => format_datetime_to_parts(parsed, self, sink),
                DateType::Time => format_time_only_to_parts(parsed, self, sink),
                DateType::Date => format_date_only_to_parts(parsed, self, sink),
            }
        }
    }
}

impl<'a> DateTimeFormatTrait for DateTimeFormat<'a> {
    type Error = Error;

    fn format(&self, value: &str) -> Result<String, Self::Error> {
        self.format_value(value)
    }

    fn format_to_parts(&self, value: &str) -> Result<Vec<DateTimeFormatPart>, Self::Error> {
        self.format_value_to_parts(value)
    }
}

fn parse_locale(locale: &str) -> Result<Locale, Error> {
    Locale::try_from_str(locale).map_err(|_| Error::InvalidLocale(locale.to_string()))
}

fn apply_numbering_system(locale: &mut Locale, options: &DateTimeFormatOptions) {
    if let Some(nu) = &options.numbering_system
        && let Ok(value) = icu::locale::extensions::unicode::Value::try_from_str(nu)
    {
        locale
            .extensions
            .unicode
            .keywords
            .set(icu::locale::extensions::unicode::key!("nu"), value);
    }
}

#[derive(Clone)]
struct Parsed {
    date: Date<Iso>,
    time: Time,
    offset: Option<UtcOffset>,
}

fn parse_input(value: &str, _options: &DateTimeFormatOptions) -> Result<Parsed, Error> {
    let s = value.trim();
    if s.is_empty() {
        return Err(Error::InvalidDate(s.to_string()));
    }

    if is_numeric_timestamp(s) {
        return parse_numeric_timestamp(s);
    }

    if s.contains('T') {
        let (year, month, day, hour, minute, second) = parse_datetime(s)?;
        let nanos = parse_nanos_from_str(s);
        let date =
            Date::try_new_iso(year, month, day).map_err(|e| Error::InvalidDate(e.to_string()))?;
        let time = Time::try_new(hour, minute, second, nanos)
            .map_err(|e| Error::InvalidDate(e.to_string()))?;
        let offset = parse_timezone_offset(s);
        return Ok(Parsed { date, time, offset });
    }

    if s.contains('-') && !s.contains(':') {
        let (year, month, day) = parse_date(s)?;
        let date =
            Date::try_new_iso(year, month, day).map_err(|e| Error::InvalidDate(e.to_string()))?;
        let time = Time::try_new(0, 0, 0, 0).map_err(|e| Error::InvalidDate(e.to_string()))?;
        return Ok(Parsed {
            date,
            time,
            offset: None,
        });
    }

    if s.contains(':') {
        let (hour, minute, second) = parse_time_only(s)?;
        let nanos = parse_nanos_from_str(s);
        let date = Date::try_new_iso(1970, 1, 1).map_err(|e| Error::InvalidDate(e.to_string()))?;
        let time = Time::try_new(hour, minute, second, nanos)
            .map_err(|e| Error::InvalidDate(e.to_string()))?;
        return Ok(Parsed {
            date,
            time,
            offset: parse_timezone_offset(s),
        });
    }

    Err(Error::InvalidDate(s.to_string()))
}

fn is_numeric_timestamp(s: &str) -> bool {
    let s = s.trim();
    if s.is_empty() {
        return false;
    }
    let mut chars = s.chars().peekable();
    if let Some(&c) = chars.peek()
        && (c == '-' || c == '+')
    {
        chars.next();
    }
    let mut has_dot = false;
    let mut has_digit = false;
    for c in chars {
        if c == '.' {
            if has_dot {
                return false;
            }
            has_dot = true;
        } else if c.is_ascii_digit() {
            has_digit = true;
        } else {
            return false;
        }
    }
    has_digit
}

fn parse_numeric_timestamp(s: &str) -> Result<Parsed, Error> {
    let value: f64 = s
        .trim()
        .parse()
        .map_err(|_| Error::InvalidDate(s.to_string()))?;
    let millis = if value.abs() >= 1e10 {
        value as i64
    } else {
        (value * 1000.0) as i64
    };
    let offset = UtcOffset::zero();
    let zdt = ZonedDateTime::from_epoch_milliseconds_and_utc_offset(millis, offset);
    Ok(Parsed {
        date: zdt.date,
        time: zdt.time,
        offset: Some(offset),
    })
}

fn parse_date(s: &str) -> Result<(i32, u8, u8), Error> {
    let s = s.trim();
    if s.is_empty() {
        return Err(Error::InvalidDate(s.to_string()));
    }
    let (sign, rest) = if let Some(rest) = s.strip_prefix('-') {
        (-1i32, rest)
    } else {
        (1i32, s)
    };
    let parts: Vec<&str> = rest.split('-').collect();
    if parts.len() != 3 {
        return Err(Error::InvalidDate(s.to_string()));
    }
    let year = parts[0]
        .parse::<i32>()
        .map_err(|_| Error::InvalidDate(format!("invalid year: {}", parts[0])))?;
    let month = parts[1]
        .parse::<u8>()
        .map_err(|_| Error::InvalidDate(format!("invalid month: {}", parts[1])))?;
    let day = parts[2]
        .parse::<u8>()
        .map_err(|_| Error::InvalidDate(format!("invalid day: {}", parts[2])))?;
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
        return Err(Error::InvalidDate(format!("invalid time: {time_part}")));
    }
    let hour = parts[0]
        .parse::<u8>()
        .map_err(|_| Error::InvalidDate(format!("invalid hour: {}", parts[0])))?;
    let minute = parts[1]
        .parse::<u8>()
        .map_err(|_| Error::InvalidDate(format!("invalid minute: {}", parts[1])))?;
    let second = parts.get(2).and_then(|s| s.parse::<u8>().ok()).unwrap_or(0);
    Ok((hour, minute, second))
}

fn parse_datetime(s: &str) -> Result<(i32, u8, u8, u8, u8, u8), Error> {
    let s = s.trim();
    let (date_part, time_part) = s
        .split_once('T')
        .ok_or_else(|| Error::InvalidDate(format!("invalid datetime (missing T): {s}")))?;
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

fn parse_timezone_offset(s: &str) -> Option<UtcOffset> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    // The offset follows the time-of-day component, which is the whole string
    // for a standalone time or the part after 'T' for a date-time.
    let time_part = if let Some((_, t)) = s.split_once('T') {
        t
    } else if s.contains(':') {
        s
    } else {
        return None;
    };
    let offset_start = time_part
        .find(['+', '-', 'Z', 'z'])
        .unwrap_or(time_part.len());
    if offset_start == time_part.len() {
        return None;
    }
    let candidate = &time_part[offset_start..];
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

fn parse_length(style: DateTimeStyle) -> Length {
    match style {
        DateTimeStyle::Full | DateTimeStyle::Long => Length::Long,
        DateTimeStyle::Short => Length::Short,
        DateTimeStyle::Medium => Length::Medium,
    }
}

fn date_fields_for_style(style: DateTimeStyle) -> DateFields {
    match style {
        // ECMA-402 "full" includes the weekday; the other date styles do not.
        DateTimeStyle::Full => DateFields::YMDE,
        _ => DateFields::YMD,
    }
}

fn parse_time_zone_id(options: &DateTimeFormatOptions) -> TimeZone {
    let Some(tz) = options.time_zone.as_deref() else {
        return TimeZone::UNKNOWN;
    };
    let tz = tz.trim();
    if tz.is_empty() {
        return TimeZone::UNKNOWN;
    }
    TimeZone::from_iana_id(tz)
}

fn build_prefs(
    locale: &Locale,
    options: &DateTimeFormatOptions,
) -> icu::datetime::DateTimeFormatterPreferences {
    let mut prefs: icu::datetime::DateTimeFormatterPreferences = locale.into();
    if let Some(hc) = options.hour_cycle {
        prefs.hour_cycle = Some(match hc {
            HourCycle::H11 => IcuHourCycle::H11,
            HourCycle::H12 => IcuHourCycle::H12,
            HourCycle::H23 => IcuHourCycle::H23,
            HourCycle::H24 => IcuHourCycle::Clock24,
        });
    } else if let Some(hour12) = options.hour12 {
        prefs.hour_cycle = if hour12 {
            Some(IcuHourCycle::H12)
        } else {
            Some(IcuHourCycle::H23)
        };
    }
    if let Some(cal) = &options.calendar {
        prefs.calendar_algorithm = match cal.as_str() {
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

fn has_field_bag(options: &DateTimeFormatOptions) -> bool {
    // time_zone_name and fractional_second_digits are treated as modifiers
    // that can ride along with date_style/time_style. day_period is included
    // here because ICU4X FieldSetBuilder does not expose it for rendering.
    options.year.is_some()
        || options.month.is_some()
        || options.day.is_some()
        || options.weekday.is_some()
        || options.hour.is_some()
        || options.minute.is_some()
        || options.second.is_some()
        || options.era.is_some()
        || options.day_period.is_some()
}

fn determine_date_fields(options: &DateTimeFormatOptions) -> Option<DateFields> {
    let has_year = options.year.is_some();
    let has_month = options.month.is_some();
    let has_day = options.day.is_some();
    let has_weekday = options.weekday.is_some();

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

fn determine_time_precision(options: &DateTimeFormatOptions) -> Option<TimePrecision> {
    if options.hour.is_none()
        && options.minute.is_none()
        && options.second.is_none()
        && options.fractional_second_digits.is_none()
        && options.day_period.is_none()
    {
        return None;
    }
    if let Some(sd) = parse_subsecond_digits(options) {
        return Some(TimePrecision::Subsecond(sd));
    }
    if options.second.is_some() {
        Some(TimePrecision::Second)
    } else if options.minute.is_some() {
        Some(TimePrecision::Minute)
    } else {
        Some(TimePrecision::Hour)
    }
}

fn determine_year_style(options: &DateTimeFormatOptions) -> Option<YearStyle> {
    let year_opt = options.year;
    let era_opt = options.era;
    match (year_opt, era_opt) {
        (Some(DateTimeFieldStyle::TwoDigit), _) => Some(YearStyle::Auto),
        (
            Some(_),
            Some(DateTimeFieldStyle::Narrow | DateTimeFieldStyle::Short | DateTimeFieldStyle::Long),
        ) => Some(YearStyle::WithEra),
        (Some(_), _) => Some(YearStyle::Full),
        (
            None,
            Some(DateTimeFieldStyle::Narrow | DateTimeFieldStyle::Short | DateTimeFieldStyle::Long),
        ) => Some(YearStyle::WithEra),
        _ => None,
    }
}

fn determine_zone_style(options: &DateTimeFormatOptions) -> Option<ZoneStyle> {
    Some(match options.time_zone_name? {
        TimeZoneNameStyle::Short => ZoneStyle::SpecificShort,
        TimeZoneNameStyle::Long => ZoneStyle::SpecificLong,
        TimeZoneNameStyle::ShortOffset => ZoneStyle::LocalizedOffsetShort,
        TimeZoneNameStyle::LongOffset => ZoneStyle::LocalizedOffsetLong,
        TimeZoneNameStyle::ShortGeneric => ZoneStyle::GenericShort,
        TimeZoneNameStyle::LongGeneric => ZoneStyle::GenericLong,
    })
}

fn derive_length_from_fields(options: &DateTimeFormatOptions, style: DateTimeStyle) -> Length {
    let named_fields = [options.month, options.weekday, options.era];
    if named_fields
        .iter()
        .any(|v| matches!(v, Some(DateTimeFieldStyle::Long)))
    {
        Length::Long
    } else if named_fields
        .iter()
        .any(|v| matches!(v, Some(DateTimeFieldStyle::Short)))
    {
        Length::Medium
    } else if named_fields
        .iter()
        .any(|v| matches!(v, Some(DateTimeFieldStyle::Narrow)))
    {
        Length::Short
    } else {
        parse_length(style)
    }
}

fn parse_subsecond_digits(options: &DateTimeFormatOptions) -> Option<SubsecondDigits> {
    match options.fractional_second_digits {
        Some(1) => Some(SubsecondDigits::S1),
        Some(2) => Some(SubsecondDigits::S2),
        Some(3) => Some(SubsecondDigits::S3),
        _ => None,
    }
}

fn parse_time_precision(options: &DateTimeFormatOptions) -> Option<TimePrecision> {
    parse_subsecond_digits(options).map(TimePrecision::Subsecond)
}

fn format_date_only_to_parts(
    parsed: &Parsed,
    dtf: &DateTimeFormat,
    sink: &mut PartsCollector,
) -> Result<(), Error> {
    let fieldset = match dtf.date_style() {
        DateTimeStyle::Full | DateTimeStyle::Long => YMD::long(),
        DateTimeStyle::Short => YMD::short(),
        DateTimeStyle::Medium => YMD::medium(),
    };

    let formatter = match dtf.provider.as_buffer_provider() {
        Some(p) => DateTimeFormatter::try_new_with_buffer_provider(p, dtf.prefs(), fieldset),
        None => DateTimeFormatter::try_new(dtf.prefs(), fieldset),
    }
    .map_err(|e| Error::Formatting(e.to_string()))?;

    let formatted = formatter.format(&parsed.date);
    formatted
        .write_to_parts(sink)
        .map_err(|e| Error::Formatting(e.to_string()))
}

fn format_time_only_to_parts(
    parsed: &Parsed,
    dtf: &DateTimeFormat,
    sink: &mut PartsCollector,
) -> Result<(), Error> {
    let time_precision = dtf
        .time_precision()
        .unwrap_or_else(|| match dtf.time_style() {
            DateTimeStyle::Short => TimePrecision::Minute,
            _ => TimePrecision::Second,
        });
    let fieldset = match dtf.time_style() {
        DateTimeStyle::Full | DateTimeStyle::Long => T::long(),
        DateTimeStyle::Short => T::short(),
        DateTimeStyle::Medium => T::medium(),
    }
    .with_time_precision(time_precision);

    if let Some(zs) = determine_zone_style(&dtf.options) {
        let mut builder = FieldSetBuilder::new();
        builder.length = Some(parse_length(dtf.time_style()));
        builder.time_precision = Some(time_precision);
        builder.zone_style = Some(zs);
        let composite = builder
            .build_composite()
            .map_err(|e| Error::Formatting(e.to_string()))?;

        let formatter = match dtf.provider.as_buffer_provider() {
            Some(p) => DateTimeFormatter::try_new_with_buffer_provider(p, dtf.prefs(), composite),
            None => DateTimeFormatter::try_new(dtf.prefs(), composite),
        }
        .map_err(|e| Error::Formatting(e.to_string()))?;

        let offset = parsed.offset.unwrap_or(UtcOffset::zero());
        let tz = parse_time_zone_id(&dtf.options);
        let zone = tz.with_offset(Some(offset)).at_date_time(DateTime {
            date: parsed.date,
            time: parsed.time,
        });
        let zoned = ZonedDateTime {
            date: parsed.date,
            time: parsed.time,
            zone,
        };
        let formatted = formatter.format(&zoned);
        formatted
            .write_to_parts(sink)
            .map_err(|e| Error::Formatting(e.to_string()))
    } else {
        let formatter = match dtf.provider.as_buffer_provider() {
            Some(p) => DateTimeFormatter::try_new_with_buffer_provider(p, dtf.prefs(), fieldset),
            None => DateTimeFormatter::try_new(dtf.prefs(), fieldset),
        }
        .map_err(|e| Error::Formatting(e.to_string()))?;

        let formatted = formatter.format(&parsed.time);
        formatted
            .write_to_parts(sink)
            .map_err(|e| Error::Formatting(e.to_string()))
    }
}

fn format_datetime_to_parts(
    parsed: &Parsed,
    dtf: &DateTimeFormat,
    sink: &mut PartsCollector,
) -> Result<(), Error> {
    let date = parsed.date;
    let time = parsed.time;
    let datetime = DateTime { date, time };

    let time_precision = dtf
        .time_precision()
        .unwrap_or_else(|| match dtf.time_style() {
            DateTimeStyle::Short => TimePrecision::Minute,
            _ => TimePrecision::Second,
        });

    let fieldset = match dtf.date_style() {
        DateTimeStyle::Full | DateTimeStyle::Long => YMDT::long(),
        DateTimeStyle::Short => YMDT::short(),
        DateTimeStyle::Medium => YMDT::medium(),
    }
    .with_time_precision(time_precision);

    if let Some(zs) = determine_zone_style(&dtf.options) {
        let mut builder = FieldSetBuilder::new();
        builder.length = Some(parse_length(dtf.date_style()));
        builder.date_fields = Some(date_fields_for_style(dtf.date_style()));
        builder.time_precision = Some(time_precision);
        builder.zone_style = Some(zs);
        let composite = builder
            .build_composite()
            .map_err(|e| Error::Formatting(e.to_string()))?;

        let formatter = match dtf.provider.as_buffer_provider() {
            Some(p) => DateTimeFormatter::try_new_with_buffer_provider(p, dtf.prefs(), composite),
            None => DateTimeFormatter::try_new(dtf.prefs(), composite),
        }
        .map_err(|e| Error::Formatting(e.to_string()))?;

        let offset = parsed.offset.unwrap_or(UtcOffset::zero());
        let tz = parse_time_zone_id(&dtf.options);
        let zone = tz
            .with_offset(Some(offset))
            .at_date_time(DateTime { date, time });
        let zoned = ZonedDateTime { date, time, zone };
        let formatted = formatter.format(&zoned);
        formatted
            .write_to_parts(sink)
            .map_err(|e| Error::Formatting(e.to_string()))
    } else {
        let formatter = match dtf.provider.as_buffer_provider() {
            Some(p) => DateTimeFormatter::try_new_with_buffer_provider(p, dtf.prefs(), fieldset),
            None => DateTimeFormatter::try_new(dtf.prefs(), fieldset),
        }
        .map_err(|e| Error::Formatting(e.to_string()))?;

        let formatted = formatter.format(&datetime);
        formatted
            .write_to_parts(sink)
            .map_err(|e| Error::Formatting(e.to_string()))
    }
}

fn format_with_field_bag_to_parts(
    parsed: &Parsed,
    dtf: &DateTimeFormat,
    sink: &mut PartsCollector,
) -> Result<(), Error> {
    let style = dtf.date_style();
    let length = derive_length_from_fields(&dtf.options, style);

    let date_fields = determine_date_fields(&dtf.options);
    let time_precision = determine_time_precision(&dtf.options);
    let year_style = determine_year_style(&dtf.options);
    let zone_style = determine_zone_style(&dtf.options);

    let prefs = dtf.prefs();

    if let Some(zs) = zone_style {
        let mut builder = FieldSetBuilder::new();
        // TODO: ICU4X 2's FieldSetBuilder does not expose a `day_period` field.
        // The `day_period` option is parsed but cannot be rendered in this backend.
        builder.length = Some(length);
        builder.date_fields = date_fields;
        builder.time_precision = time_precision;
        builder.year_style = year_style;
        builder.zone_style = Some(zs);

        let composite = builder
            .build_composite()
            .map_err(|e| Error::Formatting(e.to_string()))?;

        let formatter = match dtf.provider.as_buffer_provider() {
            Some(p) => DateTimeFormatter::try_new_with_buffer_provider(p, prefs, composite),
            None => DateTimeFormatter::try_new(prefs, composite),
        }
        .map_err(|e| Error::Formatting(e.to_string()))?;

        let offset = parsed.offset.unwrap_or(UtcOffset::zero());
        let date = parsed.date;
        let time = parsed.time;
        let tz = parse_time_zone_id(&dtf.options);
        let zone = tz
            .with_offset(Some(offset))
            .at_date_time(DateTime { date, time });
        let zoned = ZonedDateTime { date, time, zone };
        let formatted = formatter.format(&zoned);
        formatted
            .write_to_parts(sink)
            .map_err(|e| Error::Formatting(e.to_string()))?;
    } else {
        let mut builder = FieldSetBuilder::new();
        // TODO: ICU4X 2's FieldSetBuilder does not expose a `day_period` field.
        // The `day_period` option is parsed but cannot be rendered in this backend.
        builder.length = Some(length);
        builder.date_fields = date_fields;
        builder.time_precision = time_precision;
        builder.year_style = year_style;

        let composite = builder
            .build_composite_datetime()
            .map_err(|e| Error::Formatting(e.to_string()))?;

        let formatter = match dtf.provider.as_buffer_provider() {
            Some(p) => DateTimeFormatter::try_new_with_buffer_provider(p, prefs, composite),
            None => DateTimeFormatter::try_new(prefs, composite),
        }
        .map_err(|e| Error::Formatting(e.to_string()))?;

        let date = parsed.date;
        let time = parsed.time;
        let datetime = DateTime { date, time };
        let formatted = formatter.format(&datetime);
        formatted
            .write_to_parts(sink)
            .map_err(|e| Error::Formatting(e.to_string()))?;
    }
    Ok(())
}

/// A `PartsWrite` implementation that collects formatted output into
/// `DateTimeFormatPart` values.
struct PartsCollector {
    parts: Vec<DateTimeFormatPart>,
    buf: String,
    current_part: Part,
}

impl PartsCollector {
    fn new() -> Self {
        Self {
            parts: Vec::new(),
            buf: String::new(),
            current_part: Part {
                category: "",
                value: "literal",
            },
        }
    }

    fn flush(&mut self) {
        if !self.buf.is_empty() {
            let text = std::mem::take(&mut self.buf);
            self.parts.push(DateTimeFormatPart {
                part_type: DateTimeFormatPartType::from(self.current_part.value),
                value: text,
            });
        }
    }

    fn into_parts(mut self) -> Vec<DateTimeFormatPart> {
        self.flush();
        self.parts
    }

    fn into_string(mut self) -> String {
        self.flush();
        self.parts.iter().map(|p| p.value.as_str()).collect()
    }
}

impl Default for PartsCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Write for PartsCollector {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.buf.push_str(s);
        Ok(())
    }

    fn write_char(&mut self, c: char) -> fmt::Result {
        self.buf.push(c);
        Ok(())
    }
}

impl PartsWrite for PartsCollector {
    type SubPartsWrite = PartsCollector;

    fn with_part(
        &mut self,
        part: Part,
        mut f: impl FnMut(&mut Self::SubPartsWrite) -> fmt::Result,
    ) -> fmt::Result {
        self.flush();
        let prev_part = self.current_part;
        self.current_part = part;
        let result = f(self);
        self.flush();
        self.current_part = prev_part;
        result
    }
}

#[cfg(test)]
#[path = "date_time_format_tests.rs"]
mod date_time_format_tests;