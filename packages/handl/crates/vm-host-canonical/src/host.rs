use handl_vm::{Function, Host};
use handl_vm_values_json::NonFunctionValue;

use crate::functions::{Apply, Resolve, Var};

/// The canonical host implementation for the handl VM.
///
/// Provides built-in functions (`var`, `resolve`, and `apply`) that are
/// available in all handl expressions.
#[derive(Clone)]
pub struct CanonicalHost {
    pub var: Var,
    pub resolve: Resolve,
    pub apply: Apply,
}

impl Default for CanonicalHost {
    fn default() -> Self {
        CanonicalHost {
            var: Var,
            resolve: Resolve,
            apply: Apply,
        }
    }
}

impl Host<NonFunctionValue> for CanonicalHost {
    fn function(&self, name: &str) -> Option<&dyn Function<NonFunctionValue>> {
        match name {
            "var" => Some(&self.var),
            "resolve" => Some(&self.resolve),
            "apply" => Some(&self.apply),
            _ => None,
        }
    }

    fn is_reserved(&self, name: &str) -> bool {
        self.function(name).is_some()
    }
}
