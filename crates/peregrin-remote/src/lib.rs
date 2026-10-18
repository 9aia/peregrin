//! peregrin-remote — the transport port between peregrin nodes and
//! the hub. Impls: `peregrin-remote-http` (REST+SSE, current), a p2p
//! impl (iroh/libp2p) behind this same port — see project ADRs.

use async_trait::async_trait;

/// A remote endpoint a node can reach.
#[async_trait]
pub trait Transport: Send + Sync {
    /// Send an event to a peer.
    async fn send(&self, event: &peregrin_core::Event) -> std::io::Result<()>;
}
