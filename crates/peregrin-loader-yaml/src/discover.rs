//! File discovery — port of `ac-file-router`'s `discover()`.
//!
//! Scans `.peregrin/` and `.handl/` (when present — `.peregrin` is the
//! rebrand name and wins dedup; `.handl` stays for v1 compatibility),
//! then the base directory itself, recursively, classifying files by
//! suffix. The named roots win on duplicate
//! names; dot-directories are skipped everywhere.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use peregrin_loader::LoadError;

/// A discovered file: its stem name and path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// Filename minus its kind suffix (`foo.node.yml` → `foo`).
    pub name: String,
    /// Full path to the file.
    pub path: PathBuf,
}

/// Everything [`discover`] found, grouped by kind (TS `DiscoveredFiles`).
#[derive(Clone, Debug, Default)]
pub struct DiscoveredFiles {
    /// `*.node.yml` / `*.node.yaml`
    pub nodes: Vec<Entry>,
    /// `*.snippet.yml` / `*.snippet.yaml`
    pub snippets: Vec<Entry>,
    /// `*.event.json`
    pub events: Vec<Entry>,
    /// `*.handler.py`
    pub handlers: Vec<Entry>,
    /// `*.config.yml` / `*.config.yaml` — not deduped, sorted by path.
    pub config: Vec<PathBuf>,
    /// `*.secret.yml` / `*.secret.yaml`
    pub secrets: Vec<Entry>,
    /// `.env` files.
    pub env: Vec<PathBuf>,
    /// `*.fn.yml` / `*.fn.yaml`
    pub functions: Vec<Entry>,
}

/// Suffix → kind, in match order (TS `SUFFIX_KINDS`).
const SUFFIX_KINDS: &[(&str, &str)] = &[
    (".node.yml", "node"),
    (".node.yaml", "node"),
    (".snippet.yml", "snippet"),
    (".snippet.yaml", "snippet"),
    (".event.json", "event"),
    (".handler.py", "handler"),
    (".config.yml", "config"),
    (".config.yaml", "config"),
    (".secret.yml", "secret"),
    (".secret.yaml", "secret"),
    (".fn.yml", "function"),
    (".fn.yaml", "function"),
];

fn match_suffix(filename: &str) -> Option<(String, &'static str)> {
    for (suffix, kind) in SUFFIX_KINDS {
        if let Some(name) = filename.strip_suffix(suffix) {
            if !name.is_empty() {
                return Some((name.to_string(), kind));
            }
        }
    }
    None
}

/// `collect` — sorted files first, then sorted subdirectories
/// recursively. Dot-directories are skipped; the named discovery roots
/// (`.peregrin`, `.handl`) are skipped only when `skip_roots` (they are
/// scanned as their own roots instead).
fn collect(
    dir: &Path,
    skip_roots: bool,
    out: &mut Vec<(PathBuf, String)>,
) -> Result<(), LoadError> {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .map_err(|e| LoadError::new(e.to_string()))?
        .filter_map(std::result::Result::ok)
        .collect();
    entries.sort_by_key(std::fs::DirEntry::file_name);
    let mut files = Vec::new();
    let mut dirs = Vec::new();
    for entry in entries {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        let name = entry.file_name().to_string_lossy().into_owned();
        if file_type.is_dir() {
            if (name == ".peregrin" || name == ".handl") && skip_roots {
                continue;
            }
            if name.starts_with('.') {
                continue;
            }
            dirs.push(entry.path());
        } else if file_type.is_file() {
            files.push((entry.path(), name));
        }
    }
    out.extend(files);
    for sub in dirs {
        collect(&sub, false, out)?;
    }
    Ok(())
}

/// `discover(baseDir)` — named roots first (`.peregrin/` preferred,
/// `.handl/` compat — both win dedup races in scan order), then the
/// base directory (with its own named roots skipped to avoid
/// double-count).
pub fn discover(base_dir: &Path) -> Result<DiscoveredFiles, LoadError> {
    let mut roots: Vec<_> = [".peregrin", ".handl"]
        .iter()
        .map(|name| base_dir.join(name))
        .filter(|dir| dir.is_dir())
        .collect();
    roots.push(base_dir.to_path_buf());
    let named = roots[..roots.len() - 1].to_vec();

    let mut seen_nodes = HashSet::new();
    let mut seen_snippets = HashSet::new();
    let mut seen_events = HashSet::new();
    let mut seen_handlers = HashSet::new();
    let mut seen_secrets = HashSet::new();
    let mut seen_functions = HashSet::new();
    let mut found = DiscoveredFiles::default();

    for root in roots {
        let mut collected = Vec::new();
        collect(&root, !named.contains(&root), &mut collected)?;
        for (fpath, fname) in collected {
            if fname == ".env" {
                found.env.push(fpath);
                continue;
            }
            let Some((name, kind)) = match_suffix(&fname) else {
                continue;
            };
            let (seen, target) = match kind {
                "node" => (&mut seen_nodes, &mut found.nodes as &mut Vec<Entry>),
                "snippet" => (&mut seen_snippets, &mut found.snippets),
                "event" => (&mut seen_events, &mut found.events),
                "handler" => (&mut seen_handlers, &mut found.handlers),
                "secret" => (&mut seen_secrets, &mut found.secrets),
                "function" => (&mut seen_functions, &mut found.functions),
                _ => {
                    found.config.push(fpath);
                    continue;
                }
            };
            if seen.contains(&name) {
                continue;
            }
            seen.insert(name.clone());
            target.push(Entry { name, path: fpath });
        }
    }
    found.config.sort();
    Ok(found)
}
