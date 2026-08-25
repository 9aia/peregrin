use crate::context::TypegenContext;

/// A type definition generator. Format-specific.
///
/// The core produces clean `JsonValue` fragments. Each backend interprets
/// them and produces its target format.
///
/// - `handl-typegen-json` → JSON Schema (`JsonValue`)
/// - `handl-typegen-typescript` → TypeScript `.d.ts` (string)
/// - Future: Rust structs, Python TypedDict, etc.
pub trait TypegenBackend: Send + Sync {
    type Output;
    /// Generate a single document's type definition.
    fn generate(&self, ctx: &TypegenContext) -> Self::Output;
    /// File extension for emitted files (e.g. "schema.json", "d.ts").
    fn extension(&self) -> &str;
    /// IDE hint comment to inject into source files (e.g.
    /// `# yaml-language-server: $schema={path}`).
    fn ide_hint(&self, schema_rel_path: &str) -> String;
}