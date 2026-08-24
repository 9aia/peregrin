pub(crate) use crate::shared::{
    arr, b, key_to_string, num, require_array, require_function_and_array, require_function_value,
    require_one_arg, require_one_array, require_two_arrays,
};

macro_rules! array_unary_fn {
    ($struct_name:ident, $body:expr) => {
        #[derive(Clone, Copy)]
        pub struct $struct_name;
        impl ac_runtime::Function<ac_runtime_values::NonFunctionValue> for $struct_name {
            fn call(
                &self,
                args: &[NonFunctionValue],
                _ctx: &mut dyn ac_runtime::Context<ac_runtime_values::NonFunctionValue>,
            ) -> Result<NonFunctionValue, ac_runtime_values::Error> {
                let input = $crate::shared::require_one_array(args)?;
                Ok($body(input))
            }
        }
    };
}
pub(super) use array_unary_fn;
