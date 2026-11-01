//! peregrin-template — `${expr}` interpolation over the event/context
//! scope (port of the TS `ac-template-engine`).
//!
//! Placeholders look like `${...}` inside `params` strings. Two families:
//!
//! - *Jinja-style lookups* (`${config.x}`, `${event.type}`, `${secret.k}`,
//!   `${node.id}`, `${params.y}`, `${state.z}`) are **not** resolved here —
//!   they pass through untouched for a later scope-resolution pass.
//! - *Function calls* (`${env("VAR")}`, `${random(1, 10)}`, `${file("p")}`,
//!   `${shell("cmd")}`, `${now()}`, `${now.unix()}`, `${uuid()}`) resolve
//!   only when the function is whitelisted by a `capabilities` entry.
//!
//! Anything unrecognized is preserved verbatim (permissive default); in
//! `strict` mode unresolved placeholders and missing secrets become
//! [`TemplateError`]s.

mod capability;
mod context;
mod effects;
mod engine;
mod error;
mod function;
mod js;
mod parse;

pub use context::{Context, Options};
pub use effects::{Effects, SystemEffects};
pub use engine::{
    resolve_template, resolve_template_opts, resolve_value, resolve_value_for_node,
    resolve_value_opts,
};
pub use error::TemplateError;
pub use function::{TemplateFunction, builtins};
