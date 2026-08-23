use ac_runtime::NonFunctionValue;
use ac_vm::{Context, Error, Function};

use super::common::{b, require_one_string};

#[derive(Clone, Copy)]
pub struct IsValid;
impl Function for IsValid {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let input = require_one_string(args)?;
        let valid = super::common::parse_iso(input).is_ok();
        Ok(b(valid))
    }
}
