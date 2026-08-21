use ac_dialect::{Dialect, DocumentVocabulary, ExprParser, SchemaVocabulary, Syntax};
use ac_runtime::RuntimeValue;

use crate::schema_vocabulary::JSON_SCHEMA_VOCABULARY;

/// The canonical AC language dialect.
///
/// Implements the syntax defined in the AC Language Specification.
/// See the crate-level documentation for the full list of canonical values.
#[derive(Clone, Copy)]
pub struct AcDialect;

impl Dialect for AcDialect {
    fn syntax(&self) -> &dyn Syntax {
        self
    }

    fn parser(&self) -> &dyn ExprParser {
        self
    }

    fn document_vocabulary(&self) -> &dyn DocumentVocabulary {
        self
    }

    fn schema_vocabulary(&self) -> &dyn SchemaVocabulary<RuntimeValue> {
        &JSON_SCHEMA_VOCABULARY
    }
}

/// The canonical AC dialect as a static singleton.
pub static AC_DIALECT: AcDialect = AcDialect;
