use ac_runtime::NonFunctionValue;
use ac_vm::{Context, Error, Function};

use super::common::{n, parse_iso, require_two_strings};

fn duration_secs_f64(dt_a: chrono::DateTime<chrono::Utc>, dt_b: chrono::DateTime<chrono::Utc>) -> f64 {
    let dur = dt_b - dt_a;
    let ms = dur.num_milliseconds() as f64;
    let sub_ms_nanos = (dur.subsec_nanos()) as f64;
    ms / 1000.0 + sub_ms_nanos / 1e9
}

#[derive(Clone, Copy)]
pub struct DurationBetween;
impl Function for DurationBetween {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let (a, b) = require_two_strings(args)?;
        let dt_a = parse_iso(a)?;
        let dt_b = parse_iso(b)?;
        let secs = duration_secs_f64(dt_a, dt_b);
        let mut map = serde_json::Map::new();
        map.insert("seconds".to_string(), serde_json::json!(secs));
        map.insert("minutes".to_string(), serde_json::json!(secs / 60.0));
        map.insert("hours".to_string(), serde_json::json!(secs / 3600.0));
        map.insert("days".to_string(), serde_json::json!(secs / 86400.0));
        Ok(NonFunctionValue::new(serde_json::Value::Object(map)))
    }
}

#[derive(Clone, Copy)]
pub struct DurationSeconds;
impl Function for DurationSeconds {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let (a, b) = require_two_strings(args)?;
        let dt_a = parse_iso(a)?;
        let dt_b = parse_iso(b)?;
        Ok(n(duration_secs_f64(dt_a, dt_b)))
    }
}

#[derive(Clone, Copy)]
pub struct DurationMinutes;
impl Function for DurationMinutes {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let (a, b) = require_two_strings(args)?;
        let dt_a = parse_iso(a)?;
        let dt_b = parse_iso(b)?;
        Ok(n(duration_secs_f64(dt_a, dt_b) / 60.0))
    }
}

#[derive(Clone, Copy)]
pub struct DurationHours;
impl Function for DurationHours {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let (a, b) = require_two_strings(args)?;
        let dt_a = parse_iso(a)?;
        let dt_b = parse_iso(b)?;
        Ok(n(duration_secs_f64(dt_a, dt_b) / 3600.0))
    }
}

#[derive(Clone, Copy)]
pub struct DurationDays;
impl Function for DurationDays {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let (a, b) = require_two_strings(args)?;
        let dt_a = parse_iso(a)?;
        let dt_b = parse_iso(b)?;
        Ok(n(duration_secs_f64(dt_a, dt_b) / 86400.0))
    }
}
