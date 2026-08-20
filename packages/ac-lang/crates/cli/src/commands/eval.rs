use ac_core::Expr;
use ac_runtime::RuntimeValue;
use ac_vm::{Context, Vm};

use crate::cli::EvalArgs;
use crate::frontend;
use crate::host::DefaultHost;
use crate::io;

pub fn run(args: EvalArgs) -> i32 {
    let input = match io::read_input(&args.input) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    };
    let frontend = match frontend::frontend_for(&args.format) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    };
    let value = match frontend.parse(&input) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    };

    let mut vm = Vm::new(DefaultHost::default());
    let mut ctx = Context::new();

    if let Some(obj) = value.as_object() {
        for (k, v) in obj {
            ctx.define(k, RuntimeValue::new(v.clone()));
        }
    }

    let expr = Expr::value(value);
    let result = match vm.eval(&expr, &mut ctx) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    };

    let output_frontend = match frontend::frontend_for(&args.output) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    };
    let output = match output_frontend.serialize(&result) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    };
    print!("{output}");
    0
}
