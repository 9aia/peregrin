use crate::cli::TypegenArgs;
use crate::commands::ensure_input_format;
use crate::io;

pub fn run(args: TypegenArgs) -> i32 {
    if let Err(e) = ensure_input_format(&args.format) {
        eprintln!("error: {e}");
        return 1;
    }

    let input = match io::read_input(&Some(args.input.clone())) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    };

    let engine_input = ac_compiler_canonical::typegen::TypegenInput {
        input: &input,
        format: &args.format,
        input_path: &args.input,
        output_format: &args.output_format,
        output_dir: &args.output_dir,
        stdout: args.stdout,
    };

    match ac_compiler_canonical::typegen::run(&engine_input) {
        Ok(ac_compiler_canonical::typegen::TypegenResult::Stdout(json)) => {
            println!("{json}");
            0
        }
        Ok(ac_compiler_canonical::typegen::TypegenResult::FileWritten(path)) => {
            println!("schema written to {}", path.display());
            0
        }
        Err(e) => {
            eprintln!("error: {e}");
            1
        }
    }
}
