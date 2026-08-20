use ac_vm::{Function, Host};

use crate::functions::{Add, Concat, Len, Mul};

#[derive(Clone)]
pub struct DefaultHost {
    pub add: Add,
    pub mul: Mul,
    pub concat: Concat,
    pub len: Len,
}

impl Default for DefaultHost {
    fn default() -> Self {
        DefaultHost {
            add: Add,
            mul: Mul,
            concat: Concat,
            len: Len,
        }
    }
}

impl Host for DefaultHost {
    fn function(&self, name: &str) -> Option<&dyn Function> {
        match name {
            "add" => Some(&self.add),
            "mul" => Some(&self.mul),
            "concat" => Some(&self.concat),
            "len" => Some(&self.len),
            _ => None,
        }
    }
}
