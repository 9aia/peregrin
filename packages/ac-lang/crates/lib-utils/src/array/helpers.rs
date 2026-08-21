use ac_runtime::NonFunctionValue;

pub(crate) use crate::shared::{
    arr, b, bind_param, bind_two_params, is_truthy_value, key_to_string, num,
    require_array, require_function_and_array, require_function_value, require_one_arg,
    require_one_array, require_two_arrays,
};

macro_rules! array_unary_fn {
    ($struct_name:ident, $body:expr) => {
        #[derive(Clone, Copy)]
        pub struct $struct_name;
        impl ac_vm::Function for $struct_name {
            fn call(
                &self,
                args: &[NonFunctionValue],
                _ctx: &mut ac_vm::Context,
            ) -> Result<NonFunctionValue, ac_vm::Error> {
                let input = $crate::shared::require_one_array(args)?;
                Ok($body(input))
            }
        }
    };
}
pub(super) use array_unary_fn;
