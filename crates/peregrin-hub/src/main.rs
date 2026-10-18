//! peregrin-hub — the SSR host binary. Serves the `peregrin-web` Leptos UI
//! and bridges browser traffic to the node daemon over HTTP
//! (`PEREGRIN_NODE_URL`).
//!
//! Env: `PEREGRIN_HUB_HOST`/`PEREGRIN_HUB_PORT` (or `PEREGRIN_PORT`),
//! `PEREGRIN_HUB_TOKEN` (required on non-loopback binds),
//! `PEREGRIN_NODE_URL`, `PEREGRIN_NODE_TOKEN`, `PEREGRIN_SITE_ROOT`
//! (or `LEPTOS_SITE_ROOT`), `LEPTOS_ENV`/`PEREGRIN_HUB_ENV`.
//! `cargo-leptos` sets the `LEPTOS_*` vars when run via
//! `cargo leptos watch`.

use peregrin_hub::{HubConfig, hub_state, router};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // fmt + RUST_LOG as always; OTLP span export layered on when
    // PEREGRIN_OTEL_ENDPOINT / OTEL_EXPORTER_OTLP_* is set. The guard's
    // Drop flushes the provider on exit.
    let _telemetry = peregrin_telemetry::init();

    let config = HubConfig::from_env().map_err(std::io::Error::other)?;
    let addr = config.socket_addr().map_err(std::io::Error::other)?;
    let state = hub_state(&config).map_err(std::io::Error::other)?;
    let app = router(state);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(
        %addr,
        node = %config.node_url,
        site_root = %config.site_root,
        "peregrin-hub listening"
    );
    axum::serve(listener, app).await
}
