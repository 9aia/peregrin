#![allow(clippy::unwrap_used)]
use async_trait::async_trait;
use peregrin_driver::{Driver, DriverError, DriverOutcome};
use peregrin_engine::Registry;
use std::sync::Arc;

struct Noop;
#[async_trait]
impl Driver for Noop {
    async fn run(
        &self,
        _n: &peregrin_core::Node,
        _e: &peregrin_core::Event,
    ) -> Result<DriverOutcome, DriverError> {
        Ok(DriverOutcome::default())
    }
}

#[test]
fn registry_lookup() {
    let r = Registry::default().with("shell", Arc::new(Noop));
    assert!(r.get("shell").is_some());
    assert!(r.get("missing").is_none());
}
