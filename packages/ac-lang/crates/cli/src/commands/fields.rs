use crate::cli::FieldsArgs;
use crate::io;

pub fn run(args: FieldsArgs) -> i32 {
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

    match ac_compiler_canonical::fields::run(&input, &args.format, base_path.as_deref()) {
        Ok(report) => {
            println!("fields:");
            for (name, type_name) in &report.field_types {
                println!("  {name}: {type_name}");
            }
            println!();

            for name in &report.ok {
                println!("  {name}: ok");
            }

            for msg in &report.errors {
                eprintln!("  {msg}");
            }

            for msg in &report.undeclared {
                eprintln!("  {msg}");
            }

            if report.is_valid() {
                println!("\nall fields valid");
                0
            } else {
                eprintln!("\n{} validation error(s)", report.error_count());
                1
            }
        }
        Err(e) => {
            eprintln!("error: {e}");
            1
        }
    }
}
