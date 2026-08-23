use ac_runtime::NonFunctionValue;
use ac_runtime::{Context, Error, Function};

use super::common::{format_iso, s};
use crate::DatetimeError;
use chrono::{TimeZone, Utc};

fn get_number(args: &[NonFunctionValue], index: usize, default: f64) -> Result<f64, Error> {
    match args.get(index) {
        Some(v) => v.as_number().ok_or(Error::TypeMismatch {
            expected: "number",
            got: "non-number",
        }),
        None => Ok(default),
    }
}

#[derive(Clone, Copy)]
pub struct FromComponents;
impl Function for FromComponents {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 3 {
            return Err(Error::ArgumentCount {
                expected: 3,
                got: args.len(),
            });
        }
        let year = get_number(args, 0, 0.0)? as i32;
        let month = get_number(args, 1, 0.0)? as u32;
        let day = get_number(args, 2, 0.0)? as u32;
        let hour = get_number(args, 3, 0.0)? as u32;
        let minute = get_number(args, 4, 0.0)? as u32;
        let second = get_number(args, 5, 0.0)? as u32;

        let dt = Utc
            .with_ymd_and_hms(year, month, day, hour, minute, second)
            .single()
            .ok_or(
                DatetimeError::InvalidDateComponents {
                    year,
                    month,
                    day,
                    hour,
                    minute,
                    second,
                }
                .into_vm_error(),
            )?;

        Ok(s(format_iso(&dt)))
    }
}
