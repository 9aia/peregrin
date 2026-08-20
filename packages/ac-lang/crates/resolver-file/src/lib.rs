//! # ac-resolver-file
//!
//! Filesystem-based [`ModuleResolver`] implementation.
//!
//! Reads files from disk, parses them with the appropriate frontend
//! (YAML/JSON/TOML based on file extension), and extracts fields.

use std::path::{Path, PathBuf};

use ac_fields::extract_fields;
use ac_runtime::RuntimeValue;
use ac_resolver_core::{ModuleResolver, ResolvedModule};

/// Filesystem-based module resolver.
///
/// Reads files from disk, parses by extension, extracts fields.
/// This is the default resolver for CLI usage.
pub struct FileResolver;

impl ModuleResolver for FileResolver {
    fn resolve(&self, path: &str, base: Option<&Path>) -> Option<ResolvedModule> {
        let resolved = match base {
            Some(base) => base.join(path),
            None => PathBuf::from(path),
        };

        let content = std::fs::read_to_string(&resolved).ok()?;

        let format = resolved
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("yaml");

        let value = parse_document(&content, format)?;
        let extracted = extract_fields(value).ok()?;

        Some(ResolvedModule {
            schemas: extracted.schemas,
            merge: extracted.merge,
            data: extracted.data.inner().clone(),
            base_path: resolved.parent().map(|p| p.to_path_buf()),
        })
    }
}

fn parse_document(content: &str, format: &str) -> Option<RuntimeValue> {
    match format {
        "yaml" | "yml" => ac_frontend_yaml::parse(content).ok(),
        "json" => ac_frontend_json::parse(content).ok(),
        "toml" => ac_frontend_toml::parse(content).ok(),
        _ => ac_frontend_yaml::parse(content).ok(),
    }
}
