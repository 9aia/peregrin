use ac_runtime::{Function, Host};
use ac_runtime_values::NonFunctionValue;

use crate::functions::{Apply, Array, Concat, Len, Resolve, Var};

#[derive(Clone)]
pub struct DefaultHost {
    pub concat: Concat,
    pub len: Len,
    pub var: Var,
    pub resolve: Resolve,
    pub apply: Apply,
    pub array: Array,
}

impl Default for DefaultHost {
    fn default() -> Self {
        DefaultHost {
            concat: Concat,
            len: Len,
            var: Var,
            resolve: Resolve,
            apply: Apply,
            array: Array,
        }
    }
}

impl Host<NonFunctionValue> for DefaultHost {
    fn function(&self, name: &str) -> Option<&dyn Function<NonFunctionValue>> {
        match name {
            "concat" => Some(&self.concat),
            "len" => Some(&self.len),
            "var" => Some(&self.var),
            "resolve" => Some(&self.resolve),
            "apply" => Some(&self.apply),
            "array" => Some(&self.array),
            _ => None,
        }
    }

    fn is_reserved(&self, name: &str) -> bool {
        self.function(name).is_some()
    }
}
