use crate::cli::EvalArgs;
use crate::commands::{ensure_input_format, ensure_output_format};
use crate::io;

pub fn run(args: EvalArgs) -> i32 {
    if let Err(e) = ensure_input_format(&args.format) {
        eprintln!("error: {e}");
        return 1;
    }
    if let Err(e) = ensure_output_format(&args.output) {
        eprintln!("error: {e}");
        return 1;
    }

    let input = match io::read_input(&args.input) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    };
    match ac_compiler_canonical::eval::run(&input, &args.format, &args.output, &args.vm) {
        Ok(output) => {
            print!("{output}");
            0
        }
        Err(e) => {
            eprintln!("error: {e}");
            1
        }
    }
}
