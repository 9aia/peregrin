use ac_runtime::NonFunctionValue;
use ac_vm::{Context, Error, Function};

use super::common::{n, parse_iso, require_number, require_two_strings, s};

#[derive(Clone, Copy)]
pub struct DurationBetween;
impl Function for DurationBetween {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let (a, b) = require_two_strings(args)?;
        let dt_a = parse_iso(a)?;
        let dt_b = parse_iso(b)?;
        let secs = (dt_b - dt_a).num_seconds() as f64;
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
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let (a, b) = require_two_strings(args)?;
        let dt_a = parse_iso(a)?;
        let dt_b = parse_iso(b)?;
        Ok(n((dt_b - dt_a).num_seconds() as f64))
    }
}

#[derive(Clone, Copy)]
pub struct HumanizeDuration;
impl Function for HumanizeDuration {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let arg = args.first().ok_or(Error::ArgumentCount { expected: 1, got: 0 })?;
        let total_secs = require_number(arg)?;

        let negative = total_secs < 0.0;
        let abs_secs = total_secs.abs();

        let days = (abs_secs / 86400.0).floor() as i64;
        let hours = ((abs_secs % 86400.0) / 3600.0).floor() as i64;
        let minutes = ((abs_secs % 3600.0) / 60.0).floor() as i64;
        let seconds = (abs_secs % 60.0).round() as i64;

        let mut parts: Vec<String> = Vec::new();

        if days > 0 {
            parts.push(format!("{} {}", days, if days == 1 { "day" } else { "days" }));
        }
        if hours > 0 {
            parts.push(format!("{} {}", hours, if hours == 1 { "hour" } else { "hours" }));
        }
        if minutes > 0 {
            parts.push(format!("{} {}", minutes, if minutes == 1 { "minute" } else { "minutes" }));
        }
        if seconds > 0 || parts.is_empty() {
            parts.push(format!("{} {}", seconds, if seconds == 1 { "second" } else { "seconds" }));
        }

        let mut result = parts.join(" ");
        if negative {
            result = format!("-{result}");
        }

        Ok(s(result))
    }
}
