use ac_resolver_core::IMPORT_FUNCTION;
use serde_json::Value as JsonValue;

/// The kind of import detected in a field's `expr`.
#[derive(Debug, Clone, PartialEq)]
pub enum ImportKind {
    /// `import('path/to/file.yml')` — static literal path.
    Static(String),
    /// `import(arg)` — path comes from the field's data value.
    DataDriven,
    /// No import detected, or dynamic expression that can't be resolved statically.
    None,
}

/// Detect import patterns in an expression string.
///
/// Recognized patterns:
/// - `import('...')` or `import("...")` → `ImportKind::Static(path)`
/// - `import(arg)` → `ImportKind::DataDriven`
/// - Anything else → `ImportKind::None`
pub fn detect_import_path(expr: &str) -> ImportKind {
    let trimmed = expr.trim();
    let import_prefix = format!("{IMPORT_FUNCTION}(");

    if trimmed == format!("{IMPORT_FUNCTION}(arg)") {
        return ImportKind::DataDriven;
    }

    if let Some(rest) = trimmed.strip_prefix(&import_prefix) {
        let rest = rest.trim_end();
        let rest = rest.strip_suffix(')').unwrap_or(rest);

        let rest = rest.trim();
        if let Some(inner) = rest.strip_prefix('\'').and_then(|r| r.strip_suffix('\'')) {
            return ImportKind::Static(inner.to_string());
        }
        if let Some(inner) = rest.strip_prefix('"').and_then(|r| r.strip_suffix('"')) {
            return ImportKind::Static(inner.to_string());
        }
    }

    ImportKind::None
}

/// Extract the data value for a data-driven import field.
///
/// Given the document data and a field name, returns the string value
/// of that field if it's a string. Used to resolve `import(arg)` paths.
pub fn extract_data_path(data: &JsonValue, field_name: &str) -> Option<String> {
    data.get(field_name)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}
