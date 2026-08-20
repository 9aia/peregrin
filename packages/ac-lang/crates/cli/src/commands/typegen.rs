use crate::cli::TypegenArgs;
use crate::io;

pub fn run(args: TypegenArgs) -> i32 {
    let input = match io::read_input(&Some(args.input.clone())) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    };

    let engine_input = ac_compiler::typegen::TypegenInput {
        input: &input,
        format: &args.format,
        input_path: &args.input,
        output_format: &args.output_format,
        output_dir: &args.output_dir,
        stdout: args.stdout,
    };

    match ac_compiler::typegen::run(&engine_input) {
        Ok(ac_compiler::typegen::TypegenResult::Stdout(json)) => {
            println!("{json}");
            0
        }
        Ok(ac_compiler::typegen::TypegenResult::FileWritten(path)) => {
            println!("schema written to {}", path.display());
            0
        }
        Err(e) => {
            eprintln!("error: {e}");
            1
        }
    }
}
