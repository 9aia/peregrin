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
    let content = format!("{}\n", serde_json::to_string_pretty(schema).map_err(|e| e.to_string())?);
    std::fs::write(path, content)
        .map_err(|e| format!("failed to write {}: {e}", path.display()))
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::Read;

    #[test]
    fn writes_schema_file() {
        let mut tmp = std::env::temp_dir();
        tmp.push(format!("ac_typegen_json_test_{}.schema.json", std::process::id()));
        let schema = json!({ "type": "object" });

        write_schema_file(&schema, &tmp).unwrap();

        let mut content = String::new();
        std::fs::File::open(&tmp).unwrap().read_to_string(&mut content).unwrap();
        assert!(content.contains("\"type\": \"object\""));
        assert!(content.ends_with('\n'));

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn creates_parent_directories() {
        let mut tmp = std::env::temp_dir();
        tmp.push(format!("ac_typegen_json_test_{}/nested/dir/schema.json", std::process::id()));
        let schema = json!({ "type": "object" });

        write_schema_file(&schema, &tmp).unwrap();
        assert!(tmp.exists());

        let _ = std::fs::remove_dir_all(tmp.parent().unwrap().parent().unwrap());
    }

    #[test]
    fn injects_comment_when_missing() {
        let mut tmp = std::env::temp_dir();
        tmp.push(format!("ac_typegen_json_test_{}.yml", std::process::id()));
        std::fs::write(&tmp, "fields:\n  host:\n    type: string\n").unwrap();

        inject_schema_comment(&tmp, ".ac/generated/schemas/main.schema.json").unwrap();

        let content = std::fs::read_to_string(&tmp).unwrap();
        assert!(content.starts_with("# yaml-language-server: $schema="));
        assert!(content.contains("fields:"));

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn does_not_inject_when_present() {
        let mut tmp = std::env::temp_dir();
        tmp.push(format!("ac_typegen_json_test_present_{}.yml", std::process::id()));
        let original = "# yaml-language-server: $schema=existing.json\nfields:\n  host:\n    type: string\n";
        std::fs::write(&tmp, original).unwrap();

        inject_schema_comment(&tmp, ".ac/generated/schemas/main.schema.json").unwrap();

        let content = std::fs::read_to_string(&tmp).unwrap();
        assert_eq!(content, original);

        let _ = std::fs::remove_file(&tmp);
    }
}
