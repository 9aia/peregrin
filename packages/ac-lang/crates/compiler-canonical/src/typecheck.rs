//! Static type checking for AC documents.
//!
//! Delegates to `typecheck_pipeline` — parse → extract → typecheck.

use std::path::Path;

use ac_typecheck_core::TypeError;

use crate::{DriverError, Session, Source, typecheck_only_config, typecheck_pipeline};

/// Report from a typecheck run.
pub struct TypecheckReport {
    pub errors: Vec<TypeError>,
}

impl TypecheckReport {
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }
}

pub fn run(
    input: &str,
    format: &str,
    base_path: Option<&Path>,
) -> Result<TypecheckReport, DriverError> {
    let mut source = Source::new(input, format);
    if let Some(base) = base_path {
        source = source.with_path(base);
    }

    let mut driver = typecheck_pipeline();
    let mut session = Session::new(typecheck_only_config());
    let result = driver.run(source, &mut session)?;

    Ok(TypecheckReport {
        errors: result.type_errors,
    })
}
