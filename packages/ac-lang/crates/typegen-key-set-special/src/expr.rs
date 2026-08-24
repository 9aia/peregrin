use ac_dialect::DocumentVocabulary;
use serde_json::Value as JsonValue;

use ac_typegen_core::TypegenContext;

/// Get the expression string from a field schema, if present.
pub fn get_expr(schema: &JsonValue, vocab: &dyn DocumentVocabulary) -> Option<String> {
    schema
        .get(vocab.expr_key())
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// Strip the `expr` key from all field schemas in the context.
pub fn strip_key(ctx: &mut TypegenContext, vocab: &dyn DocumentVocabulary) {
    ctx.strip_keys(&[vocab.expr_key()]);
}