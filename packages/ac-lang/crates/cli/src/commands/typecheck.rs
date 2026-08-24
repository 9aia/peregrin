use crate::cli::TypecheckArgs;
use crate::commands::ensure_input_format;
use crate::io;

pub fn run(args: TypecheckArgs) -> i32 {
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
    let base_path = args
        .input
        .as_ref()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()));

    match ac_compiler_canonical::typecheck::run(&input, &args.format, base_path.as_deref()) {
        Ok(report) => {
            if report.is_valid() {
                println!("all types valid");
                0
            } else {
                for err in &report.errors {
                    eprintln!("  {err}");
                }
                eprintln!("\n{} type error(s)", report.errors.len());
                1
            }
        }
        Err(e) => {
            eprintln!("error: {e}");
            1
        }
    }
}
