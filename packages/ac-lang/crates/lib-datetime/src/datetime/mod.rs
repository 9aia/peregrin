mod arithmetic;
mod bounds;
mod common;
mod compare;
mod components;
mod construct;
mod convert;
mod duration;
mod format_parse;
mod timezone;
mod validate;

pub use arithmetic::{AddDays, AddHours, AddMinutes, AddMonths, AddSeconds, AddWeeks, AddYears};
pub use bounds::{
    EndOfDay, EndOfHour, EndOfMonth, EndOfYear, StartOfDay, StartOfHour, StartOfMonth,
    StartOfYear};
pub use compare::{Compare, IsAfter, IsBefore, IsBetween, IsBetweenInclusive, IsEqual};
pub use components::{
    Day, DayOfYear, Hour, IsLeapYear, IsWeekend, Minute, Month, Quarter, Second, WeekOfYear,
    Weekday, Year};
pub use construct::FromComponents;
pub use convert::{FromUnix, ToUnix};
pub use duration::{DurationBetween, DurationDays, DurationHours, DurationMinutes, DurationSeconds};
pub use format_parse::{Format, Parse};
pub use timezone::{ToTimezone, ToUtc};
pub use validate::IsValid;

use ac_runtime_core::Module;
use ac_vm::Function;

/// The `datetime` module — date and time utility functions.
///
/// Provides functions callable as `datetime.from_unix(ts)`, `datetime.format(d, fmt)`,
/// `datetime.parse(s)`, `datetime.add_days(d, n)`, etc. in AC expressions.
///
/// Datetimes are represented as ISO 8601 strings (`YYYY-MM-DDTHH:MM:SSZ`).
/// Unix timestamps are represented as numbers (seconds since epoch).
#[derive(Clone, Copy)]
pub struct DateTimeModule;

impl Module for DateTimeModule {
    fn name(&self) -> &str {
        "datetime"
    }

    fn function(&self, name: &str) -> Option<&dyn Function> {
        match name {
            // convert
            "from_unix" => Some(&FromUnix),
            "to_unix" => Some(&ToUnix),
            // format / parse
            "format" => Some(&Format),
            "parse" => Some(&Parse),
            // components
            "year" => Some(&Year),
            "month" => Some(&Month),
            "day" => Some(&Day),
            "hour" => Some(&Hour),
            "minute" => Some(&Minute),
            "second" => Some(&Second),
            "weekday" => Some(&Weekday),
            "day_of_year" => Some(&DayOfYear),
            "week_of_year" => Some(&WeekOfYear),
            "quarter" => Some(&Quarter),
            "is_leap_year" => Some(&IsLeapYear),
            "is_weekend" => Some(&IsWeekend),
            // arithmetic
            "add_seconds" => Some(&AddSeconds),
            "add_minutes" => Some(&AddMinutes),
            "add_hours" => Some(&AddHours),
            "add_days" => Some(&AddDays),
            "add_weeks" => Some(&AddWeeks),
            "add_months" => Some(&AddMonths),
            "add_years" => Some(&AddYears),
            // compare
            "compare" => Some(&Compare),
            "is_before" => Some(&IsBefore),
            "is_after" => Some(&IsAfter),
            "is_equal" => Some(&IsEqual),
            "is_between" => Some(&IsBetween),
            "is_between_inclusive" => Some(&IsBetweenInclusive),
            // duration
            "duration_between" => Some(&DurationBetween),
            "duration_seconds" => Some(&DurationSeconds),
            "duration_minutes" => Some(&DurationMinutes),
            "duration_hours" => Some(&DurationHours),
            "duration_days" => Some(&DurationDays),
            // bounds
            "start_of_day" => Some(&StartOfDay),
            "end_of_day" => Some(&EndOfDay),
            "start_of_hour" => Some(&StartOfHour),
            "end_of_hour" => Some(&EndOfHour),
            "start_of_month" => Some(&StartOfMonth),
            "end_of_month" => Some(&EndOfMonth),
            "start_of_year" => Some(&StartOfYear),
            "end_of_year" => Some(&EndOfYear),
            // timezone
            "to_timezone" => Some(&ToTimezone),
            "to_utc" => Some(&ToUtc),
            // construct
            "from_components" => Some(&FromComponents),
            // validate
            "is_valid" => Some(&IsValid),
            _ => None}
    }
}

#[cfg(test)]
mod arithmetic_tests;
#[cfg(test)]
mod bounds_tests;
#[cfg(test)]
mod compare_tests;
#[cfg(test)]
mod components_tests;
#[cfg(test)]
mod construct_tests;
#[cfg(test)]
mod convert_tests;
#[cfg(test)]
mod duration_tests;
#[cfg(test)]
mod format_parse_tests;
#[cfg(test)]
mod timezone_tests;
#[cfg(test)]
mod validate_tests;
