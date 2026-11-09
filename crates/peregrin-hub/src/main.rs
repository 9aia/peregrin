//! peregrin-hub — the node-API gateway binary. Multi-node sync engine
//! plus HTTP/SSE bridges to the node daemon (`PEREGRIN_NODE_URL`).
//!
//! Env: `PEREGRIN_HUB_HOST`/`PEREGRIN_HUB_PORT` (or `PEREGRIN_PORT`),
//! `PEREGRIN_HUB_TOKEN` (required on non-loopback binds),
//! `PEREGRIN_NODE_URL`, `PEREGRIN_NODE_TOKEN`, `PEREGRIN_NODES`
//! (`id=url[@token];…` for multi-node).

use peregrin_hub::{HubConfig, hub_state, router};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // fmt + RUST_LOG as always; OTLP span export layered on when
    // HYPHA_OTEL_ENDPOINT / OTEL_EXPORTER_OTLP_* is set. The guard's
    // Drop flushes the provider on exit.
    let _telemetry = hypha_telemetry::init();

    let config = HubConfig::from_env().map_err(std::io::Error::other)?;
    let addr = config.socket_addr().map_err(std::io::Error::other)?;
    let state = hub_state(&config).map_err(std::io::Error::other)?;
    let app = router(state);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(%addr, node = %config.node_url, "peregrin-hub listening");
    // Graceful SIGINT — mirrors the node's shutdown so an operator's
    // ^C (or a test child's stop signal) closes listeners cleanly
    // instead of dying on the signal.
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
            tracing::info!("peregrin-hub shutting down");
        })
        .await
}
