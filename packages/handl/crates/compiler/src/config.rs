//! Pipeline configuration — holds the pluggable components.
//!
//! Each component is a trait object, so callers can swap implementations:
//!
//! - `resolver`: how modules are loaded (`handl-module-resolver-file`, `handl-module-resolver-virtual`, `handl-module-resolver-http`, `handl-module-resolver-git`)
//! - `interpreter`: how schema fragments become types (`handl-typecheck-schema-json`, future custom)
//! - `backend`: how types are emitted (`handl-typegen-backend-json-schema`, future TypeScript, Cue)
//!
//! Default configurations (`default_cli`, `typecheck_only`) are provided
//! by `handl-compiler`, which depends on the implementation crates.
//!
//! Custom passes can read their own config via [`Config::extension`]
//! without modifying this struct.

use std::any::{Any, TypeId};
use std::collections::HashMap;

use handl_dialect::Dialect;
use handl_module_resolver_core::ModuleResolver;
use handl_typecheck_core::SchemaInterpreter;
use handl_typegen_core::{ImportResolver, KeyHandler, TypegenBackend};
use serde_json::Value as JsonValue;

/// Configuration for a compilation pipeline.
pub struct Config {
    pub dialect: Box<dyn Dialect>,
    pub resolver: Box<dyn ModuleResolver>,
    pub interpreter: Box<dyn SchemaInterpreter>,
    pub backend: Option<Box<dyn TypegenBackend<Output = JsonValue>>>,
    pub key_handler: Box<dyn KeyHandler>,
    pub import_resolver: Box<dyn ImportResolver>,
    /// Extensible typed config — custom passes can read their config
    /// via [`Config::extension`] without modifying this struct.
    pub extensions: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl Config {
    /// Borrow a typed extension previously added via [`Config::with_extension`].
    pub fn extension<T: 'static>(&self) -> Option<&T> {
        self.extensions
            .get(&TypeId::of::<T>())
            .and_then(|v| v.downcast_ref::<T>())
    }

    /// Add a typed extension. Enables custom passes to receive config
    /// without modifying `Config`.
    pub fn with_extension<T: 'static + Send + Sync>(mut self, ext: T) -> Self {
        self.extensions.insert(TypeId::of::<T>(), Box::new(ext));
        self
    }
}
