pub(crate) use crate::shared::{
    arr, b, key_to_string, num, require_array, require_function_and_array, require_function_value,
    require_one_arg, require_one_array, require_two_arrays,
};

macro_rules! array_unary_fn {
    ($struct_name:ident, $body:expr) => {
        #[derive(Clone, Copy)]
        pub struct $struct_name;
        impl handl_vm::Function<handl_vm_values_json::NonFunctionValue> for $struct_name {
            fn call(
                &self,
                args: &[NonFunctionValue],
                _ctx: &mut dyn handl_vm::Context<handl_vm_values_json::NonFunctionValue>,
            ) -> Result<NonFunctionValue, handl_vm_values_json::Error> {
                let input = $crate::shared::require_one_array(args)?;
                Ok($body(input))
            }
        }
    };
}
pub(super) use array_unary_fn;
