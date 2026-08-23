use ac_ast_canonical::Expr;
use ac_frontend::frontend_for;
use ac_runtime_example_1::DefaultHost;
use ac_runtime::NonFunctionValue;
use ac_runtime_canonical::{CanonicalContext, Context, Vm};

pub fn run(input: &str, format: &str, output_format: &str) -> Result<String, String> {
    let frontend = frontend_for(format)?;
    let value = frontend.parse(input)?;

    let vm = Vm::new(DefaultHost::default());
    let mut ctx = CanonicalContext::new();

    if let Some(obj) = value.as_object() {
        for (k, v) in obj {
            ctx.define(k, NonFunctionValue::new(v.clone()));
        }
    }

    let expr = Expr::value(value);
    let result = vm.eval(&expr, &mut ctx).map_err(|e| e.to_string())?;

    let output_frontend = frontend_for(output_format)?;
    let output = output_frontend.serialize(&result)?;
    Ok(output)
}
