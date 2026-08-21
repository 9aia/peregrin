/// Document vocabulary — the reserved key names used in AC documents.
///
/// These keys have special meaning during field extraction, type checking,
/// and type generation. A custom dialect may use different key names
/// (e.g. `fields` → `properties`, `expr` → `compute`).
///
/// This trait defines the interface only. Canonical key values are
/// implemented in the `ac-dialect-canonical` crate.
pub trait DocumentVocabulary: Send + Sync {
    /// The top-level key for field declarations.
    fn fields_key(&self) -> &str;

    /// The top-level key for function declarations.
    fn functions_key(&self) -> &str;

    /// The field-level key for computed field expressions.
    fn expr_key(&self) -> &str;

    /// The field-level key for parameterized computed field argument type.
    fn arg_key(&self) -> &str;

    /// The field-level key for merge field flag.
    fn merge_key(&self) -> &str;

    /// The field-level key for scoped inheritance flag.
    fn inheritable_key(&self) -> &str;

    /// The field-level key for immutability flag.
    fn mutable_key(&self) -> &str;

    /// The function-level key for parameters.
    fn params_key(&self) -> &str;

    /// The function-level key for return schema.
    fn return_key(&self) -> &str;

    /// The function-level key for user-defined function body.
    fn body_key(&self) -> &str;

    /// All field-level reserved keys (used for stripping during typegen).
    fn reserved_field_keys(&self) -> Vec<&str> {
        vec![
            self.expr_key(),
            self.arg_key(),
            self.merge_key(),
            self.inheritable_key(),
            self.mutable_key(),
        ]
    }

    /// Reserved top-level keys that cannot be declared as field names.
    fn top_level_reserved_keys(&self) -> Vec<&str> {
        vec![self.fields_key(), self.functions_key()]
    }
}
