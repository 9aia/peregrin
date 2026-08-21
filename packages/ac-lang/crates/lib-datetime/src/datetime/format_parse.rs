use ac_runtime::NonFunctionValue;
use ac_vm::{Context, Error, Function};

use super::common::{format_iso, parse_iso, parse_with_format_and_offset, require_one_string, require_string, s};

#[derive(Clone, Copy)]
pub struct Format;
impl Function for Format {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let input = require_one_string(args)?;
        let dt = parse_iso(input)?;
        let result = if args.len() >= 2 {
            let fmt = require_string(&args[1])?;
            dt.format(fmt).to_string()
        } else {
            format_iso(&dt)
        };
        Ok(s(result))
    }
}

#[derive(Clone, Copy)]
pub struct Parse;
impl Function for Parse {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut Context,
    ) -> Result<NonFunctionValue, Error> {
        let input = require_one_string(args)?;
        let dt = if args.len() >= 2 {
            let fmt = require_string(&args[1])?;
            let offset = if args.len() >= 3 {
                Some(require_string(&args[2])?)
            } else {
                None
            };
            parse_with_format_and_offset(input, fmt, offset)?
        } else {
            parse_iso(input)?
        };
        Ok(s(format_iso(&dt)))
    }
}
