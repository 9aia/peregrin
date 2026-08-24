use ac_runtime::Function;
use ac_runtime_modules::Module;
use ac_runtime_values::NonFunctionValue;

use crate::result::functions::{Err, IsErr, IsOk, Ok};

/// The `result` module — constructors and predicates for `Result` values
/// in AC expressions.
///
/// A `Result` is represented as `{"_tag": "Ok", "value": <T>}` or
/// `{"_tag": "Err", "error": <E>}`.
#[derive(Clone, Copy)]
pub struct ResultModule;

impl Module for ResultModule {
    fn name(&self) -> &str {
        "result"
    }

    fn function(&self, name: &str) -> Option<&dyn Function<NonFunctionValue>> {
        match name {
            "ok" => Some(&Ok),
            "err" => Some(&Err),
            "is_ok" => Some(&IsOk),
            "is_err" => Some(&IsErr),
            _ => None,
        }
    }
}
