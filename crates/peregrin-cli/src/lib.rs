//! peregrin — the CLI's library surface. There is no `peregrin`
//! binary: the command ships as the tamias pack's `bins[]` shim →
//! `tamias-driver-peregrin cli` → [`cli::run_from`]. Every verb
//! implementation lives here so integration tests and the driver
//! can drive the same code the shim runs.
//!
//! The "CLI-as-generated-surface" model (P-021): [`cli`] is the
//! engine — the clap tree plus `command()`/`parse_from()`/
//! `run_from()` — and [`engine_ops`] derives the driver's ops
//! manifest from that tree (`cli.<verb>` wire queries re-exec
//! `run` in a captured child). [`pavo`] is the driver's wire
//! dispatch (`manifest`/`views`/`query`) shared by the
//! `tamias-driver-peregrin serve` loop.

pub mod agent;
pub mod api;
pub mod campaign;
pub mod cli;

pub mod engine_ops;
pub mod import;
pub mod node_ops;
pub mod ops;
pub mod pair;
pub mod pavo;
pub mod provision;
#[cfg(feature = "mcp")]
pub mod surface;

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
