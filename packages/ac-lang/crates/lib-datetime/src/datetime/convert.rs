use ac_runtime::NonFunctionValue;
use ac_runtime::{Context, Error, Function};

use super::common::{format_iso, n, parse_iso, require_number, require_one_string, s};
use crate::DatetimeError;

#[derive(Clone, Copy)]
pub struct FromUnix;
impl Function for FromUnix {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let arg = args.first().ok_or(Error::ArgumentCount {
            expected: 1,
            got: 0,
        })?;
        let ts = require_number(arg)?;
        let secs = ts.floor() as i64;
        let nanos = ((ts - ts.floor()) * 1e9) as u32;
        let dt = chrono::DateTime::<chrono::Utc>::from_timestamp(secs, nanos)
            .ok_or(DatetimeError::InvalidUnixTimestamp { value: ts }.into_vm_error())?;
        Ok(s(format_iso(&dt)))
    }
}

#[derive(Clone, Copy)]
pub struct ToUnix;
impl Function for ToUnix {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let input = require_one_string(args)?;
        let dt = parse_iso(input)?;
        let secs = dt.timestamp() as f64;
        let nanos = dt.timestamp_subsec_nanos() as f64 * 1e-9;
        Ok(n(secs + nanos))
    }
}
