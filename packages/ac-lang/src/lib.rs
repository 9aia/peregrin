//! # ac-lang
//!
//! A batteries-included facade for the AC language.
//!
//! This crate re-exports the core types, VM, format frontends,
//! and schema validation in a single module tree. It is the
//! recommended entry point for most users.
//!
//! ## Crate structure
//!
//! | Crate | Role |
//! |---|---|
//! | `ac-ast` | Language representation — expressions, fields (generic over value type). |
//! | `ac-runtime` | Concrete value type — `RuntimeValue` + `Frontend` trait. |
//! | `ac-vm` | Execution engine — VM, host, functions, context. |
//! | `ac-frontend-json` | JSON frontend — implements `Frontend`. |
//! | `ac-frontend-yaml` | YAML frontend — implements `Frontend`. |
//! | `ac-frontend-toml` | TOML frontend — implements `Frontend`. |
//! | `ac-schema` | Abstract `Schema<V>` trait — zero deps. |
//! | `ac-schema-json` | JSON Schema implementation of `Schema<RuntimeValue>`. |
//! | `ac-fields` | Field extraction, interpolation, function calls, undeclared-field detection. |
//! | `ac-cli` | CLI binary (`ac`) — parse, evaluate, validate. |
//! | `ac-lang` | This facade — re-exports everything. |
//!
//! ## Dependency direction
//!
//! ```text
//!                    ac-lang (facade)
//!                 /   |    |    |    \
//!            ac-vm  f-json  f-yaml  f-toml  ac-schema
//!               |     |    |    |        |
//!          ac-runtime ◄──┴────┴────────┘
//!               |
//!            ac-ast (no deps)
//! ```
//!
//! `ac-ast` has no dependencies — it is generic over the value
//! type. `ac-runtime` provides the concrete `RuntimeValue` wrapping
//! `serde_json::Value`. All other crates depend on one or both.

pub use ac_ast as core;
pub use ac_runtime as runtime;
pub use ac_frontend_json as json;
pub use ac_frontend_toml as toml;
pub use ac_frontend_yaml as yaml;
pub use ac_schema as schema;
pub use ac_schema_json as schema_json;
pub use ac_vm as vm;
pub use ac_fields as fields;

/// Convenience prelude.
///
/// ```ignore
/// use ac_lang::prelude::*;
/// ```
pub mod prelude {
    pub use ac_ast::{Expr, Field};
    pub use ac_runtime::{Frontend, HostValue, RuntimeValue};
    pub use ac_frontend_json;
    pub use ac_frontend_toml;
    pub use ac_frontend_yaml;
    pub use ac_schema::{Schema, ValidationError};
    pub use ac_schema_json::JsonSchema;
    pub use ac_vm::{Context, Error, Function, Host, Vm};
}
