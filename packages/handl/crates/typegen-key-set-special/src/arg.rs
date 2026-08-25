use handl_dialect::DocumentVocabulary;
use serde_json::Value as JsonValue;

use handl_typegen_core::TypegenContext;

/// Check if a field schema has an `arg` key.
pub fn has_arg(schema: &JsonValue, vocab: &dyn DocumentVocabulary) -> bool {
    schema.get(vocab.arg_key()).is_some()
}

/// Strip the `arg` key from all field schemas in the context.
pub fn strip_key(ctx: &mut TypegenContext, vocab: &dyn DocumentVocabulary) {
    ctx.strip_keys(&[vocab.arg_key()]);
}
