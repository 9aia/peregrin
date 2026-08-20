use crate::cli::ParseArgs;
use crate::frontend;
use crate::io;

pub fn run(args: ParseArgs) -> i32 {
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
    let output = match ac_frontend_yaml::to_string(&value) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    };
    print!("{output}");
    0
}
