//! peregrin-store — the state port: run records, emitted events,
//! checkpoints. Impls: memory/json first, redb/sqlite later.

use async_trait::async_trait;

/// Persistence for engine runs and their event history.
#[async_trait]
pub trait Store: Send + Sync {
    async fn record(&self, event: &peregrin_core::Event) -> std::io::Result<()>;
}
