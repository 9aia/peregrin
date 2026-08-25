use handl_dialect::{Dialect, DocumentVocabulary, ExprParser, SchemaVocabulary, Syntax};
use handl_vm_values_json::NonFunctionValue;

use crate::schema_vocabulary::JSON_SCHEMA_VOCABULARY;

/// The canonical handl language dialect.
///
/// Implements the syntax defined in the handl Language Specification.
/// See the crate-level documentation for the full list of canonical values.
#[derive(Clone, Copy)]
pub struct HandlDialect;

impl Dialect for HandlDialect {
    fn syntax(&self) -> &dyn Syntax {
        self
    }

    fn parser(&self) -> &dyn ExprParser {
        self
    }

    fn document_vocabulary(&self) -> &dyn DocumentVocabulary {
        self
    }

    fn schema_vocabulary(&self) -> &dyn SchemaVocabulary<NonFunctionValue> {
        &JSON_SCHEMA_VOCABULARY
    }
}

/// The canonical handl dialect as a static singleton.
pub static AC_DIALECT: HandlDialect = HandlDialect;
