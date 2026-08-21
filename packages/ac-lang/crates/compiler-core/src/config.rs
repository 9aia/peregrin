//! Pipeline configuration — holds the pluggable components.
//!
//! Each component is a trait object, so callers can swap implementations:
//!
//! - `resolver`: how modules are loaded (`ac-module-resolver-file`, `ac-module-resolver-virtual`, `ac-module-resolver-http`, `ac-module-resolver-git`)
//! - `interpreter`: how schema fragments become types (`ac-typecheck-schema-json`, future custom)
//! - `backend`: how types are emitted (`ac-typegen-backend-json-schema`, future TypeScript, Cue)
//!
//! Default configurations (`default_cli`, `typecheck_only`) are provided
//! by `ac-compiler`, which depends on the implementation crates.

use ac_dialect::Dialect;
use ac_module_resolver_core::ModuleResolver;
use ac_typecheck_core::SchemaInterpreter;
use ac_typegen_core::{ImportResolver, KeyHandler, TypegenBackend};
use serde_json::Value as JsonValue;

/// Configuration for a compilation pipeline.
pub struct Config {
    pub dialect: Box<dyn Dialect>,
    pub resolver: Box<dyn ModuleResolver>,
    pub interpreter: Box<dyn SchemaInterpreter>,
    pub backend: Option<Box<dyn TypegenBackend<Output = JsonValue>>>,
    pub key_handler: Box<dyn KeyHandler>,
    pub import_resolver: Box<dyn ImportResolver>,
}
