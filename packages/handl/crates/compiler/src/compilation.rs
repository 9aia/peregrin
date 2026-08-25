//! Compilation intermediate representation.
//!
//! A [`Compilation`] flows through the pipeline, accumulating state
//! as each pass runs. Fields are `Option`-wrapped because not all
//! passes produce all state — a typecheck-only run never populates
//! `output`, for example.
//!
//! In addition to the named fields, passes can store arbitrary typed
//! data via the [`Compilation::store`] / [`Compilation::get`] /
//! [`Compilation::take`] API (backed by a `TypeId` map). This lets
//! custom passes carry state without modifying this struct.

use handl_vm_values_json::NonFunctionValue;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::path::PathBuf;

use handl_fields::ExtractedFields;
use handl_source::SourceMap;
use handl_typecheck_core::TypeError;
use handl_typegen_core::TypegenContext;
use serde_json::Value as JsonValue;

use crate::diagnostic::{Diagnostic, DiagnosticSink};

/// Source input for a compilation.
#[derive(Clone)]
pub struct Source {
    /// Raw input text.
    pub input: String,
    /// Format name (`yaml`, `json`, `toml`).
    pub format: String,
    /// File path of the input, if known. Used for relative import resolution.
    pub path: Option<PathBuf>}

impl Source {
    pub fn new(input: impl Into<String>, format: impl Into<String>) -> Self {
        Source {
            input: input.into(),
            format: format.into(),
            path: None}
    }

    pub fn with_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Base path for resolving relative imports — the parent directory
    /// of the source file, if known.
    pub fn base_path(&self) -> Option<PathBuf> {
        self.path
            .as_ref()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
    }
}

/// A compilation in progress — flows through the pipeline.
///
/// Each pass reads from and writes to this struct. Fields are populated
/// in pipeline order:
///
/// 1. `parsed` — after [`ParsePass`]
/// 2. `extracted` — after [`ExtractPass`]
/// 3. `type_errors` — after [`TypecheckPass`]
/// 4. `output` — after [`GeneratePass`]
///
/// Custom passes can read any populated field and write new ones.
pub struct Compilation {
    pub source: Source,
    pub parsed: Option<NonFunctionValue>,
    pub extracted: Option<ExtractedFields>,
    pub typegen_ctx: Option<TypegenContext>,
    pub type_errors: Vec<TypeError>,
    pub output: Option<JsonValue>,
    /// Source location map built by the frontend during parsing.
    /// Used to resolve error locations post-hoc.
    pub source_map: SourceMap,
    /// Structured diagnostics emitted by passes.
    pub diagnostics: DiagnosticSink,
    /// Extensible typed storage — passes can store arbitrary `T: 'static`
    /// without modifying this struct.
    store: HashMap<TypeId, Box<dyn Any + Send + Sync>>}

impl Compilation {
    pub fn new(source: Source) -> Self {
        Compilation {
            source,
            parsed: None,
            extracted: None,
            typegen_ctx: None,
            type_errors: Vec::new(),
            output: None,
            source_map: SourceMap::new(),
            diagnostics: DiagnosticSink::new(),
            store: HashMap::new()}
    }

    /// Convenience: borrow the parsed value, if present.
    pub fn parsed(&self) -> Option<&NonFunctionValue> {
        self.parsed.as_ref()
    }

    /// Convenience: borrow the extracted fields, if present.
    pub fn extracted(&self) -> Option<&ExtractedFields> {
        self.extracted.as_ref()
    }

    /// Resolve source locations for all type errors using the source map
    /// built by the frontend during parsing. Errors that already have a
    /// location are left as-is.
    pub fn resolve_error_locations(&mut self) {
        for err in &mut self.type_errors {
            if err.location.is_none() {
                err.location = self.source_map.resolve_path(&err.path);
            }
        }
    }

    // -- TypeMap storage -------------------------------------------------

    /// Store typed data. Passes can use this to carry state without
    /// modifying `Compilation`.
    pub fn store<T: 'static + Send + Sync>(&mut self, value: T) {
        self.store.insert(TypeId::of::<T>(), Box::new(value));
    }

    /// Borrow typed data previously stored via [`Compilation::store`].
    pub fn get<T: 'static>(&self) -> Option<&T> {
        self.store
            .get(&TypeId::of::<T>())
            .and_then(|v| v.downcast_ref::<T>())
    }

    /// Take ownership of typed data previously stored via [`Compilation::store`].
    /// Removes the value from the map.
    pub fn take<T: 'static + Send + Sync>(&mut self) -> Option<T> {
        self.store
            .remove(&TypeId::of::<T>())
            .and_then(|v| v.downcast::<T>().ok())
            .map(|b| *b)
    }

    /// Take ownership of typed data previously stored via [`Compilation::store`].
    /// Removes the value from the map. Alias for [`Compilation::take`].
    #[deprecated(note = "use `take` instead — same semantics")]
    pub fn remove<T: 'static + Send + Sync>(&mut self) -> Option<T> {
        self.take()
    }

    // -- Diagnostics -----------------------------------------------------

    /// Emit a structured diagnostic.
    pub fn emit(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    /// Whether any error-severity diagnostics have been emitted.
    pub fn has_errors(&self) -> bool {
        self.diagnostics.has_errors()
    }
}