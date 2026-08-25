use handl_cli_plugin::CommandOutcome;

use crate::cli::TypecheckArgs;
use crate::commands::ensure_input_format;
use crate::io;

pub fn run(args: &TypecheckArgs) -> CommandOutcome {
    if let Err(e) = ensure_input_format(&args.format) {
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
    for (path, input) in &inputs {
        let base_path = path.as_ref().and_then(|p| p.parent());

        match handl_compiler_canonical::typecheck::run(input, &args.format, base_path) {
            Ok(report) => {
                if report.is_valid() {
                    stdout.push_str("all types valid\n");
                } else {
                    for err in &report.errors {
                        stderr.push_str(&format!("  {err}\n"));
                    }
                    stderr.push_str(&format!("\n{} type error(s)\n", report.errors.len()));
                    code = 1;
                }
            }
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
