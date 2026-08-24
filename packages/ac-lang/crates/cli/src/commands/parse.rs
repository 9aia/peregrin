use crate::cli::ParseArgs;
use crate::commands::ensure_input_format;
use crate::io;

pub fn run(args: ParseArgs) -> i32 {
    if let Err(e) = ensure_input_format(&args.format) {
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
    match ac_compiler_canonical::parse::run(&input, &args.format) {
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
