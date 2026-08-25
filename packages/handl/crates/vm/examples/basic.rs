use handl_vm::{Expr, NoopHost};
use handl_vm_backend_rust_interpreter::{CanonicalContext, Vm};
use handl_vm_values_json::NonFunctionValue;

fn main() {
    let host = NoopHost;
    let vm = Vm::new(host);
    let mut ctx = CanonicalContext::new();
    let expr = Expr::value(NonFunctionValue::new(42));
    let result = vm.eval(&expr, &mut ctx).unwrap();
    assert_eq!(result.as_number(), Some(42.0));
    println!("literal evaluated to {}", result.as_number().unwrap_or(0.0));
}
