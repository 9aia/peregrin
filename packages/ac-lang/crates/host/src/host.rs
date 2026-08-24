use ac_runtime::{Function, Host};
use ac_runtime_values::NonFunctionValue;

use crate::functions::{Apply, Resolve, Var};

#[derive(Clone)]
pub struct DefaultHost {
    pub var: Var,
    pub resolve: Resolve,
    pub apply: Apply,
}

impl Default for DefaultHost {
    fn default() -> Self {
        DefaultHost {
            var: Var,
            resolve: Resolve,
            apply: Apply,
        }
    }
}

impl Host<NonFunctionValue> for DefaultHost {
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
