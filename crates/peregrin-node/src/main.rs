//! peregrin-node — the headless daemon. Discovers drivers, builds the
//! merged store + control plane, serves the `/api/*` surface, and shuts
//! the plane down on SIGINT/SIGTERM.
//!
//! The boot sequence lives in [`peregrin_node::serve`] — `peregrin serve`
//! (crates/peregrin-cli) runs the same path.

use peregrin_http::env::Env;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // fmt + RUST_LOG as always; OTLP span export layered on when
    // HYPHA_OTEL_ENDPOINT / OTEL_EXPORTER_OTLP_* is set. The guard's
    // Drop flushes the provider on exit.
    let spec = lampy_telemetry::TelemetrySpec::service("peregrin-node")
        .with_alias("PEREGRIN")
        .with_alias("HYPHA");
    let _telemetry = lampy_telemetry_otel::init(&spec, env!("CARGO_PKG_NAME"));

    let env = Env::parse().map_err(std::io::Error::other)?;
    peregrin_node::serve(&env).await
}
