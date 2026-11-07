//! peregrin-core — the pure domain: events, campaigns, nodes, and the
//! graph model every other crate speaks. No I/O. Port of `ac-core`'s
//! `model.ts`.

mod capability;
mod env;
mod matching;
mod model;

pub use capability::{capability_matches, glob_match};
pub use env::{CHILD_ENV_ALLOWLIST, child_env, sanitize_env};
pub use matching::{capture_wildcards, matches, subscribes_to};
pub use model::{
    Campaign, Event, EventSchema, FunctionDefinition, MaxFires, ModelError, Node, PythonImport,
    Supersede, SupersedeMode, TriggerMode, js_string, js_truthy,
};
