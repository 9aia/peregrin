use ac_runtime::NonFunctionValue;
use ac_runtime::{Context, Error, Function};

use super::common::{b, n, parse_iso, require_one_string};
use chrono::{Datelike, Timelike};

macro_rules! component_fn {
    ($struct_name:ident, $extract:expr) => {
        #[derive(Clone, Copy)]
        pub struct $struct_name;
        impl Function for $struct_name {
            fn call(
                &self,
                args: &[NonFunctionValue],
                _ctx: &mut dyn Context,
            ) -> Result<NonFunctionValue, Error> {
                let input = require_one_string(args)?;
                let dt = parse_iso(input)?;
                Ok(n($extract(&dt) as f64))
            }
        }
    };
}

component_fn!(Year, |dt: &chrono::DateTime<chrono::Utc>| dt.year());
component_fn!(Month, |dt: &chrono::DateTime<chrono::Utc>| dt.month());
component_fn!(Day, |dt: &chrono::DateTime<chrono::Utc>| dt.day());
component_fn!(Hour, |dt: &chrono::DateTime<chrono::Utc>| dt.hour());
component_fn!(Minute, |dt: &chrono::DateTime<chrono::Utc>| dt.minute());
component_fn!(Second, |dt: &chrono::DateTime<chrono::Utc>| dt.second());

#[derive(Clone, Copy)]
pub struct Weekday;
impl Function for Weekday {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let input = require_one_string(args)?;
        let dt = parse_iso(input)?;
        // chrono: Sunday = 0, Monday = 1, ..., Saturday = 6
        // We want: Monday = 1, ..., Sunday = 7
        let wday = dt.weekday().num_days_from_monday() + 1;
        Ok(n(wday as f64))
    }
}

#[derive(Clone, Copy)]
pub struct DayOfYear;
impl Function for DayOfYear {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let input = require_one_string(args)?;
        let dt = parse_iso(input)?;
        Ok(n(dt.ordinal() as f64))
    }
}

#[derive(Clone, Copy)]
pub struct WeekOfYear;
impl Function for WeekOfYear {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let input = require_one_string(args)?;
        let dt = parse_iso(input)?;
        Ok(n(dt.iso_week().week() as f64))
    }
}

#[derive(Clone, Copy)]
pub struct Quarter;
impl Function for Quarter {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let input = require_one_string(args)?;
        let dt = parse_iso(input)?;
        let month = dt.month();
        let quarter = (month - 1) / 3 + 1;
        Ok(n(quarter as f64))
    }
}

#[derive(Clone, Copy)]
pub struct IsLeapYear;
impl Function for IsLeapYear {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let input = require_one_string(args)?;
        let dt = parse_iso(input)?;
        let year = dt.year();
        let is_leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
        Ok(b(is_leap))
    }
}

#[derive(Clone, Copy)]
pub struct IsWeekend;
impl Function for IsWeekend {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let input = require_one_string(args)?;
        let dt = parse_iso(input)?;
        // Sunday = 0, Saturday = 6 in chrono's num_days_from_sunday
        let wday = dt.weekday().num_days_from_sunday();
        Ok(b(wday == 0 || wday == 6))
    }
}
