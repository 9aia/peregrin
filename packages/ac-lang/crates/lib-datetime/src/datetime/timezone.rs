use ac_runtime::NonFunctionValue;
use ac_runtime::{Context, Error, Function};

use super::common::{format_iso, parse_iso, parse_offset, require_one_string, require_two_strings, s};
use chrono::Utc;

#[derive(Clone, Copy)]
pub struct ToTimezone;
impl Function for ToTimezone {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let (input, offset_str) = require_two_strings(args)?;
        let dt = parse_iso(input)?;
        let offset = parse_offset(offset_str)?;
        let converted = dt.with_timezone(&offset);
        let nanos = dt.timestamp_subsec_nanos();
        if nanos > 0 {
            let base = converted.format("%Y-%m-%dT%H:%M:%S").to_string();
            let frac = format!(".{:09}", nanos);
            let frac_trimmed = frac.trim_end_matches('0');
            Ok(s(format!("{}{}{}", base, frac_trimmed, offset)))
        } else {
            Ok(s(format!("{}{}", converted.format("%Y-%m-%dT%H:%M:%S"), offset)))
        }
    }
}

#[derive(Clone, Copy)]
pub struct ToUtc;
impl Function for ToUtc {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let input = require_one_string(args)?;
        let dt = parse_iso(input)?;
        Ok(s(format_iso(&dt.with_timezone(&Utc))))
    }
}
