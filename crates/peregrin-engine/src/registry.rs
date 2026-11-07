//! The driver registry — port of `registry.ts`. Names resolve exact
//! first, then by the `name:` prefix (`"ns:x"` → `"ns"`).

use std::collections::HashMap;
use std::sync::Arc;

use peregrin_driver::{Driver, DriverError};

/// `DriverRegistryLive` — driver lookup keyed by `node.driver`.
#[derive(Clone, Default)]
pub struct Registry {
    drivers: HashMap<String, Arc<dyn Driver>>,
}

impl Registry {
    /// Builder-style `register`.
    #[must_use]
    pub fn with(mut self, name: impl Into<String>, driver: Arc<dyn Driver>) -> Self {
        self.register(name, driver);
        self
    }

    /// `register(name, driver)`.
    pub fn register(&mut self, name: impl Into<String>, driver: Arc<dyn Driver>) {
        self.drivers.insert(name.into(), driver);
    }

    /// `resolve(name)` — exact match, else the `:`-prefix; missing
    /// drivers are a `DriverError`.
    pub fn resolve(&self, name: &str) -> Result<Arc<dyn Driver>, DriverError> {
        let prefix = name.split(':').next().unwrap_or(name);
        self.drivers
            .get(name)
            .or_else(|| self.drivers.get(prefix))
            .cloned()
            .ok_or_else(|| DriverError::new(format!("no driver registered for '{name}'")))
    }

    /// The registry a nested run sees (ADR-0022) — the same drivers
    /// minus the ingress impls. A sub-run is finite and runs on its own
    /// isolated bus, so an ingress node inside it could only publish
    /// into the *parent* run's channel — a namespace leak; excluding
    /// them makes such a node fail to resolve instead.
    #[must_use]
    pub fn without_ingress(&self) -> Self {
        Self {
            drivers: self
                .drivers
                .iter()
                .filter(|(_, driver)| !driver.is_ingress())
                .map(|(name, driver)| (name.clone(), Arc::clone(driver)))
                .collect(),
        }
    }
}
