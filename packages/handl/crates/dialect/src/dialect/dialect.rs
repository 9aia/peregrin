use handl_vm_values_json::NonFunctionValue;

use crate::document_vocabulary::DocumentVocabulary;
use crate::expr_parser::ExprParser;
use crate::schema_vocabulary::SchemaVocabulary;
use crate::syntax::Syntax;

/// A dialect for parsing and evaluating handl expressions and documents.
///
/// The handl language specification defines one canonical syntax. This trait
/// abstracts the syntax decisions so the implementation can be structured
/// around a single canonical dialect, with the architecture allowing others
/// if needed.
///
/// `Dialect` is a facade that groups four sub-traits:
///
/// - [`Syntax`] — interpolation markers, path separator, call delimiters.
/// - [`ExprParser`] — call detection, argument splitting, literal parsing.
/// - [`DocumentVocabulary`] — reserved key names (`fields`, `expr`, etc.).
/// - [`SchemaVocabulary`] — schema language for field and function validation.
///
/// Consumers that only need one aspect can depend on the sub-trait directly.
/// The facade provides a convenient "give me the whole language configuration"
/// accessor for call sites that need multiple aspects.
pub trait Dialect: Send + Sync {
    /// The syntax tokens — interpolation markers, path separator, call delimiters.
    fn syntax(&self) -> &dyn Syntax;

    /// The expression parser — call detection, argument splitting, literal parsing.
    fn parser(&self) -> &dyn ExprParser;

    /// The document vocabulary — reserved key names used in handl documents.
    fn document_vocabulary(&self) -> &dyn DocumentVocabulary;

    /// The schema vocabulary used for field and function validation.
    ///
    /// The canonical dialect returns a JSON Schema draft 2020-12 vocabulary.
    fn schema_vocabulary(&self) -> &dyn SchemaVocabulary<NonFunctionValue>;
}
