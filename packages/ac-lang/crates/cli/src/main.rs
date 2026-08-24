//! # ac-cli
//!
//! Command-line interface for the AC language.
//!
//! Thin wrapper around `ac-compiler` — handles argument parsing, I/O,
//! and output formatting. All logic lives in the engine crate.

mod cli;
mod commands;
mod io;

use clap::Parser;

use cli::{Cli, Command};

fn main() {
    let cli = Cli::parse();
    let code = match cli.command {
        Command::Parse(args) => commands::parse::run(args),
        Command::Eval(args) => commands::eval::run(args),
        Command::Compile(args) => commands::compile::run(args),
        Command::Validate(args) => commands::validate::run(args),
        Command::Fields(args) => commands::fields::run(args),
        Command::Typegen(args) => commands::typegen::run(args),
        Command::Typecheck(args) => commands::typecheck::run(args),
    };
    std::process::exit(code);
}
