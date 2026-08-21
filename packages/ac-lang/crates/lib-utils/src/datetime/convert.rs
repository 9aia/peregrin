use ac_runtime::NonFunctionValue;
use ac_vm::{Context, Error, Function};

use super::common::{n, parse_iso, require_number, require_one_string, s};

#[derive(Clone, Copy)]
pub struct Now;
impl Function for Now {
    fn call(
        &self,
        _args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        Ok(n(
            Utc::now().timestamp() as f64 + Utc::now().timestamp_subsec_nanos() as f64 * 1e-9
        ))
    }
}

#[derive(Clone, Copy)]
pub struct UtcNow;
impl Function for UtcNow {
    fn call(
        &self,
        _args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        Ok(s(Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()))
    }
}

#[derive(Clone, Copy)]
pub struct FromUnix;
impl Function for FromUnix {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let arg = args.first().ok_or(Error::ArgumentCount {
            expected: 1,
            got: 0,
        })?;
        let ts = require_number(arg)?;
        let secs = ts.floor() as i64;
        let nanos = ((ts - ts.floor()) * 1e9) as u32;
        let dt = chrono::DateTime::<chrono::Utc>::from_timestamp(secs, nanos).ok_or(
            Error::PrimitiveError(format!("invalid unix timestamp: {ts}")),
        )?;
        Ok(s(dt.format("%Y-%m-%dT%H:%M:%SZ").to_string()))
    }
}

#[derive(Clone, Copy)]
pub struct ToUnix;
impl Function for ToUnix {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let input = require_one_string(args)?;
        let dt = parse_iso(input)?;
        Ok(n(dt.timestamp() as f64))
    }
}

use chrono::Utc;
