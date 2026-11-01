//! peregrin-core — the pure domain: events, campaigns, nodes, and the
//! graph model every other crate speaks. No I/O. Port of `ac-core`'s
//! `model.ts`.

mod model;

pub use model::{
    Campaign, Event, EventSchema, FunctionDefinition, MaxFires, ModelError, Node, PythonImport,
    TriggerMode, js_string, js_truthy,
};
