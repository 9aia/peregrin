use ac_runtime::NonFunctionValue;
use ac_vm::{Context, Error, Function};

use super::common::{parse_iso, require_string_and_number, s};
use chrono::{DateTime, Datelike, Duration, TimeZone, Timelike, Utc};

#[derive(Clone, Copy)]
pub struct AddSeconds;
impl Function for AddSeconds {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let (input, amount) = require_string_and_number(args)?;
        let dt = parse_iso(input)?;
        let result = dt + Duration::seconds(amount as i64)
            + Duration::nanoseconds(((amount - amount.floor()) * 1e9) as i64);
        Ok(s(result.format("%Y-%m-%dT%H:%M:%SZ").to_string()))
    }
}

#[derive(Clone, Copy)]
pub struct AddMinutes;
impl Function for AddMinutes {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let (input, amount) = require_string_and_number(args)?;
        let dt = parse_iso(input)?;
        let result = dt + Duration::minutes(amount as i64);
        Ok(s(result.format("%Y-%m-%dT%H:%M:%SZ").to_string()))
    }
}

#[derive(Clone, Copy)]
pub struct AddHours;
impl Function for AddHours {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let (input, amount) = require_string_and_number(args)?;
        let dt = parse_iso(input)?;
        let result = dt + Duration::hours(amount as i64);
        Ok(s(result.format("%Y-%m-%dT%H:%M:%SZ").to_string()))
    }
}

#[derive(Clone, Copy)]
pub struct AddDays;
impl Function for AddDays {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let (input, amount) = require_string_and_number(args)?;
        let dt = parse_iso(input)?;
        let result = dt + Duration::days(amount as i64);
        Ok(s(result.format("%Y-%m-%dT%H:%M:%SZ").to_string()))
    }
}

#[derive(Clone, Copy)]
pub struct AddWeeks;
impl Function for AddWeeks {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let (input, amount) = require_string_and_number(args)?;
        let dt = parse_iso(input)?;
        let result = dt + Duration::weeks(amount as i64);
        Ok(s(result.format("%Y-%m-%dT%H:%M:%SZ").to_string()))
    }
}

fn add_months_to_date(dt: DateTime<Utc>, months: i64) -> DateTime<Utc> {
    let year = dt.year();
    let month = dt.month() as i64;
    let day = dt.day() as u32;

    let total_months = (year as i64) * 12 + (month - 1) + months;
    let new_year = total_months.div_euclid(12);
    let new_month = total_months.rem_euclid(12) + 1;

    // Clamp the day to the last day of the new month
    let last_day = chrono::NaiveDate::from_ymd_opt(
        new_year as i32,
        new_month as u32,
        1,
    )
    .map(|d| d + Duration::days(31))
    .and_then(|d| d.with_day(1).map(|d| d - Duration::days(1)))
    .map(|d| d.day())
    .unwrap_or(28);

    let clamped_day = day.min(last_day);

    let naive = chrono::NaiveDate::from_ymd_opt(new_year as i32, new_month as u32, clamped_day)
        .unwrap()
        .and_hms_opt(dt.time().hour(), dt.time().minute(), dt.time().second())
        .unwrap();
    Utc.from_utc_datetime(&naive)
}

#[derive(Clone, Copy)]
pub struct AddMonths;
impl Function for AddMonths {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let (input, amount) = require_string_and_number(args)?;
        let dt = parse_iso(input)?;
        let result = add_months_to_date(dt, amount as i64);
        Ok(s(result.format("%Y-%m-%dT%H:%M:%SZ").to_string()))
    }
}

#[derive(Clone, Copy)]
pub struct AddYears;
impl Function for AddYears {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let (input, amount) = require_string_and_number(args)?;
        let dt = parse_iso(input)?;
        let result = add_months_to_date(dt, (amount as i64) * 12);
        Ok(s(result.format("%Y-%m-%dT%H:%M:%SZ").to_string()))
    }
}
