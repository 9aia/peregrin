use std::path::PathBuf;

use ac_compiler_backend_js::compile_module;
use ac_lexer_canonical::CanonicalLexer;
use ac_parser::Parser;
use ac_parser_canonical::CanonicalParser;

use crate::cli::CompileArgs;
use crate::io;

pub fn run(args: CompileArgs) -> i32 {
    let input = match io::read_input(&args.input) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    };
    match compile(&input, &args.format, &args.target, &args.output) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("error: {e}");
            1
        }
    }
}

fn compile(input: &str, format: &str, target: &str, output: &Option<PathBuf>) -> Result<(), String> {
    if target != "js" {
        return Err(format!("unsupported target: {target}"));
    }
    if format != "ac" {
        return Err(format!("unsupported input format for compile: {format}"));
    }
    let expr = CanonicalParser
        .parse_text(input, &CanonicalLexer::default())
        .map_err(|e| e.to_string())?;
    let js = compile_module(&expr).map_err(|e| e.to_string())?;
    match output {
        Some(p) => std::fs::write(p, js).map_err(|e| e.to_string()),
        None => {
            print!("{js}");
            Ok(())
        }
    }
}
