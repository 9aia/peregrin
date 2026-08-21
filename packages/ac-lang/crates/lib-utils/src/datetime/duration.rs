use ac_runtime::NonFunctionValue;
use ac_vm::{Context, Error, Function};

use super::common::{n, parse_iso, require_two_strings};

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
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let (a, b) = require_two_strings(args)?;
        let dt_a = parse_iso(a)?;
        let dt_b = parse_iso(b)?;
        Ok(n((dt_b - dt_a).num_seconds() as f64))
    }
}
