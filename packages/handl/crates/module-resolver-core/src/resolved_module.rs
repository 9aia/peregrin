use std::path::PathBuf;

use serde_json::Value as JsonValue;

/// A resolved module — the result of a `ModuleResolver::resolve` call.
///
/// Contains everything needed to merge imported schemas into a parent
/// context and recurse into nested imports.
#[derive(Clone)]
pub struct ResolvedModule {
    /// Field name → raw schema fragment (handl keys still present).
    pub schemas: Vec<(String, JsonValue)>,
    /// Field names marked with `merge: true`.
    pub merge: Vec<String>,
    /// The document's data values (for data-driven import resolution).
    pub data: JsonValue,
    /// Base path for resolving nested imports from this module.
    pub base_path: Option<PathBuf>,
}
