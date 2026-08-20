use super::*;
use crate::context::Context;
use crate::error::Error;
use crate::host::{Function, Host};
use ac_ast::Expr;
use ac_runtime::RuntimeValue;

#[derive(Clone)]
struct Add;
impl Function for Add {
    fn call(&self, args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
        let sum: f64 = args.iter().filter_map(|v| v.as_number()).sum();
        Ok(RuntimeValue::new(sum))
    }
}

#[derive(Clone)]
struct DefaultHost {
    add: Add,
}

impl Host for DefaultHost {
    fn function(&self, name: &str) -> Option<&dyn Function> {
        match name {
            "add" => Some(&self.add),
            _ => None,
        }
    }
}

impl Default for DefaultHost {
    fn default() -> Self {
        DefaultHost { add: Add }
    }
}

#[test]
fn vm_eval_value() {
    let mut vm = Vm::new(DefaultHost::default());
    let mut ctx = Context::new();
    let expr = Expr::value(RuntimeValue::new(42));
    let result = vm.eval(&expr, &mut ctx).unwrap();
    assert_eq!(result.as_number(), Some(42.0));
}

#[test]
fn vm_eval_call() {
    let mut vm = Vm::new(DefaultHost::default());
    let mut ctx = Context::new();
    let expr = Expr::call("add", vec![Expr::value(RuntimeValue::new(1)), Expr::value(RuntimeValue::new(2))]);
    let result = vm.eval(&expr, &mut ctx).unwrap();
    assert_eq!(result.as_number(), Some(3.0));
}

#[test]
fn vm_eval_unknown_function() {
    let mut vm = Vm::new(DefaultHost::default());
    let mut ctx = Context::new();
    let expr = Expr::call("nonexistent", vec![]);
    let result = vm.eval(&expr, &mut ctx);
    assert!(result.is_err());
}

#[test]
fn vm_host_accessor() {
    let vm = Vm::new(DefaultHost::default());
    assert!(vm.host().function("add").is_some());
    assert!(vm.host().function("nope").is_none());
}
