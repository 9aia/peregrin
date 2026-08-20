//! Pipeline configuration — holds the pluggable components.
//!
//! Each component is a trait object, so callers can swap implementations:
//!
//! - `resolver`: how modules are loaded (`ac-resolver-file`, `ac-resolver-virtual`, future git/http)
//! - `interpreter`: how schema fragments become types (`ac-typecheck-schema-json`, future custom)
//! - `backend`: how types are emitted (`ac-typegen-backend-json-schema`, future TypeScript, Cue)
//!
//! Default configurations (`default_cli`, `typecheck_only`) are provided
//! by `ac-engine`, which depends on the implementation crates.

use ac_resolver_core::ModuleResolver;
use ac_typecheck_core::SchemaInterpreter;
use ac_typegen_core::TypegenBackend;
use serde_json::Value as JsonValue;

/// Configuration for a compilation pipeline.
pub struct Config {
    pub resolver: Box<dyn ModuleResolver>,
    pub interpreter: Box<dyn SchemaInterpreter>,
    pub backend: Option<Box<dyn TypegenBackend<Output = JsonValue>>>,
}
