//! # ac-resolver-file
//!
//! Filesystem-based [`ModuleResolver`] implementation.
//!
//! Reads files from disk, parses them with the appropriate frontend
//! (YAML/JSON/TOML based on file extension), and extracts fields.
//! Supports optional [`ModuleCache`] for memoizing resolved modules.

use std::path::{Path, PathBuf};

use ac_fields::extract_fields;
use ac_runtime::RuntimeValue;
use ac_resolver_core::{ModuleCache, ModuleResolver, NoCache, ResolvedModule};

/// Filesystem-based module resolver.
///
/// Reads files from disk, parses by extension, extracts fields.
/// This is the default resolver for CLI usage.
///
/// By default, no module cache is used. Pass a [`ModuleCache`] to
/// [`FileResolver::with_cache`] to memoize resolved modules by
/// canonical path.
pub struct FileResolver<C: ModuleCache = NoCache> {
    cache: C,
}

impl FileResolver<NoCache> {
    /// Create a resolver with no caching.
    pub fn new() -> Self {
        FileResolver { cache: NoCache }
    }
}

impl Default for FileResolver<NoCache> {
    fn default() -> Self {
        Self::new()
    }
}

impl<C: ModuleCache> FileResolver<C> {
    /// Create a resolver with a specific cache implementation.
    pub fn with_cache(cache: C) -> Self {
        FileResolver { cache }
    }
}

impl<C: ModuleCache> ModuleResolver for FileResolver<C> {
    fn resolve(&self, path: &str, base: Option<&Path>) -> Option<ResolvedModule> {
        let resolved = match base {
            Some(base) => base.join(path),
            None => PathBuf::from(path),
        };

        let canonical = resolved.canonicalize().unwrap_or(resolved.clone());
        let cache_key = canonical.to_string_lossy().to_string();

        if let Some(cached) = self.cache.get(&cache_key) {
            return Some(cached);
        }

        let content = std::fs::read_to_string(&resolved).ok()?;

        let format = resolved
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("yaml");

        let value = parse_document(&content, format)?;
        let extracted = extract_fields(value).ok()?;

        let module = ResolvedModule {
            schemas: extracted.schemas,
            merge: extracted.merge,
            data: extracted.data.inner().clone(),
            base_path: resolved.parent().map(|p| p.to_path_buf()),
        };

        self.cache.insert(&cache_key, module.clone());
        Some(module)
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
