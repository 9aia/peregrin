use ac_runtime::Function;
use ac_runtime_modules::Module;
use ac_runtime_values::NonFunctionValue;

use crate::r#type::coercion::{ToBool, ToNumber, ToString};
use crate::r#type::predicates::{
    IsArray, IsBool, IsEmpty, IsNull, IsNumber, IsObject, IsString, IsTruthy, TypeOf,
};
use crate::r#type::utilities::{Coalesce, Default, Identity};

/// The `type` module — type coercion and predicate functions.
///
/// Provides functions callable as `type.to_string(v)`, `type.to_number(v)`,
/// `type.to_bool(v)`, `type.is_null(v)`, `type.is_bool(v)`, etc. in AC
/// expressions.
///
/// # NaN and Infinity
///
/// `serde_json::Value` cannot represent NaN or Infinity — they serialize as
/// `null`. Values that would be NaN or Infinity are therefore handled by the
/// `null` branch in each function, yielding `"null"`, `0.0`, and `false`
/// respectively. This is a deliberate graceful-degradation choice, not a bug.
#[derive(Clone, Copy)]
pub struct TypeModule;

impl Module for TypeModule {
    fn name(&self) -> &str {
        "type"
    }

    fn function(&self, name: &str) -> Option<&dyn Function<NonFunctionValue>> {
        match name {
            // coercion
            "to_string" => Some(&ToString),
            "to_number" => Some(&ToNumber),
            "to_bool" => Some(&ToBool),
            // predicates
            "is_null" => Some(&IsNull),
            "is_bool" => Some(&IsBool),
            "is_number" => Some(&IsNumber),
            "is_string" => Some(&IsString),
            "is_array" => Some(&IsArray),
            "is_object" => Some(&IsObject),
            "type_of" => Some(&TypeOf),
            "is_empty" => Some(&IsEmpty),
            "is_truthy" => Some(&IsTruthy),
            // utilities
            "identity" => Some(&Identity),
            "default" => Some(&Default),
            "coalesce" => Some(&Coalesce),
            _ => None,
        }
    }
}
