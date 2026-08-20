use ac_schema::Schema;
use ac_schema_json::JsonSchema;

use crate::cli::ValidateArgs;
use crate::frontend;
use crate::io;

pub fn run(args: ValidateArgs) -> i32 {
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

    let schema_str = match std::fs::read_to_string(&args.schema) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: failed to read schema: {e}");
            return 1;
        }
    };
    let schema = match JsonSchema::parse(&schema_str) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: failed to parse schema: {e}");
            return 1;
        }
    };

    match schema.validate(&value) {
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
