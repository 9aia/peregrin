use serde_json::Value as JsonValue;

use crate::Type;

/// Interprets schema fragments as types.
///
/// Each schema language (JSON Schema, Cue, custom) implements this trait
/// to convert its schema vocabulary into the neutral [`Type`] enum.
/// The typechecker uses the interpreter to build [`TypeEnv`]s from
/// raw field schemas.
///
/// Implementations:
/// - `handl-typecheck-schema-json` — JSON Schema → Type
/// - Future: Cue, custom schema languages
pub trait SchemaInterpreter: Send + Sync {
    /// Interpret a raw schema fragment as a [`Type`].
    fn interpret(&self, schema: &JsonValue) -> Type;
}
