use std::path::PathBuf;

use handl_cli_plugin::CommandOutcome;
use handl_compiler_backend_js::compile_module;
use handl_lexer_canonical::CanonicalLexer;
use handl_parser::Parser;
use handl_parser_canonical::CanonicalParser;

use crate::cli::CompileArgs;
use crate::io;

pub fn run(args: &CompileArgs) -> CommandOutcome {
    if args.input.len() > 1 {
        return CommandOutcome::new(1).with_stderr("error: compile takes a single input\n");
    }

    let inputs = match io::read_inputs(&args.input) {
        Ok(v) => v,
        Err(e) => {
            return CommandOutcome::new(1).with_stderr(format!("error: {e}\n"));
        }
    };

    let (_, input) = &inputs[0];
    match compile(input, &args.format, &args.target, &args.output) {
        Ok(js) => CommandOutcome::new(0).with_stdout(js),
        Err(e) => CommandOutcome::new(1).with_stderr(format!("error: {e}\n")),
    }
}

fn compile(
    input: &str,
    format: &str,
    target: &str,
    output: &Option<PathBuf>,
) -> Result<String, String> {
    if target != "js" {
        return Err(format!("unsupported target: {target}"));
    }
    if format != "handl" {
        return Err(format!("unsupported input format for compile: {format}"));
    }
    let expr = CanonicalParser
        .parse_text(input, &CanonicalLexer::default())
        .map_err(|e| e.to_string())?;
    let js = compile_module(&expr).map_err(|e| e.to_string())?;
    match output {
        Some(p) => {
            std::fs::write(p, &js).map_err(|e| e.to_string())?;
            Ok(String::new())
        }
        None => Ok(js),
    }
}
