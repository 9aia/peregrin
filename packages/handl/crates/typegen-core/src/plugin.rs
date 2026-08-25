use std::path::Path;

use handl_dialect::DocumentVocabulary;
use handl_module_resolver_core::ModuleResolver;
use serde_json::Value as JsonValue;

use crate::TypegenContext;

/// Handles special keys in field schemas during type generation.
///
/// Implementations know which keys are special and how to process them
/// (filter merge fields, strip special keys). The core provides generic
/// primitives (`TypegenContext::remove_fields`, `TypegenContext::strip_keys`);
/// the handler decides which keys to act on and when.
///
/// The canonical implementation lives in `handl-typegen-key-set-special`.
pub trait KeyHandler: Send + Sync {
    /// Remove fields that should be excluded from the output schema
    /// (e.g. merge fields that are spread into the parent at runtime).
    fn filter_fields(&self, ctx: &mut TypegenContext, merge_names: &[String]);

    /// Strip all special keys from field schemas in the context.
    fn strip_keys(&self, ctx: &mut TypegenContext, vocab: &dyn DocumentVocabulary);
}

/// Resolves imports in field schemas during type generation.
///
/// Implementations detect import expressions in merge fields, follow
/// import paths via a [`ModuleResolver`], and merge imported schemas
/// into the parent context (parent wins on conflicts).
///
/// The canonical implementation lives in `handl-typegen-import`.
pub trait ImportResolver: Send + Sync {
    /// Detect and resolve imports from merge fields, merging imported
    /// schemas into the context.
    #[allow(clippy::too_many_arguments)]
    fn resolve_imports(
        &self,
        ctx: &mut TypegenContext,
        schemas: &[(String, JsonValue)],
        merge_fields: &[String],
        data: &JsonValue,
        resolver: &dyn ModuleResolver,
        base: Option<&Path>,
        vocab: &dyn DocumentVocabulary,
    );
}

/// Read the expression string from a field schema, if present.
///
/// Uses the vocabulary's `expr_key` to look up the expression value.
pub fn get_expr(schema: &JsonValue, vocab: &dyn DocumentVocabulary) -> Option<String> {
    schema
        .get(vocab.expr_key())
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}
