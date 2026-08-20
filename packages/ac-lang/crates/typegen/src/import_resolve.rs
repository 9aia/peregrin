use std::path::{Path, PathBuf};

use serde_json::Value as JsonValue;

use ac_fields::extract_fields;
use ac_runtime::RuntimeValue;

use crate::strip::strip_ac_keys;
use crate::import_detect::{detect_import_path, extract_data_path, ImportKind};

/// Resolve imports from merge fields.
///
/// For each merge field, checks if its `expr` contains an `import()` call.
/// If so, loads the imported document, extracts its `fields:`, strips AC
/// keys, and returns the imported field schemas.
///
/// Returns a list of (relative_path, imported_field_schemas) pairs.
/// The caller is responsible for merging these into the parent schema.
pub fn resolve_imports(
    schemas: &[(String, JsonValue)],
    merge_fields: &[String],
    data: &JsonValue,
    base_path: Option<&Path>,
) -> Vec<(String, Vec<(String, JsonValue)>)> {
    let mut results = Vec::new();
    let mut visited = std::collections::HashSet::new();

    for field_name in merge_fields {
        // Find the field's schema.
        let Some((_, raw)) = schemas.iter().find(|(n, _)| n == field_name) else {
            continue;
        };

        // Get the expr string.
        let Some(expr) = raw.get("expr").and_then(|v| v.as_str()) else {
            continue;
        };

        let kind = detect_import_path(expr);
        let path = match &kind {
            ImportKind::Static(p) => Some(p.clone()),
            ImportKind::DataDriven => extract_data_path(data, field_name),
            ImportKind::None => None,
        };

        let Some(path) = path else { continue };

        // Resolve relative to base_path.
        let resolved = match base_path {
            Some(base) => base.join(&path),
            None => PathBuf::from(&path),
        };

        let cache_key = resolved.to_string_lossy().to_string();
        if visited.contains(&cache_key) {
            continue;
        }
        visited.insert(cache_key.clone());

        // Load and parse the imported document.
        let Ok(content) = std::fs::read_to_string(&resolved) else {
            continue;
        };

        let format = resolved
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("yaml");

        let value = match parse_document(&content, format) {
            Some(v) => v,
            None => continue,
        };

        let Ok(extracted) = extract_fields(value) else {
            continue;
        };

        // Strip AC keys from imported schemas.
        let imported_fields: Vec<(String, JsonValue)> = extracted
            .schemas
            .iter()
            .filter(|(name, _)| !extracted.merge.contains(name))
            .map(|(name, raw)| (name.clone(), strip_ac_keys(raw)))
            .collect();

        // Recursively resolve nested imports in the imported document.
        let nested_base = resolved.parent();
        let nested_data = extracted.data.inner().clone();
        let nested_results = resolve_imports(
            &extracted.schemas,
            &extracted.merge,
            &nested_data,
            nested_base,
        );

        // Merge nested imports into the imported fields.
        let mut all_fields = imported_fields;
        for (_, nested_fields) in &nested_results {
            for (name, schema) in nested_fields {
                if !all_fields.iter().any(|(n, _)| n == name) {
                    all_fields.push((name.clone(), schema.clone()));
                }
            }
        }

        results.push((path, all_fields));
    }

    results
}

/// Parse a document string into a `RuntimeValue` based on format.
fn parse_document(content: &str, format: &str) -> Option<RuntimeValue> {
    match format {
        "yaml" | "yml" => ac_frontend_yaml::parse(content).ok(),
        "json" => ac_frontend_json::parse(content).ok(),
        "toml" => ac_frontend_toml::parse(content).ok(),
        _ => ac_frontend_yaml::parse(content).ok(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_temp_file(content: &str, suffix: &str) -> std::path::PathBuf {
        let mut tmp = std::env::temp_dir();
        tmp.push(format!("ac_typegen_test_{}_{suffix}", std::process::id()));
        std::fs::write(&tmp, content).unwrap();
        tmp
    }

    #[test]
    fn resolves_static_import() {
        let base_content = "fields:\n  host:\n    type: string\n  port:\n    type: number\n";
        let base_path = make_temp_file(base_content, "base.yml");

        let base_dir = base_path.parent().unwrap();
        let base_name = base_path.file_name().unwrap().to_str().unwrap();

        let schemas = vec![
            (
                "import".to_string(),
                json!({
                    "type": "object",
                    "expr": format!("import('{base_name}')"),
                    "merge": true
                }),
            ),
            ("name".to_string(), json!({ "type": "string" })),
        ];
        let merge = vec!["import".to_string()];
        let data = json!({});

        let results = resolve_imports(&schemas, &merge, &data, Some(base_dir));

        assert_eq!(results.len(), 1);
        let (path, fields) = &results[0];
        assert!(path.ends_with("base.yml"));
        assert_eq!(fields.len(), 2);
        assert!(fields.iter().any(|(n, _)| n == "host"));
        assert!(fields.iter().any(|(n, _)| n == "port"));

        let _ = std::fs::remove_file(&base_path);
    }

    #[test]
    fn resolves_data_driven_import() {
        let base_content = "fields:\n  host:\n    type: string\n  port:\n    type: number\n";
        let base_path = make_temp_file(base_content, "base2.yml");

        let base_dir = base_path.parent().unwrap();
        let base_name = base_path.file_name().unwrap().to_str().unwrap();

        let schemas = vec![
            (
                "import".to_string(),
                json!({
                    "type": "object",
                    "arg": { "type": "string" },
                    "expr": "import(arg)",
                    "merge": true
                }),
            ),
            ("name".to_string(), json!({ "type": "string" })),
        ];
        let merge = vec!["import".to_string()];
        let data = json!({ "import": base_name });

        let results = resolve_imports(&schemas, &merge, &data, Some(base_dir));

        assert_eq!(results.len(), 1);
        let (_, fields) = &results[0];
        assert_eq!(fields.len(), 2);
        assert!(fields.iter().any(|(n, _)| n == "host"));

        let _ = std::fs::remove_file(&base_path);
    }

    #[test]
    fn skips_non_import_merge_field() {
        let schemas = vec![
            (
                "config".to_string(),
                json!({
                    "type": "object",
                    "arg": { "type": "object" },
                    "expr": "arg",
                    "merge": true
                }),
            ),
        ];
        let merge = vec!["config".to_string()];
        let data = json!({});

        let results = resolve_imports(&schemas, &merge, &data, None);
        assert!(results.is_empty());
    }

    #[test]
    fn handles_missing_file_gracefully() {
        let schemas = vec![
            (
                "import".to_string(),
                json!({
                    "type": "object",
                    "expr": "import('./nonexistent.yml')",
                    "merge": true
                }),
            ),
        ];
        let merge = vec!["import".to_string()];
        let data = json!({});

        let results = resolve_imports(&schemas, &merge, &data, None);
        assert!(results.is_empty());
    }

    #[test]
    fn strips_ac_keys_from_imported_schemas() {
        let base_content = "fields:\n  host:\n    type: string\n    expr: \"compute()\"\n    arg: true\n";
        let base_path = make_temp_file(base_content, "base3.yml");

        let base_dir = base_path.parent().unwrap();
        let base_name = base_path.file_name().unwrap().to_str().unwrap();

        let schemas = vec![
            (
                "import".to_string(),
                json!({
                    "type": "object",
                    "expr": format!("import('{base_name}')"),
                    "merge": true
                }),
            ),
        ];
        let merge = vec!["import".to_string()];
        let data = json!({});

        let results = resolve_imports(&schemas, &merge, &data, Some(base_dir));

        assert_eq!(results.len(), 1);
        let (_, fields) = &results[0];
        let (_, host_schema) = fields.iter().find(|(n, _)| n == "host").unwrap();
        assert!(!host_schema.as_object().unwrap().contains_key("expr"));
        assert!(!host_schema.as_object().unwrap().contains_key("arg"));

        let _ = std::fs::remove_file(&base_path);
    }
}
