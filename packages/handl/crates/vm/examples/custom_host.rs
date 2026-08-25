use handl_vm::{Context, Error, Expr, Function, Host};
use handl_vm_backend_rust_interpreter::{CanonicalContext, Vm};
use handl_vm_values_json::NonFunctionValue;

#[derive(Clone, Default)]
struct Add;

impl Function<NonFunctionValue> for Add {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error<NonFunctionValue>> {
        let sum: f64 = args.iter().filter_map(|v| v.as_number()).sum();
        Ok(NonFunctionValue::new(sum))
    }
}

#[derive(Clone, Default)]
struct MathHost {
    add: Add,
}

impl Host<NonFunctionValue> for MathHost {
    fn function(&self, name: &str) -> Option<&dyn Function<NonFunctionValue>> {
        match name {
            "add" => Some(&self.add),
            _ => None,
        }
    }
}

fn main() {
    let host = MathHost::default();
    let vm = Vm::new(host);
    let mut ctx = CanonicalContext::new();
    let expr = Expr::call(
        "add",
        vec![
            Expr::value(NonFunctionValue::new(1)),
            Expr::value(NonFunctionValue::new(2)),
        ],
    );
    let result = vm.eval(&expr, &mut ctx).unwrap();
    assert_eq!(result.as_number(), Some(3.0));
    println!(
        "custom host add(1, 2) = {}",
        result.as_number().unwrap_or(0.0)
    );
}
