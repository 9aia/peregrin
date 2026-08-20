use crate::cli::EvalArgs;
use crate::io;

pub fn run(args: EvalArgs) -> i32 {
    let input = match io::read_input(&args.input) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    };
    match ac_compiler::eval::run(&input, &args.format, &args.output) {
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
