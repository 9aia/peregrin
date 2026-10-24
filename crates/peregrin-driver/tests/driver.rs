#![allow(clippy::unwrap_used)]
use async_trait::async_trait;
use peregrin_driver::{Driver, DriverError, DriverOutcome};

struct Echo;
#[async_trait]
impl Driver for Echo {
    async fn run(
        &self,
        _n: &peregrin_core::Node,
        _e: &peregrin_core::Event,
    ) -> Result<DriverOutcome, DriverError> {
        Err(DriverError::fatal("no"))
    }
}

#[tokio::test]
async fn outcome_and_error() {
    let err = Echo
        .run(
            &peregrin_core::Node {
                id: "n".into(),
                driver: "x".into(),
                params: Default::default(),
                on: vec![],
            },
            &peregrin_core::Event {
                kind: "k".into(),
                payload: Default::default(),
            },
        )
        .await
        .unwrap_err();
    assert!(!err.retryable);
    assert_eq!(format!("{err}"), "no");
    assert!(DriverOutcome::default().events.is_empty());
}
