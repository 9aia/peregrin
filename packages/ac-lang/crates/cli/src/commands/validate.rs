use crate::cli::ValidateArgs;
use crate::io;

pub fn run(args: ValidateArgs) -> i32 {
    let input = match io::read_input(&args.input) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    };
    let schema_str = match std::fs::read_to_string(&args.schema) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: failed to read schema: {e}");
            return 1;
        }
    };
    match ac_engine::validate::run(&input, &args.format, &schema_str) {
        Ok(()) => {
            println!("valid");
            0
        }
        Err(e) => {
            eprintln!("invalid: {e}");
            1
        }
    }
}
