use ac_runtime::NonFunctionValue;
use ac_runtime::{Context, Error, Function};

use super::common::{format_iso, parse_iso, require_string_and_number, s};
use crate::DatetimeError;
use chrono::{DateTime, Datelike, Duration, TimeZone, Timelike, Utc};

#[derive(Clone, Copy)]
pub struct AddSeconds;
impl Function for AddSeconds {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let (input, amount) = require_string_and_number(args)?;
        let dt = parse_iso(input)?;
        let secs = amount.trunc() as i64;
        let nanos = ((amount - amount.trunc()) * 1e9) as i64;
        let result = dt + Duration::seconds(secs) + Duration::nanoseconds(nanos);
        Ok(s(format_iso(&result)))
    }
}

#[derive(Clone, Copy)]
pub struct AddMinutes;
impl Function for AddMinutes {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let (input, amount) = require_string_and_number(args)?;
        let dt = parse_iso(input)?;
        let total_secs = amount * 60.0;
        let secs = total_secs.trunc() as i64;
        let nanos = ((total_secs - total_secs.trunc()) * 1e9) as i64;
        let result = dt + Duration::seconds(secs) + Duration::nanoseconds(nanos);
        Ok(s(format_iso(&result)))
    }
}

#[derive(Clone, Copy)]
pub struct AddHours;
impl Function for AddHours {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let (input, amount) = require_string_and_number(args)?;
        let dt = parse_iso(input)?;
        let total_secs = amount * 3600.0;
        let secs = total_secs.trunc() as i64;
        let nanos = ((total_secs - total_secs.trunc()) * 1e9) as i64;
        let result = dt + Duration::seconds(secs) + Duration::nanoseconds(nanos);
        Ok(s(format_iso(&result)))
    }
}

#[derive(Clone, Copy)]
pub struct AddDays;
impl Function for AddDays {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let (input, amount) = require_string_and_number(args)?;
        let dt = parse_iso(input)?;
        let total_secs = amount * 86400.0;
        let secs = total_secs.trunc() as i64;
        let nanos = ((total_secs - total_secs.trunc()) * 1e9) as i64;
        let result = dt + Duration::seconds(secs) + Duration::nanoseconds(nanos);
        Ok(s(format_iso(&result)))
    }
}

#[derive(Clone, Copy)]
pub struct AddWeeks;
impl Function for AddWeeks {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let (input, amount) = require_string_and_number(args)?;
        let dt = parse_iso(input)?;
        let total_secs = amount * 604800.0;
        let secs = total_secs.trunc() as i64;
        let nanos = ((total_secs - total_secs.trunc()) * 1e9) as i64;
        let result = dt + Duration::seconds(secs) + Duration::nanoseconds(nanos);
        Ok(s(format_iso(&result)))
    }
}

fn add_months_to_date(dt: DateTime<Utc>, months: i64) -> Result<DateTime<Utc>, Error> {
    let year = dt.year();
    let month = dt.month() as i64;
    let day = dt.day() as u32;

    let total_months = (year as i64) * 12 + (month - 1) + months;
    let new_year = total_months.div_euclid(12);
    let new_month = total_months.rem_euclid(12) + 1;

    if !((-262145..=262144).contains(&new_year)) {
        return Err(DatetimeError::YearOverflow { year: new_year }.into_vm_error());
    }

    // Clamp the day to the last day of the new month
    let last_day = chrono::NaiveDate::from_ymd_opt(new_year as i32, new_month as u32, 1)
        .map(|d| d + Duration::days(31))
        .and_then(|d| d.with_day(1).map(|d| d - Duration::days(1)))
        .map(|d| d.day())
        .unwrap_or(28);

    let clamped_day = day.min(last_day);

    let naive = chrono::NaiveDate::from_ymd_opt(new_year as i32, new_month as u32, clamped_day)
        .and_then(|d| {
            d.and_hms_opt(dt.time().hour(), dt.time().minute(), dt.time().second())
        })
        .ok_or_else(|| DatetimeError::InvalidDateAfterArithmetic {
            year: new_year,
            month: new_month,
            day: clamped_day}.into_vm_error())?;
    Ok(Utc.from_utc_datetime(&naive))
}

#[derive(Clone, Copy)]
pub struct AddMonths;
impl Function for AddMonths {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let (input, amount) = require_string_and_number(args)?;
        let dt = parse_iso(input)?;
        let result = add_months_to_date(dt, amount as i64)?;
        Ok(s(format_iso(&result)))
    }
}

#[derive(Clone, Copy)]
pub struct AddYears;
impl Function for AddYears {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let (input, amount) = require_string_and_number(args)?;
        let dt = parse_iso(input)?;
        let result = add_months_to_date(dt, (amount as i64) * 12)?;
        Ok(s(format_iso(&result)))
    }
}
