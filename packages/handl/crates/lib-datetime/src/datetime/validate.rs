use handl_vm_values_json::Error;
use handl_vm_values_json::NonFunctionValue;
use handl_vm_values_json::{Context, Function};

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
