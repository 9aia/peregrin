//! # ac-typecheck-core
//!
//! Typechecking primitives — schema-system-agnostic.
//!
//! The core provides **generic types and traits** for static type
//! checking of AC documents. It has zero knowledge of any AC-specific
//! key (`expr`, `arg`, `merge`, `inheritable`, `mutable`).
//!
//! AC key checks live in the `ac-typecheck-keys` crate, where each key
//! is a module (`arg`, `expr`, `merge`, `inherit`, `mutable`,
//! `termination`) that uses these primitives for its specific checks.
//! The orchestrator composes them.
//!
//! ## Architectural rule
//!
//! > **The core knows nothing about AC keys, the schema language, or
//! > the filesystem.**
//!
//! ## Dependency graph
//!
//! ```text
//! ac-typecheck-core — generic types, traits, no AC key knowledge
//! ├── ac-typecheck-infer           — expression type inference
//! ├── ac-typecheck-keys            — AC key check modules
//! │   ├── arg                      — checks "arg" key
//! │   ├── expr                     — checks "expr" key
//! │   ├── inherit                  — checks "inheritable" key
//! │   ├── merge                    — checks "merge" key
//! │   ├── mutable                  — checks "mutable" key
//! │   └── termination              — checks "termination" key
//! ├── ac-typecheck-import          — cross-module type checking (via resolver)
//! └── ac-typecheck-schema-json     — JSON Schema → Type mapping
//!
//! ac-module-resolver-core — ModuleResolver trait (shared with typegen)
//! ```

mod compat;
mod env;
mod error;
mod interpreter;
mod trait_registry;
mod type_;
mod unify;

pub use compat::{is_compatible, variance_compatible};
pub use env::TypeEnv;
pub use error::{CallCheckError, CallErrorKind, TypeError, TypeResult};
pub use interpreter::SchemaInterpreter;
pub use trait_registry::TraitRegistry;
pub use type_::{FunctionType, Kind, Type, TypeParam, Variance};
pub use unify::{Substitution, check_bounds, substitute, unify};

#[cfg(test)]
mod lib_tests;
