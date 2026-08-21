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
pub use ac_compiler as compiler;
pub use ac_compiler_core as compiler_core;
pub use ac_dialect as dialect;
pub use ac_dialect_canonical as dialect_canonical;
pub use ac_fields as fields;
pub use ac_frontend_json as json;
pub use ac_frontend_toml as toml;
pub use ac_frontend_yaml as yaml;
pub use ac_host as host;
pub use ac_module_resolver_core as resolver_core;
pub use ac_module_resolver_file as resolver_file;
pub use ac_module_resolver_memory as resolver_memory;
pub use ac_module_resolver_noop as resolver_noop;
pub use ac_module_resolver_virtual as resolver_virtual;
pub use ac_runtime as runtime;
pub use ac_source as source;
pub use ac_typecheck_core as typecheck_core;
pub use ac_typecheck_schema_json as typecheck_schema_json;
pub use ac_typegen_backend_json_schema as typegen_json_schema;
pub use ac_typegen_core as typegen_core;
pub use ac_vm as vm;

/// Convenience prelude.
///
/// ```ignore
/// use ac_lang::prelude::*;
/// ```
pub mod prelude {
    pub use ac_ast::{Expr, Field};
    pub use ac_compiler::{Compilation, Config, Pass, Pipeline, Source};
    pub use ac_compiler::{
        default_cli_config, full_pipeline, typecheck_only_config, typecheck_pipeline,
        typegen_pipeline,
    };
    pub use ac_dialect::{Schema, ValidationError};
    pub use ac_dialect_canonical::JsonSchema;
    pub use ac_frontend_json;
    pub use ac_frontend_toml;
    pub use ac_frontend_yaml;
    pub use ac_host::DefaultHost;
    pub use ac_module_resolver_core::{
        CachingResolver, ChainResolver, ModuleCache, ModuleResolver, ResolvedModule,
    };
    pub use ac_module_resolver_file::FileResolver;
    pub use ac_module_resolver_memory::MemoryModuleCache;
    pub use ac_module_resolver_noop::NoCache;
    pub use ac_module_resolver_virtual::VirtualResolver;
    pub use ac_runtime::{Frontend, HostValue, NonFunctionValue};
    pub use ac_source::{SourceLocation, SourceMap};
    pub use ac_typecheck_core::{SchemaInterpreter, Type, TypeEnv, TypeError, TypeResult};
    pub use ac_typecheck_schema_json::JsonSchemaInterpreter;
    pub use ac_typegen_backend_json_schema::JsonSchemaBackend;
    pub use ac_typegen_core::{TypegenBackend, TypegenContext};
    pub use ac_vm::{Context, Error, Function, Host, Vm};
}
