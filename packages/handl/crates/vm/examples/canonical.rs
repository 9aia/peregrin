use handl_vm::Expr;
use handl_vm_backend_rust_interpreter::{CanonicalContext, Vm};
use handl_vm_host_canonical::CanonicalHost;
use handl_vm_values_json::NonFunctionValue;

fn main() {
    let host = CanonicalHost::default();
    let vm = Vm::new(host);
    let mut ctx = CanonicalContext::new();
    ctx.define("greeting", NonFunctionValue::new("hello"));

    let expr = Expr::call("var", vec![Expr::value(NonFunctionValue::new("greeting"))]);
    let result = vm.eval(&expr, &mut ctx).unwrap();
    assert_eq!(result.as_str(), Some("hello"));
    println!("canonical host resolved to {:?}", result.as_str());
}
