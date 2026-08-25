use handl_vm::Function;
use handl_vm_common::Module;
use handl_vm_values_json::NonFunctionValue;

use crate::option::functions::{
    And, Filter, FlatMap, IsNone, IsSome, Map, MapOr, NoneFn, Or, OrElse, SomeFn, Unwrap, UnwrapOr,
    UnwrapOrElse, Xor,
};

/// The `option` module — constructors and operations for `Option` values
/// in handl expressions.
///
/// An `Option` is represented as `{"_tag": "Some", "value": <T>}` or
/// `{"_tag": "None"}`.
#[derive(Clone, Copy)]
pub struct OptionModule;

impl Module for OptionModule {
    fn name(&self) -> &str {
        "option"
    }

    fn function(&self, name: &str) -> Option<&dyn Function<NonFunctionValue>> {
        match name {
            "some" => Some(&SomeFn),
            "none" => Some(&NoneFn),
            "is_some" => Some(&IsSome),
            "is_none" => Some(&IsNone),
            "unwrap" => Some(&Unwrap),
            "unwrap_or" => Some(&UnwrapOr),
            "unwrap_or_else" => Some(&UnwrapOrElse),
            "map" => Some(&Map),
            "flat_map" => Some(&FlatMap),
            "and_then" => Some(&FlatMap),
            "filter" => Some(&Filter),
            "map_or" => Some(&MapOr),
            "or" => Some(&Or),
            "or_else" => Some(&OrElse),
            "and" => Some(&And),
            "xor" => Some(&Xor),
            _ => None,
        }
    }
}
