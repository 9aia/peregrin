//! Shared test stub for the dialect facade.

use ac_runtime::RuntimeValue;

use crate::dialect::Dialect;
use crate::document_vocabulary::DocumentVocabulary;
use crate::expr_parser::ExprParser;
use crate::schema::SchemaVocabulary;
use crate::syntax::Syntax;

use crate::schema::schema_vocabulary_stub::StubVocabulary;

/// A minimal dialect implementation for testing.
pub(crate) struct StubDialect;

impl Dialect for StubDialect {
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
        &StubVocabulary
    }
}
