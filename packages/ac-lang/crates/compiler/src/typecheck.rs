//! Static type checking for AC documents.
//!
//! Delegates to [`Pipeline::typecheck`] — parse → extract → typecheck.

use std::path::Path;

use ac_typecheck_core::TypeError;

use crate::{PassError, Source};

/// Report from a typecheck run.
pub struct TypecheckReport {
    pub errors: Vec<TypeError>,
}

impl TypecheckReport {
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }
}

pub fn run(input: &str, format: &str, base_path: Option<&Path>) -> Result<TypecheckReport, PassError> {
    let mut source = Source::new(input, format);
    if let Some(base) = base_path {
        source = source.with_path(base);
    }

    let pipeline = crate::typecheck_pipeline(crate::typecheck_only_config());
    let compilation = pipeline.run(source)?;

    Ok(TypecheckReport {
        errors: compilation.type_errors,
    })
}
