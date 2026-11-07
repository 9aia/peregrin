//! tamias-driver-peregrin — the tamias content **driver** for
//! `peregrin-bundle`. tamias discovers this on `PATH` /
//! `<tamias-home>/drivers/` and runs
//! `<driver> <verb> --dir <d> --name <n> [--config <c>]`.
//!
//! Verbs: `install` (materialize a node config), `deploy` (write the unit),
//! `uninstall` (remove both). Thin wrapper over `peregrin_cli::provision`.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use peregrin_cli::provision;

#[derive(Parser)]
#[command(name = "tamias-driver-peregrin", version)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Materialize a node config from an installed bundle dir.
    Install {
        #[arg(long)]
        dir: PathBuf,
        #[arg(long)]
        name: String,
        #[arg(long)]
        config: Option<PathBuf>,
    },
    /// Write the per-user systemd unit.
    Deploy {
        #[arg(long)]
        dir: PathBuf,
        #[arg(long)]
        name: String,
        #[arg(long)]
        config: Option<PathBuf>,
    },
    /// Remove the node config + the unit.
    Uninstall {
        #[arg(long)]
        name: String,
        #[arg(long)]
        config: Option<PathBuf>,
    },
    /// Report the content types + verbs this driver handles (JSON).
    Capabilities,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.cmd {
        Cmd::Install { dir, name, config } => provision::install(&dir, &name, config.as_deref()),
        Cmd::Deploy { dir, name, config } => provision::deploy(&dir, &name, config.as_deref()),
        Cmd::Uninstall { name, config } => provision::uninstall(&name, config.as_deref()),
        Cmd::Capabilities => {
            println!(
                "{{\"types\":[\"peregrin-bundle\"],\"verbs\":[\"install\",\"deploy\",\"uninstall\"]}}"
            );
            return ExitCode::SUCCESS;
        }
    };
    match result {
        Ok(line) => {
            println!("{line}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("tamias-driver-peregrin: {e}");
            ExitCode::FAILURE
        }
    }
}
