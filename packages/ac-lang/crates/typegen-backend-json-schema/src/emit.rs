use std::path::Path;

use serde_json::Value as JsonValue;

/// Write a JSON Schema to a file.
///
/// Creates parent directories if needed. The output is pretty-printed
/// JSON with a trailing newline.
pub fn write_schema_file(schema: &JsonValue, path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create {}: {e}", parent.display()))?;
    }
    let content = format!(
        "{}\n",
        serde_json::to_string_pretty(schema).map_err(|e| e.to_string())?
    );
    std::fs::write(path, content).map_err(|e| format!("failed to write {}: {e}", path.display()))
}

/// Inject a `# yaml-language-server: $schema=...` comment at the top
/// of a YAML file if not already present.
pub fn inject_schema_comment(yaml_path: &Path, schema_rel_path: &str) -> Result<(), String> {
    let content = std::fs::read_to_string(yaml_path)
        .map_err(|e| format!("failed to read {}: {e}", yaml_path.display()))?;

    let has_schema = content
        .lines()
        .any(|line| line.trim().starts_with("# yaml-language-server: $schema="));

    if has_schema {
        return Ok(());
    }

    let comment = format!("# yaml-language-server: $schema={schema_rel_path}\n");
    std::fs::write(yaml_path, format!("{comment}{content}"))
        .map_err(|e| format!("failed to write {}: {e}", yaml_path.display()))
}