//! peregrin-engine — the graph walker: events in, driver dispatch,
//! events out, until nothing is ready to fire. Spec: `project/adrs`
//! and the TS `handl-engine` on `main`.

use std::sync::Arc;

use peregrin_driver::Driver;

/// A driver registry keyed by `node.driver`.
#[derive(Default)]
pub struct Registry {
    drivers: std::collections::HashMap<String, Arc<dyn Driver>>,
}

impl Registry {
    #[must_use]
    pub fn with(mut self, name: impl Into<String>, driver: Arc<dyn Driver>) -> Self {
        self.drivers.insert(name.into(), driver);
        self
    }

    #[must_use]
    pub fn get(&self, name: &str) -> Option<Arc<dyn Driver>> {
        self.drivers.get(name).cloned()
    }
}
