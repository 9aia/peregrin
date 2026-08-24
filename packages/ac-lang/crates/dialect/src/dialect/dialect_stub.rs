//! Shared test stub for the dialect facade.

use ac_runtime_values::NonFunctionValue;

use crate::dialect::Dialect;
use crate::document_vocabulary::DocumentVocabulary;
use crate::expr_parser::ExprParser;
use crate::schema_vocabulary::SchemaVocabulary;
use crate::syntax::Syntax;

use crate::schema_vocabulary::schema_vocabulary_stub::StubVocabulary;

/// A minimal dialect implementation for testing.
pub struct StubDialect;

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

    fn schema_vocabulary(&self) -> &dyn SchemaVocabulary<NonFunctionValue> {
        &StubVocabulary
    }
}

/// A static [`StubDialect`] instance for convenience.
pub static STUB_DIALECT: StubDialect = StubDialect;

/// Access the stub's [`DocumentVocabulary`] without importing the [`Dialect`] trait.
pub fn stub_vocab() -> &'static dyn DocumentVocabulary {
    STUB_DIALECT.document_vocabulary()
}
