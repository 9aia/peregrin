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
//! | `ac-runtime` | Concrete value type — `NonFunctionValue` + `Frontend` trait. |
//! | `ac-vm` | Execution engine — VM, host, functions, context. |
//! | `ac-frontend-json` | JSON frontend — implements `Frontend`. |
//! | `ac-frontend-yaml` | YAML frontend — implements `Frontend`. |
//! | `ac-frontend-toml` | TOML frontend — implements `Frontend`. |
//! | `ac-dialect` | Dialect abstraction — `Dialect` trait, `Schema<V>`, `SchemaVocabulary<V>`, `ValidationError`. |
//! | `ac-dialect-canonical` | Canonical dialect impl — `AcDialect`, `AC_DIALECT`, `JsonSchema`, `JsonSchemaVocabulary`. |
//! | `ac-fields` | Field extraction, interpolation, function calls, undeclared-field detection. |
//! | `ac-source` | Source location tracking — `SourceLocation`, `SourceMap`. |
//! | `ac-host` | Default host implementation — built-in functions (`add`, `mul`, `concat`, `len`). |
//! | `ac-compiler` | Compilation pipeline — parse → extract → resolve → typecheck → generate. |
//! | `ac-compiler-core` | Pipeline core — `Config`, `Pass`, `Pipeline`, `Compilation`, `Source`. |
//! | `ac-module-resolver-core` | Module resolution trait — `ModuleResolver`, `CachingResolver`. |
//! | `ac-module-resolver-memory` | In-memory `ModuleCache` implementation. |
//! | `ac-module-resolver-noop` | No-op `ModuleCache` implementation. |
//! | `ac-module-resolver-file` | Filesystem `ModuleResolver` implementation. |
//! | `ac-module-resolver-virtual` | In-memory `ModuleResolver` implementation. |
//! | `ac-typecheck-core` | Typechecking primitives — `Type`, `TypeEnv`, `SchemaInterpreter`. |
//! | `ac-typecheck-schema-json` | JSON Schema → `Type` interpreter. |
//! | `ac-typegen-core` | Type generation core — `TypegenContext`, `TypegenBackend`. |
//! | `ac-typegen-backend-json-schema` | JSON Schema backend for typegen. |
//! | `ac-cli` | CLI binary (`ac`) — parse, evaluate, validate. |
//! | `ac-lang` | This facade — re-exports everything. |
//!
//! ## Dependency direction
//!
//! ```text
//!                          ac-lang (facade)
//!                /      /      |       |       \      \
//!           ac-compiler  ac-vm  f-json  f-yaml  f-toml  ac-dialect
//!              / \       |      |       |       |        |
//!   ac-resolver  ac-typecheck-*  ac-runtime ◄──┴───────┘
//!      |              |              |
//!   ac-resolver-*  ac-typegen-*    ac-ast (no deps)
//!                     |
//!                  ac-host
//! ```
//!
//! `ac-ast` has no dependencies — it is generic over the value
//! type. `ac-runtime` provides the concrete `NonFunctionValue` wrapping
//! `serde_json::Value`. The compiler, typecheck, typegen, and resolver
//! crates build on top of the core layers to provide the full pipeline.

pub use ac_ast as core;
pub use ac_dialect as dialect;
pub use ac_runtime as runtime;
pub use ac_source as source;

#[cfg(feature = "compiler")]
pub use ac_compiler as compiler;
#[cfg(feature = "compiler-core")]
pub use ac_compiler_core as compiler_core;
#[cfg(feature = "dialect-canonical")]
pub use ac_dialect_canonical as dialect_canonical;
#[cfg(feature = "fields")]
pub use ac_fields as fields;
#[cfg(feature = "json")]
pub use ac_frontend_json as json;
#[cfg(feature = "toml")]
pub use ac_frontend_toml as toml;
#[cfg(feature = "yaml")]
pub use ac_frontend_yaml as yaml;
#[cfg(feature = "host")]
pub use ac_host as host;
#[cfg(feature = "resolver-core")]
pub use ac_module_resolver_core as resolver_core;
#[cfg(feature = "resolver-file")]
pub use ac_module_resolver_file as resolver_file;
#[cfg(feature = "resolver-memory")]
pub use ac_module_resolver_memory as resolver_memory;
#[cfg(feature = "resolver-noop")]
pub use ac_module_resolver_noop as resolver_noop;
#[cfg(feature = "resolver-virtual")]
pub use ac_module_resolver_virtual as resolver_virtual;
#[cfg(feature = "typecheck")]
pub use ac_typecheck_core as typecheck_core;
#[cfg(feature = "typecheck")]
pub use ac_typecheck_schema_json as typecheck_schema_json;
#[cfg(feature = "typegen")]
pub use ac_typegen_backend_json_schema as typegen_json_schema;
#[cfg(feature = "typegen")]
pub use ac_typegen_core as typegen_core;
#[cfg(feature = "vm")]
pub use ac_vm as vm;

/// Convenience prelude.
///
/// ```ignore
/// use ac_lang::prelude::*;
/// ```
pub mod prelude {
    pub use ac_ast::{Expr, Field};
    pub use ac_dialect::{Schema, ValidationError};
    pub use ac_runtime::{Frontend, HostValue, NonFunctionValue};
    pub use ac_source::{SourceLocation, SourceMap};

    #[cfg(feature = "compiler")]
    pub use ac_compiler::{Compilation, Config, Pass, Pipeline, Source};
    #[cfg(feature = "compiler")]
    pub use ac_compiler::{
        default_cli_config, full_pipeline, typecheck_only_config, typecheck_pipeline,
        typegen_pipeline,
    };
    #[cfg(feature = "dialect-canonical")]
    pub use ac_dialect_canonical::JsonSchema;
    #[cfg(feature = "json")]
    pub use ac_frontend_json;
    #[cfg(feature = "toml")]
    pub use ac_frontend_toml;
    #[cfg(feature = "yaml")]
    pub use ac_frontend_yaml;
    #[cfg(feature = "host")]
    pub use ac_host::DefaultHost;
    #[cfg(feature = "resolver-core")]
    pub use ac_module_resolver_core::{
        CachingResolver, ChainResolver, ModuleCache, ModuleResolver, ResolvedModule,
    };
    #[cfg(feature = "resolver-file")]
    pub use ac_module_resolver_file::FileResolver;
    #[cfg(feature = "resolver-memory")]
    pub use ac_module_resolver_memory::MemoryModuleCache;
    #[cfg(feature = "resolver-noop")]
    pub use ac_module_resolver_noop::NoCache;
    #[cfg(feature = "resolver-virtual")]
    pub use ac_module_resolver_virtual::VirtualResolver;
    #[cfg(feature = "typecheck")]
    pub use ac_typecheck_core::{SchemaInterpreter, Type, TypeEnv, TypeError, TypeResult};
    #[cfg(feature = "typecheck")]
    pub use ac_typecheck_schema_json::JsonSchemaInterpreter;
    #[cfg(feature = "typegen")]
    pub use ac_typegen_backend_json_schema::JsonSchemaBackend;
    #[cfg(feature = "typegen")]
    pub use ac_typegen_core::{TypegenBackend, TypegenContext};
    #[cfg(feature = "vm")]
    pub use ac_vm::{Context, Error, Function, Host, Vm};
}
