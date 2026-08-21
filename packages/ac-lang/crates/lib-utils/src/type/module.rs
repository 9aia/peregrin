use ac_runtime_core::Module;
use ac_vm::Function;

use crate::r#type::coercion::{ToBool, ToNumber, ToString};
use crate::r#type::predicates::{IsArray, IsBool, IsEmpty, IsNull, IsNumber, IsObject, IsString, IsTruthy, TypeOf};
use crate::r#type::utilities::{Coalesce, Default, Identity};

/// The `type` module — type coercion and predicate functions.
///
/// Provides functions callable as `type.to_string(v)`, `type.to_number(v)`,
/// `type.to_bool(v)`, `type.is_null(v)`, `type.is_bool(v)`, etc. in AC
/// expressions.
#[derive(Clone, Copy)]
pub struct TypeModule;

impl Module for TypeModule {
    fn name(&self) -> &str {
        "type"
    }

    fn function(&self, name: &str) -> Option<&dyn Function> {
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
