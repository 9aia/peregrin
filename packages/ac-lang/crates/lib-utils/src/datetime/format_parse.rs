use ac_runtime::NonFunctionValue;
use ac_vm::{Context, Error, Function};

use super::common::{parse_iso, parse_with_format, require_one_string, require_string, s};

#[derive(Clone, Copy)]
pub struct Format;
impl Function for Format {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let input = require_one_string(args)?;
        let dt = parse_iso(input)?;
        let result = if args.len() >= 2 {
            let fmt = require_string(&args[1])?;
            dt.format(fmt).to_string()
        } else {
            dt.format("%Y-%m-%dT%H:%M:%SZ").to_string()
        };
        Ok(s(result))
    }
}

#[derive(Clone, Copy)]
pub struct Parse;
impl Function for Parse {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let input = require_one_string(args)?;
        let dt = if args.len() >= 2 {
            let fmt = require_string(&args[1])?;
            parse_with_format(input, fmt)?
        } else {
            parse_iso(input)?
        };
        Ok(s(dt.format("%Y-%m-%dT%H:%M:%SZ").to_string()))
    }
}
