//! Compilation intermediate representation.
//!
//! A [`Compilation`] flows through the pipeline, accumulating state
//! as each pass runs. Fields are `Option`-wrapped because not all
//! passes produce all state — a typecheck-only run never populates
//! `output`, for example.

use std::path::PathBuf;

use ac_fields::ExtractedFields;
use ac_runtime::RuntimeValue;
use ac_typecheck_core::TypeError;
use ac_typegen_core::TypegenContext;
use serde_json::Value as JsonValue;

/// Source input for a compilation.
#[derive(Clone)]
pub struct Source {
    /// Raw input text.
    pub input: String,
    /// Format name (`yaml`, `json`, `toml`).
    pub format: String,
    /// File path of the input, if known. Used for relative import resolution.
    pub path: Option<PathBuf>,
}

impl Source {
    pub fn new(input: impl Into<String>, format: impl Into<String>) -> Self {
        Source {
            input: input.into(),
            format: format.into(),
            path: None,
        }
    }

    pub fn with_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Base path for resolving relative imports — the parent directory
    /// of the source file, if known.
    pub fn base_path(&self) -> Option<PathBuf> {
        self.path.as_ref().and_then(|p| p.parent().map(|p| p.to_path_buf()))
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
    pub parsed: Option<RuntimeValue>,
    pub extracted: Option<ExtractedFields>,
    pub typegen_ctx: Option<TypegenContext>,
    pub type_errors: Vec<TypeError>,
    pub output: Option<JsonValue>,
}

impl Compilation {
    pub fn new(source: Source) -> Self {
        Compilation {
            source,
            parsed: None,
            extracted: None,
            typegen_ctx: None,
            type_errors: Vec::new(),
            output: None,
        }
    }

    /// Convenience: borrow the parsed value, if present.
    pub fn parsed(&self) -> Option<&RuntimeValue> {
        self.parsed.as_ref()
    }

    /// Convenience: borrow the extracted fields, if present.
    pub fn extracted(&self) -> Option<&ExtractedFields> {
        self.extracted.as_ref()
    }
}
