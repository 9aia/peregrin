use ac_runtime_values::NonFunctionValue;
use ac_runtime_values::{Error};
use ac_runtime_values::{Context, Function};

use super::common::{format_iso, parse_iso, require_one_string, s};
use chrono::{Datelike, Duration, Timelike, Utc};

macro_rules! bound_fn {
    ($struct_name:ident, $body:expr) => {
        #[derive(Clone, Copy)]
        pub struct $struct_name;
        impl Function<NonFunctionValue> for $struct_name {
            fn call(
                &self,
                args: &[NonFunctionValue],
                _ctx: &mut dyn Context<NonFunctionValue>,
            ) -> Result<NonFunctionValue, Error> {
                let input = require_one_string(args)?;
                let dt = parse_iso(input)?;
                let result: chrono::DateTime<Utc> = $body(&dt);
                Ok(s(format_iso(&result)))
            }
        }
    };
}

bound_fn!(StartOfDay, |dt: &chrono::DateTime<Utc>| {
    dt.with_time(chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap())
        .unwrap()
});

bound_fn!(EndOfDay, |dt: &chrono::DateTime<Utc>| {
    dt.with_time(chrono::NaiveTime::from_hms_opt(23, 59, 59).unwrap())
        .unwrap()
});

bound_fn!(StartOfHour, |dt: &chrono::DateTime<Utc>| {
    dt.with_minute(0).unwrap().with_second(0).unwrap()
});

bound_fn!(EndOfHour, |dt: &chrono::DateTime<Utc>| {
    dt.with_minute(59).unwrap().with_second(59).unwrap()
});

bound_fn!(StartOfMonth, |dt: &chrono::DateTime<Utc>| {
    let naive = chrono::NaiveDate::from_ymd_opt(dt.year(), dt.month(), 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    chrono::TimeZone::from_utc_datetime(&Utc, &naive)
});

bound_fn!(EndOfMonth, |dt: &chrono::DateTime<Utc>| {
    let next_month = if dt.month() == 12 {
        chrono::NaiveDate::from_ymd_opt(dt.year() + 1, 1, 1)
    } else {
        chrono::NaiveDate::from_ymd_opt(dt.year(), dt.month() + 1, 1)
    };
    let last_day = next_month.unwrap() - Duration::days(1);
    let naive = last_day.and_hms_opt(23, 59, 59).unwrap();
    chrono::TimeZone::from_utc_datetime(&Utc, &naive)
});

bound_fn!(StartOfYear, |dt: &chrono::DateTime<Utc>| {
    let naive = chrono::NaiveDate::from_ymd_opt(dt.year(), 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    chrono::TimeZone::from_utc_datetime(&Utc, &naive)
});

bound_fn!(EndOfYear, |dt: &chrono::DateTime<Utc>| {
    let naive = chrono::NaiveDate::from_ymd_opt(dt.year(), 12, 31)
        .unwrap()
        .and_hms_opt(23, 59, 59)
        .unwrap();
    chrono::TimeZone::from_utc_datetime(&Utc, &naive)
});