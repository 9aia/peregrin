use ac_runtime::{Function, Host};

use crate::functions::{Add, Apply, Array, Concat, Len, Mul, Resolve, Var};

#[derive(Clone)]
pub struct DefaultHost {
    pub add: Add,
    pub mul: Mul,
    pub concat: Concat,
    pub len: Len,
    pub var: Var,
    pub resolve: Resolve,
    pub apply: Apply,
    pub array: Array}

impl Default for DefaultHost {
    fn default() -> Self {
        DefaultHost {
            add: Add,
            mul: Mul,
            concat: Concat,
            len: Len,
            var: Var,
            resolve: Resolve,
            apply: Apply,
            array: Array}
    }
}

impl Host for DefaultHost {
    fn function(&self, name: &str) -> Option<&dyn Function> {
        match name {
            "add" => Some(&self.add),
            "mul" => Some(&self.mul),
            "concat" => Some(&self.concat),
            "len" => Some(&self.len),
            "var" => Some(&self.var),
            "resolve" => Some(&self.resolve),
            "apply" => Some(&self.apply),
            "array" => Some(&self.array),
            _ => None}
    }
}
