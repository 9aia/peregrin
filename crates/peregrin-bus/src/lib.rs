//! peregrin-bus — the event bus port: publish/subscribe for campaign
//! events. Local impl in-process; remote fan-out lives on the
//! `peregrin-remote` transport.

use async_trait::async_trait;

/// Subscribers receive every published event.
#[async_trait]
pub trait Bus: Send + Sync {
    async fn publish(&self, event: peregrin_core::Event);
}
