//! peregrin-driver — the campaign driver port. Impls:
//! `peregrin-driver-shell` first; http/agent/human/webhook next.

use async_trait::async_trait;

/// What a driver returns — the events its action emitted.
#[derive(Debug, Default)]
pub struct DriverOutcome {
    pub events: Vec<peregrin_core::Event>,
}

/// The driver port — invoked by the engine when a node fires.
#[async_trait]
pub trait Driver: Send + Sync {
    /// Run the node; emitted events feed the graph.
    async fn run(
        &self,
        node: &peregrin_core::Node,
        event: &peregrin_core::Event,
    ) -> Result<DriverOutcome, DriverError>;
}

/// Driver failure — `retryable` lets the engine decide.
#[derive(Debug, thiserror::Error)]
#[error("{message}")]
pub struct DriverError {
    pub message: String,
    pub retryable: bool,
}

impl DriverError {
    pub fn fatal(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            retryable: false,
        }
    }
}
