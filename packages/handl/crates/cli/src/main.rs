//! # handl-cli
//!
//! Command-line interface for the handl language.
//!
//! Thin wrapper around `handl-compiler` — handles argument parsing, I/O,
//! and output formatting. All logic lives in the engine crate.

mod cli;
mod commands;
mod io;

use std::io::Write;

use handl_cli_plugin::{
    CliContext, Command as PluginCommand, InProcessPlugin, PluginRegistry, ProjectManifest,
};
use clap::Parser;

use cli::{Cli, Command};

fn main() {
    let cli = Cli::parse();
    let mut registry = PluginRegistry::new();
    let manifest = ProjectManifest::load().ok();

    let command = match cli.command {
        Command::Eval(args) => {
            let command = PluginCommand {
                name: "eval".to_string(),
                target: None,
                args: args
                    .input
                    .iter()
                    .map(|p| p.to_string_lossy().into_owned())
                    .collect(),
                cwd: std::env::current_dir().ok(),
                env: std::env::vars().collect(),
            };
            let plugin = InProcessPlugin::new("eval", &["eval"], move |_, _| {
                Ok(commands::eval::run(&args))
            });
            registry.register(plugin);
            command
        }
        Command::Compile(args) => {
            let mut command = PluginCommand {
                name: "compile".to_string(),
                target: Some(args.target.clone()),
                args: args
                    .input
                    .iter()
                    .map(|p| p.to_string_lossy().into_owned())
                    .collect(),
                cwd: std::env::current_dir().ok(),
                env: std::env::vars().collect(),
            };
            if let Some(out) = &args.output {
                command.args.push(out.to_string_lossy().into_owned());
            }
            command.args.push(format!("--format={}", args.format));
            let plugin = InProcessPlugin::new("compile", &["compile"], move |_, _| {
                Ok(commands::compile::run(&args))
            });
            registry.register(plugin);
            command
        }
        Command::Typecheck(args) => {
            let command = PluginCommand {
                name: "typecheck".to_string(),
                target: None,
                args: args
                    .input
                    .iter()
                    .map(|p| p.to_string_lossy().into_owned())
                    .collect(),
                cwd: std::env::current_dir().ok(),
                env: std::env::vars().collect(),
            };
            let plugin = InProcessPlugin::new("typecheck", &["typecheck"], move |_, _| {
                Ok(commands::typecheck::run(&args))
            });
            registry.register(plugin);
            command
        }
    };

    let ctx = CliContext::new(&registry, manifest);
    match registry.resolve(&command.name, command.target.as_deref()) {
        Some(plugin) => match plugin.run(&command, &ctx) {
            Ok(outcome) => {
                let _ = std::io::stdout().write_all(&outcome.stdout);
                let _ = std::io::stderr().write_all(&outcome.stderr);
                std::process::exit(outcome.exit_code);
            }
            Err(e) => {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        },
        None => {
            eprintln!("error: no plugin for command {}", command.name);
            std::process::exit(1);
        }
    }
}
