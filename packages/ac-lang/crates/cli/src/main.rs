//! # ac-cli
//!
//! Command-line interface for the AC language.
//!
//! Parses input files (YAML, JSON, or TOML), evaluates expressions,
//! and outputs results. Validates against external JSON Schema or
//! inline `fields:` declarations.

mod cli;
mod commands;
mod frontend;
mod host;
mod io;
mod resolver;

use clap::Parser;

use cli::{Cli, Command};

fn main() {
    let cli = Cli::parse();
    let code = match cli.command {
        Command::Parse(args) => commands::parse::run(args),
        Command::Eval(args) => commands::eval::run(args),
        Command::Validate(args) => commands::validate::run(args),
        Command::Fields(args) => commands::fields::run(args),
        Command::Typegen(args) => commands::typegen::run(args),
    };
    std::process::exit(code);
}
