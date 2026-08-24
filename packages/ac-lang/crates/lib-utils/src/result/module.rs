use ac_runtime::{Function};
use ac_runtime_modules::Module;
use ac_runtime_values::NonFunctionValue;

use crate::result::functions::{Err, IsErr, IsOk, Match, Ok};

/// The `result` module — constructors, predicates, and eliminator for
/// `Result` values in AC expressions.
///
/// A `Result` is represented as `{"_tag": "Ok", "value": <T>}` or
/// `{"_tag": "Err", "error": <E>}`. Combinators like `map` and `and_then`
/// are intentionally left to the `fn` module and composed with `result.match`.
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
            "match" => Some(&Match),
            _ => None,
        }
    }
}
