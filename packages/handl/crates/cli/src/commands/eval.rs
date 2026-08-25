use handl_cli_plugin::CommandOutcome;

use crate::cli::EvalArgs;
use crate::commands::{ensure_input_format, ensure_output_format};
use crate::io;

pub fn run(args: &EvalArgs) -> CommandOutcome {
    if let Err(e) = ensure_input_format(&args.format) {
        return CommandOutcome::new(1).with_stderr(format!("error: {e}\n"));
    }
    if let Err(e) = ensure_output_format(&args.output) {
        return CommandOutcome::new(1).with_stderr(format!("error: {e}\n"));
    }

    let inputs = match io::read_inputs(&args.input) {
        Ok(v) => v,
        Err(e) => {
            return CommandOutcome::new(1).with_stderr(format!("error: {e}\n"));
        }
    };

    let mut stdout = String::new();
    let mut stderr = String::new();
    let mut code = 0;
    for (i, (_, input)) in inputs.iter().enumerate() {
        if i > 0 {
            stdout.push('\n');
        }
        match handl_compiler_canonical::eval::run(input, &args.format, &args.output, &args.vm) {
            Ok(output) => stdout.push_str(&output),
            Err(e) => {
                stderr.push_str(&format!("error: {e}\n"));
                code = 1;
            }
        }
    }
    CommandOutcome::new(code)
        .with_stdout(stdout)
        .with_stderr(stderr)
}
