use ac_runtime_values::NonFunctionValue;
use ac_runtime_values::{Error};
use ac_runtime_values::{Context, Function};

use super::common::{b, require_one_string};

#[derive(Clone, Copy)]
pub struct IsValid;
impl Function<NonFunctionValue> for IsValid {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let input = require_one_string(args)?;
        let valid = super::common::parse_iso(input).is_ok();
        Ok(b(valid))
    }
}