use handl_vm::{Function, LazyFunction};
use handl_vm_common::Module;
use handl_vm_values_json::NonFunctionValue;

use crate::r#fn::combinators::{
    Apply, Choice, Compose, Constantly, Converge, Fanout, Flip, Flow, IfLazy, Match, Partial, Pipe,
    Tap, Unless, When,
};

/// The `fn` module — function combinator utilities.
///
/// Provides functions callable as `fn.pipe(v, f, g)`, `fn.compose(f, g)`,
/// etc. in handl expressions.
#[derive(Clone, Copy)]
pub struct FnModule;

impl Module for FnModule {
    fn name(&self) -> &str {
        "fn"
    }

    fn function(&self, name: &str) -> Option<&dyn Function<NonFunctionValue>> {
        match name {
            "pipe" => Some(&Pipe),
            "flow" => Some(&Flow),
            "compose" => Some(&Compose),
            "fanout" => Some(&Fanout),
            "choice" => Some(&Choice),
            "apply" => Some(&Apply),
            "tap" => Some(&Tap),
            "match" => Some(&Match),
            "constantly" => Some(&Constantly),
            "flip" => Some(&Flip),
            "partial" => Some(&Partial),
            "when" => Some(&When),
            "unless" => Some(&Unless),
            "converge" => Some(&Converge),
            _ => None,
        }
    }

    fn lazy_function(&self, name: &str) -> Option<&dyn LazyFunction<NonFunctionValue>> {
        match name {
            "if" => Some(&IfLazy),
            _ => None,
        }
    }
}
