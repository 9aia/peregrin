//! peregrin-loader-yaml — `campaign.yaml` → `peregrin_core::Campaign`,
//! with `include`/`snippet` expansion and `.handl/` file discovery
//! (port of `ac-loader-yaml` + `ac-file-router`).
//!
//! Pipeline (matching the TS `loadRaw`, in order):
//! 1. parse YAML → must be a mapping
//! 2. auto-discover `.handl/` + base-dir `*.node/snippet/config/fn/event`
//!    artifacts and fold them into the raw mapping
//! 3. `resolveIncludeDict` — dict-level `include:` keys
//! 4. `resolveSnippetDefinitions` — `snippets:` inheritance fixpoint
//! 5. `resolveSnippetRefs` — dict-level `snippet:` keys
//! 6. `resolveSnippetStrings` — `${snippet(name)}` expressions
//! 7. `resolveIncludeStrings` — `${include(path)}` expressions
//! 8. `rejectWithKey` — `with:` fields are a hard error
//! 9. `Campaign::from_dict` — typed model + load-time normalization
//!
//! Template `${...}` expressions (`${state.*}`, `${config.*}`, …) are
//! *not* touched here — they resolve at eval time in peregrin-template.

mod discover;
mod expand;
mod parse;
mod tamias;

use std::collections::HashSet;
use std::path::Path;

use peregrin_loader::{LoadError, Loader};
use serde_json::{Map, Value};

pub use discover::{DiscoveredFiles, Entry, discover};
pub use expand::{dedupe_node_ids, deep_merge, merge_documents};
use expand::{
    resolve_include_dict, resolve_include_strings, resolve_snippet_definitions,
    resolve_snippet_refs, resolve_snippet_strings,
};
use parse::{parse_json, parse_yaml, read_text_file};
pub use tamias::TamiasRoots;

/// `autoDiscover` — fold discovered `*.node`/`*.snippet`/`*.config`/
/// `*.event.json`/`*.fn` artifacts into the raw mapping.
fn auto_discover(raw: &mut Map<String, Value>, base_dir: &Path) -> Result<(), LoadError> {
    let discovered = discover(base_dir)?;

    if !discovered.snippets.is_empty() {
        let mut snippets = raw
            .get("snippets")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        for entry in &discovered.snippets {
            let text = read_text_file(&entry.path)?;
            let parsed = parse_yaml(&text)?;
            snippets.insert(entry.name.clone(), parsed);
        }
        raw.insert("snippets".to_string(), Value::Object(snippets));
    }

    if !discovered.nodes.is_empty() {
        let mut node_list = Vec::new();
        for entry in &discovered.nodes {
            let text = read_text_file(&entry.path)?;
            let parsed = parse_yaml(&text)?;
            match parsed {
                Value::Array(items) => node_list.extend(items),
                Value::Object(_) => node_list.push(parsed),
                _ => {}
            }
        }
        let existing = raw
            .get("nodes")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        node_list.extend(existing);
        raw.insert("nodes".to_string(), Value::Array(node_list));
    }

    if !discovered.config.is_empty() {
        let mut config = raw
            .get("config")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        for file_path in &discovered.config {
            let text = read_text_file(file_path)?;
            let parsed = parse_yaml(&text)?;
            if parsed.is_object() {
                // Inline config and earlier (sorted) files win their keys.
                config = match deep_merge(&Value::Object(config), &parsed) {
                    Value::Object(map) => map,
                    _ => Map::new(),
                };
            }
        }
        raw.insert("config".to_string(), Value::Object(config));
    }

    if !discovered.events.is_empty() {
        let mut schemas = raw
            .get("payload_schemas")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        for entry in &discovered.events {
            let text = read_text_file(&entry.path)?;
            let parsed = parse_json(&text)?;
            if parsed.is_object() {
                schemas.insert(entry.name.clone(), parsed);
            }
        }
        raw.insert("payload_schemas".to_string(), Value::Object(schemas));
    }

    if !discovered.functions.is_empty() {
        let mut function_list = Vec::new();
        for entry in &discovered.functions {
            let text = read_text_file(&entry.path)?;
            let parsed = parse_yaml(&text)?;
            if let Value::Object(mut map) = parsed {
                let has_name = map
                    .get("name")
                    .and_then(Value::as_str)
                    .is_some_and(|n| !n.is_empty());
                if !has_name {
                    map.insert("name".to_string(), Value::String(entry.name.clone()));
                }
                function_list.push(Value::Object(map));
            }
        }
        let mut existing = Vec::new();
        match raw.get("functions") {
            Some(Value::Array(items)) => {
                existing.extend(items.iter().filter(|v| v.is_object()).cloned());
            }
            Some(Value::Object(map)) => {
                for (name, body) in map {
                    let mut record = body.as_object().cloned().unwrap_or_default();
                    record.insert("name".to_string(), Value::String(name.clone()));
                    existing.push(Value::Object(record));
                }
            }
            _ => {}
        }
        function_list.extend(existing);
        raw.insert("functions".to_string(), Value::Array(function_list));
    }

    Ok(())
}

/// `rejectWithKey` — `with:` was removed; point authors at `params:`.
fn reject_with_key(raw: &Map<String, Value>) -> Result<(), LoadError> {
    let Some(nodes) = raw.get("nodes").and_then(Value::as_array) else {
        return Ok(());
    };
    for node in nodes {
        if let Some(obj) = node.as_object() {
            if obj.contains_key("with") {
                let id = obj
                    .get("id")
                    .and_then(Value::as_str)
                    .map_or_else(String::new, |s| format!(" '{s}'"));
                return Err(LoadError::new(format!(
                    "Node{id} uses removed 'with' field; use 'params' instead"
                )));
            }
        }
    }
    Ok(())
}

/// YAML impl of the [`Loader`] port.
#[derive(Clone, Copy, Debug, Default)]
pub struct YamlLoader;

impl YamlLoader {
    /// TS `loadRaw` — YAML text → the expanded raw mapping (before the
    /// typed-model pass). `base_dir` anchors relative includes and
    /// `.handl/` discovery.
    pub fn load_raw(&self, text: &str, base_dir: &Path) -> Result<Map<String, Value>, LoadError> {
        Self::load_raw_inner(text, base_dir, None)
    }

    /// [`load_raw`](Self::load_raw) with `tamias:` includes resolved
    /// against an explicit store root instead of the
    /// `TAMIAS_HOME`/XDG/HOME chain — for embedded hosts and tests.
    /// The fetch cache stays env-derived (`TAMIAS_CACHE`/XDG/`HOME`);
    /// use [`load_raw_with_tamias_roots`](Self::load_raw_with_tamias_roots)
    /// to pin both.
    pub fn load_raw_with_tamias_store(
        &self,
        text: &str,
        base_dir: &Path,
        store_root: &Path,
    ) -> Result<Map<String, Value>, LoadError> {
        self.load_raw_with_tamias_roots(text, base_dir, &TamiasRoots::store_only(store_root))
    }

    /// [`load_raw`](Self::load_raw) with `tamias:` includes resolved
    /// against explicit store + fetch-cache roots — the pair a
    /// store-miss fetch installs through. For embedded hosts and
    /// tests.
    pub fn load_raw_with_tamias_roots(
        &self,
        text: &str,
        base_dir: &Path,
        roots: &TamiasRoots,
    ) -> Result<Map<String, Value>, LoadError> {
        Self::load_raw_inner(text, base_dir, Some(roots))
    }

    fn load_raw_inner(
        text: &str,
        base_dir: &Path,
        tamias: Option<&TamiasRoots>,
    ) -> Result<Map<String, Value>, LoadError> {
        let parsed = parse_yaml(text)?;
        let Value::Object(mut raw) = parsed else {
            return Err(LoadError::new("workflow must be a mapping"));
        };
        auto_discover(&mut raw, base_dir)?;
        let resolved =
            resolve_include_dict(&Value::Object(raw), base_dir, tamias, &mut HashSet::new())?;
        let Value::Object(raw) = resolved else {
            return Err(LoadError::new("workflow must be a mapping"));
        };
        let snippets = match raw.get("snippets") {
            None => Map::new(),
            Some(Value::Object(map)) => resolve_snippet_definitions(map)?,
            Some(_) => return Err(LoadError::new("'snippets' must be a mapping")),
        };
        let raw = resolve_snippet_refs(&Value::Object(raw), &snippets, &mut HashSet::new())?;
        let raw = resolve_snippet_strings(&raw, &snippets, &mut HashSet::new())?;
        let raw = resolve_include_strings(&raw, base_dir, tamias, &mut HashSet::new())?;
        let Value::Object(mut raw) = raw else {
            return Err(LoadError::new("workflow must be a mapping"));
        };
        // Include/overlay merges concat `nodes:` local-first — a
        // redefined id would double-fire, so dedupe keeps the first
        // (local) definition.
        dedupe_node_ids(&mut raw);
        reject_with_key(&raw)?;
        validate_params_schema(&raw, base_dir)?;
        Ok(raw)
    }
}

/// `params.schema.json` (ADR-0081 / I-022) — when the loaded file's
/// directory carries one, the merged `config:` mapping must satisfy
/// it; a typo'd key fails the load instead of silently resolving
/// `${config.*}` to an empty string at eval time. Absent schema → no
/// validation (plain campaigns carry arbitrary `config:` keys).
fn validate_params_schema(raw: &Map<String, Value>, base_dir: &Path) -> Result<(), LoadError> {
    let schema_path = base_dir.join(peregrin_loader::params_schema::PARAMS_SCHEMA_FILE);
    let text = match std::fs::read_to_string(&schema_path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => {
            return Err(LoadError::caused(
                format!("Failed to read {}", schema_path.display()),
                e,
            ));
        }
    };
    let schema: Value = serde_json::from_str(&text)
        .map_err(|e| LoadError::new(format!("{}: invalid JSON: {e}", schema_path.display())))?;
    let Some(config) = raw.get("config").and_then(Value::as_object) else {
        return Ok(());
    };
    let violations = peregrin_loader::params_schema::params_violations(&schema, config)
        .map_err(LoadError::new)?;
    if violations.is_empty() {
        return Ok(());
    }
    Err(LoadError::new(format!(
        "{} rejected config: {}",
        schema_path.display(),
        violations.join("; ")
    )))
}

impl Loader for YamlLoader {
    /// TS `loadWorkflow` — read the file, expand relative to its
    /// directory, normalize into a [`peregrin_core::Campaign`]. The
    /// campaign's `source_dir` is stamped with the file's directory so
    /// run-time relative paths (`subworkflow` `params.path`, ADR-0022)
    /// anchor the same way `include:` does.
    fn load(&self, source: &Path) -> Result<peregrin_core::Campaign, LoadError> {
        let text = read_text_file(source)?;
        let absolute = std::path::absolute(source).map_err(|e| {
            LoadError::caused(format!("Failed to read file: {}", source.display()), e)
        })?;
        let base_dir = absolute.parent().unwrap_or_else(|| Path::new("."));
        let raw = self.load_raw(&text, base_dir)?;
        let mut campaign = peregrin_core::Campaign::from_dict(&Value::Object(raw))
            .map_err(|e| LoadError::new(e.to_string()))?;
        campaign.source_dir = Some(base_dir.to_path_buf());
        Ok(campaign)
    }

    /// TS `loadWorkflowFromText`. `source_dir` stamps `base_dir` —
    /// the same anchor includes resolve against.
    fn load_str(&self, text: &str, base_dir: &Path) -> Result<peregrin_core::Campaign, LoadError> {
        let raw = self.load_raw(text, base_dir)?;
        let mut campaign = peregrin_core::Campaign::from_dict(&Value::Object(raw))
            .map_err(|e| LoadError::new(e.to_string()))?;
        campaign.source_dir = Some(base_dir.to_path_buf());
        Ok(campaign)
    }
}

impl YamlLoader {
    /// [`load_str`](Loader::load_str) with `tamias:` includes resolved
    /// against an explicit store root — see
    /// [`load_raw_with_tamias_store`](Self::load_raw_with_tamias_store).
    pub fn load_str_with_tamias_store(
        &self,
        text: &str,
        base_dir: &Path,
        store_root: &Path,
    ) -> Result<peregrin_core::Campaign, LoadError> {
        let raw = self.load_raw_with_tamias_store(text, base_dir, store_root)?;
        let mut campaign = peregrin_core::Campaign::from_dict(&Value::Object(raw))
            .map_err(|e| LoadError::new(e.to_string()))?;
        campaign.source_dir = Some(base_dir.to_path_buf());
        Ok(campaign)
    }

    /// [`load_str`](Loader::load_str) with `tamias:` includes resolved
    /// against explicit store + fetch-cache roots — see
    /// [`load_raw_with_tamias_roots`](Self::load_raw_with_tamias_roots).
    pub fn load_str_with_tamias_roots(
        &self,
        text: &str,
        base_dir: &Path,
        roots: &TamiasRoots,
    ) -> Result<peregrin_core::Campaign, LoadError> {
        let raw = self.load_raw_with_tamias_roots(text, base_dir, roots)?;
        let mut campaign = peregrin_core::Campaign::from_dict(&Value::Object(raw))
            .map_err(|e| LoadError::new(e.to_string()))?;
        campaign.source_dir = Some(base_dir.to_path_buf());
        Ok(campaign)
    }
}
