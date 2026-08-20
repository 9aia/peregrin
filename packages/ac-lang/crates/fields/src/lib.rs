//! # ac-fields
//!
//! Field extraction, interpolation, and validation orchestration
//! for self-describing documents.
//!
//! This crate provides the domain logic for documents that declare
//! their own field schemas inline using a `fields:` section. It
//! handles:
//!
//! - Extracting the `fields:` and `functions:` sections from data.
//! - `${...}` interpolation supporting:
//!   - Namespace paths: `${doc.name}`, `${env.HOME}`, `${doc.author.email}`
//!   - Shorthand paths: `${name}` (equivalent to `${doc.name}`)
//!   - Function calls: `${now()}`, `${add(1, 2)}`, `${concat(doc.name, "!")}`
//!   - Nested calls: `${add(mul(2, 3), 4)}`
//! - Typed/validated function params via the `Schema<V>` system.
//! - User-defined function bodies (`body:` field) that compose via
//!   host primitives — no Rust code needed for new functions.
//! - Scoped inheritance: `inheritable: true` fields propagate into nested
//!   objects; `mutable: false` fields cannot be overridden in child scopes.
//! - Detecting undeclared fields.
//!
//! The crate is schema-agnostic — it does not validate fields itself.
//! Callers pass raw schema values and choose their own `Schema<V>`
//! implementation.
//!
//! ## Architectural rule
//!
//! > **Field extraction and interpolation are domain logic, not CLI
//! > plumbing.** This crate keeps them testable and reusable.

mod types;
mod extraction;
mod interpolation;
mod expression;
mod path;
mod computed;
mod merge;
mod scope;
mod namespace;
mod validation;

pub use types::{
    ArgSpec, ComputedField, ExtractedFields, FunctionResolver, FunctionSignature,
    Namespaces, NoFunctions, DEFAULT_NAMESPACE, RESERVED_FIELDS,
};
pub use extraction::{extract_fields, undeclared_fields};
pub use interpolation::{interpolate, interpolate_scoped};
pub use expression::{eval_body, eval_expression};
pub use computed::compute_fields;
pub use merge::merge_fields;
pub use scope::validate_mutable;
pub use namespace::{build_doc_namespace, build_env_namespace};
pub use validation::{validate_args, validate_return};

#[cfg(test)]
mod test_helpers;

#[cfg(test)]
mod extraction_tests;

#[cfg(test)]
mod interpolation_tests;

#[cfg(test)]
mod expression_tests;

#[cfg(test)]
mod computed_tests;

#[cfg(test)]
mod merge_tests;

#[cfg(test)]
mod scope_tests;
