use ac_ast::Expr;
use ac_frontend::frontend_for;
use ac_host::DefaultHost;
use ac_runtime::RuntimeValue;
use ac_vm::{Context, Vm};

pub fn run(input: &str, format: &str, output_format: &str) -> Result<String, String> {
    let frontend = frontend_for(format)?;
    let value = frontend.parse(input)?;

    let mut vm = Vm::new(DefaultHost::default());
    let mut ctx = Context::new();

    if let Some(obj) = value.as_object() {
        for (k, v) in obj {
            ctx.define(k, RuntimeValue::new(v.clone()));
        }
    }

    let expr = Expr::value(value);
    let result = vm.eval(&expr, &mut ctx).map_err(|e| e.to_string())?;

    let output_frontend = frontend_for(output_format)?;
    let output = output_frontend.serialize(&result)?;
    Ok(output)
}
