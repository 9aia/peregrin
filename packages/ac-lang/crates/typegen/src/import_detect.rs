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

    // Check for import(arg) — data-driven import.
    if trimmed == "import(arg)" {
        return ImportKind::DataDriven;
    }

    // Check for import('...') or import("...") — static import.
    if let Some(rest) = trimmed.strip_prefix("import(") {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_static_single_quotes() {
        let result = detect_import_path("import('./base.yml')");
        assert_eq!(result, ImportKind::Static("./base.yml".to_string()));
    }

    #[test]
    fn detects_static_double_quotes() {
        let result = detect_import_path("import(\"./base.yml\")");
        assert_eq!(result, ImportKind::Static("./base.yml".to_string()));
    }

    #[test]
    fn detects_data_driven() {
        let result = detect_import_path("import(arg)");
        assert_eq!(result, ImportKind::DataDriven);
    }

    #[test]
    fn returns_none_for_non_import() {
        let result = detect_import_path("concat(a, b)");
        assert_eq!(result, ImportKind::None);
    }

    #[test]
    fn returns_none_for_empty() {
        let result = detect_import_path("");
        assert_eq!(result, ImportKind::None);
    }

    #[test]
    fn returns_none_for_dynamic_expression() {
        let result = detect_import_path("import(concat(arg, env.ENV))");
        assert_eq!(result, ImportKind::None);
    }

    #[test]
    fn handles_whitespace() {
        let result = detect_import_path("  import('./base.yml')  ");
        assert_eq!(result, ImportKind::Static("./base.yml".to_string()));
    }

    #[test]
    fn extracts_data_path_from_string_field() {
        let data = serde_json::json!({ "import": "./base.yml" });
        let result = extract_data_path(&data, "import");
        assert_eq!(result, Some("./base.yml".to_string()));
    }

    #[test]
    fn returns_none_for_non_string_field() {
        let data = serde_json::json!({ "import": 42 });
        let result = extract_data_path(&data, "import");
        assert_eq!(result, None);
    }

    #[test]
    fn returns_none_for_missing_field() {
        let data = serde_json::json!({});
        let result = extract_data_path(&data, "import");
        assert_eq!(result, None);
    }
}
