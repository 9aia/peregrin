use ac_dialect::DocumentVocabulary;
use serde_json::Value as JsonValue;

use ac_typegen_core::TypegenContext;

/// Check if a field schema has `merge: true`.
pub fn is_merge_field(schema: &JsonValue, vocab: &dyn DocumentVocabulary) -> bool {
    schema.get(vocab.merge_key()).and_then(|v| v.as_bool()).unwrap_or(false)
}

/// Remove merge fields from the context by name.
///
/// Merge fields are excluded from the output schema — they're spread
/// into the parent scope at runtime and removed.
pub fn filter_fields(ctx: &mut TypegenContext, merge_names: &[String]) {
    ctx.remove_fields(merge_names);
}

/// Strip the `merge` key from all field schemas in the context.
pub fn strip_key(ctx: &mut TypegenContext, vocab: &dyn DocumentVocabulary) {
    ctx.strip_keys(&[vocab.merge_key()]);
}
