//! peregrin-engine — the graph walker: events in, driver dispatch,
//! events out, until nothing is ready to fire. Port of the TS
//! `ac-engine` (`runner.ts`/`bus.ts`/`registry.ts`) plus the
//! `ac-safety` pieces it calls. Spec: `project/adrs` and the TS
//! `handl-engine` on `main`.
//!
//! [`Runner`] drives a [`peregrin_core::Campaign`] against a
//! [`Registry`] of drivers, a [`peregrin_bus::EventBus`], and the
//! persistence ports in `peregrin-store`, returning the event log of
//! the run.

mod ingress;
mod registry;
mod runner;
mod safety;

pub use ingress::{Ingress, InjectError, Injector, WaitOutcome, ingress};
pub use peregrin_driver::SecretSource;
pub use registry::Registry;
pub use runner::{PendingTimer, RunOptions, Runner};
pub use safety::{
    CapabilityChecker, ResourceGate, SchemaValidator, build_validator, validate_object,
    validate_record,
};
