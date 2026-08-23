use ac_runtime::Function;
use ac_runtime_modules::Module;

use crate::r#fn::combinators::{
    Apply, Choice, Compose, Constantly, Converge, Fanout, Flip, Flow, Partial, Pipe, Tap, Unless,
    When,
};

/// The `fn` module — function combinator utilities.
///
/// Provides functions callable as `fn.pipe(v, f, g)`, `fn.compose(f, g)`,
/// etc. in AC expressions.
#[derive(Clone, Copy)]
pub struct FnModule;

impl Module for FnModule {
    fn name(&self) -> &str {
        "fn"
    }

    fn function(&self, name: &str) -> Option<&dyn Function> {
        match name {
            "pipe" => Some(&Pipe),
            "flow" => Some(&Flow),
            "compose" => Some(&Compose),
            "fanout" => Some(&Fanout),
            "choice" => Some(&Choice),
            "apply" => Some(&Apply),
            "tap" => Some(&Tap),
            "constantly" => Some(&Constantly),
            "flip" => Some(&Flip),
            "partial" => Some(&Partial),
            "when" => Some(&When),
            "unless" => Some(&Unless),
            "converge" => Some(&Converge),
            _ => None,
        }
    }
}
