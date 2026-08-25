use handl_dialect::DocumentVocabulary;

use handl_typegen_core::TypegenContext;

/// Strip the `mutable` key from all field schemas in the context.
pub fn strip_key(ctx: &mut TypegenContext, vocab: &dyn DocumentVocabulary) {
    ctx.strip_keys(&[vocab.mutable_key()]);
}
