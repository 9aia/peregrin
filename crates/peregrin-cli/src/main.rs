//! peregrin — the compat CLI driver. The engine is the
//! `peregrin_cli::cli` module (the clap tree, dispatch, and every
//! verb); this binary is just [`peregrin_cli::cli::run`] mapped to a
//! process exit code. `tamias-driver-peregrin` reaches the same
//! surface through the lib (`ops` manifest + `run` passthrough) —
//! see `peregrin_cli::engine_ops` / `peregrin_cli::pavo`.

fn main() {
    std::process::exit(peregrin_cli::cli::run());
}
