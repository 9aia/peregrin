use serde_json::Value as JsonValue;

/// Detect import path from a merge field's `expr`.
///
/// Reuses the same patterns as `handl-typegen-import`:
/// - `import('path')` or `import("path")` → static path
/// - `import(arg)` → data-driven (path from the field's data value)
pub fn detect_import(schema: &JsonValue, data: &JsonValue, field_name: &str) -> Option<String> {
    let expr_str = schema.get("expr").and_then(|v| v.as_str())?;

    let trimmed = expr_str.trim();

    if trimmed == "import(arg)" {
        return data
            .get(field_name)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
    }

    if let Some(rest) = trimmed.strip_prefix("import(") {
        let rest = rest.trim_end();
        let rest = rest.strip_suffix(')').unwrap_or(rest);
        let rest = rest.trim();
        if let Some(inner) = rest.strip_prefix('\'').and_then(|r| r.strip_suffix('\'')) {
            return Some(inner.to_string());
        }
        if let Some(inner) = rest.strip_prefix('"').and_then(|r| r.strip_suffix('"')) {
            return Some(inner.to_string());
        }
    }

    None
}
