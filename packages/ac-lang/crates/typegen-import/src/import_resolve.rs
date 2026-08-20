use std::path::Path;

use serde_json::Value as JsonValue;

use ac_resolver_core::ModuleResolver;
use ac_typegen_core::strip_keys;
use ac_typegen_key_expr as expr;

use crate::import_detect::{detect_import_path, extract_data_path, ImportKind};

/// Resolve imports from merge fields.
///
/// For each merge field, checks if its `expr` contains an `import()` call.
/// If so, resolves the module via `resolver`, filters merge fields,
/// strips AC keys, and returns the imported field schemas.
///
/// Recursively resolves nested imports from imported modules.
///
/// Returns a list of (path, imported_field_schemas) pairs.
pub fn resolve_imports(
    schemas: &[(String, JsonValue)],
    merge_fields: &[String],
    data: &JsonValue,
    resolver: &dyn ModuleResolver,
    base: Option<&Path>,
    ac_keys: &[&str],
) -> Vec<(String, Vec<(String, JsonValue)>)> {
    let mut results = Vec::new();
    let mut visited = std::collections::HashSet::new();

    for field_name in merge_fields {
        let Some((_, raw)) = schemas.iter().find(|(n, _)| n == field_name) else {
            continue;
        };

        let Some(expr_str) = expr::get_expr(raw) else {
            continue;
        };

        let kind = detect_import_path(&expr_str);
        let path = match &kind {
            ImportKind::Static(p) => Some(p.clone()),
            ImportKind::DataDriven => extract_data_path(data, field_name),
            ImportKind::None => None,
        };

        let Some(path) = path else { continue };

        if visited.contains(&path) {
            continue;
        }
        visited.insert(path.clone());

        let Some(module) = resolver.resolve(&path, base) else {
            continue;
        };

        let imported_fields: Vec<(String, JsonValue)> = module
            .schemas
            .iter()
            .filter(|(name, _)| !module.merge.contains(name))
            .map(|(name, raw)| (name.clone(), strip_keys(raw, ac_keys)))
            .collect();

        let nested_results = resolve_imports(
            &module.schemas,
            &module.merge,
            &module.data,
            resolver,
            module.base_path.as_deref(),
            ac_keys,
        );

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
