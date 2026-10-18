//! peregrin — the CLI's library surface. `src/main.rs` is the thin clap
//! front-end; every verb implementation lives here so integration tests
//! can drive the same code the binary runs.

pub mod api;
pub mod node_ops;
pub mod pair;
pub mod service;

/// The version stamp — same value `/api/node` reports
/// (`peregrin-http`'s `PEREGRIN_VERSION` is the same
/// `CARGO_PKG_VERSION`).
pub const PEREGRIN_VERSION: &str = env!("CARGO_PKG_VERSION");

/// A command failure — printed to stderr, exits 1.
#[derive(Debug)]
pub struct CliError(pub String);

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for CliError {}
