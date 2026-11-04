//! `include:` overlay composition (ADR-0082 #3). A bundle's payload
//! deep-merges over its bases with the ported loader's semantics —
//! local wins, dicts merge per-key, arrays concat local-first — and
//! node `id` collisions dedupe keeping the local definition (the
//! double-fire fix). Conflicting `.yaml`/`.yml`/`.json` files merge
//! document-wise; every other conflict resolves whole-file, higher
//! priority wins. Earlier `include:` entries outrank later ones.

use std::fs;
use std::path::{Path, PathBuf};

use peregrin_loader_yaml::merge_documents;
use serde_json::Value;

use crate::digest::payload_files;
use crate::{Error, Manifest};

/// The payload root inside a fetched dir — `dir` itself when it holds
/// a manifest, else the single child dir it wraps everything under
/// (foreign tarballs commonly nest under `name-version/`). Descends
/// while each level is exactly one dir; errors when no manifest turns
/// up before the tree widens.
pub fn payload_root(dir: &Path) -> Result<PathBuf, Error> {
    let mut current = dir.to_path_buf();
    loop {
        if Manifest::path_in(&current).is_some() {
            return Ok(current);
        }
        let mut children = Vec::new();
        let read =
            fs::read_dir(&current).map_err(Error::io(format!("read {}", current.display())))?;
        for child in read {
            children.push(child.map_err(Error::io("read dir entry"))?.path());
        }
        if children.len() == 1 {
            let child = children.into_iter().next().unwrap_or_default();
            if child.is_dir() {
                current = child;
                continue;
            }
        }
        return Err(Error::Failed(format!(
            "no bundle manifest under {}",
            dir.display()
        )));
    }
}

/// Compose `local` over `bases` into `dest`. `bases` list the
/// `include:`d payload dirs in manifest order — earlier includes win
/// over later ones, `local` wins over everything.
pub fn overlay(bases: &[&Path], local: &Path, dest: &Path) -> Result<(), Error> {
    fs::create_dir_all(dest).map_err(Error::io(format!("create {}", dest.display())))?;
    // Lowest priority first — each layer merges over what landed.
    for base in bases.iter().rev() {
        apply_layer(base, dest)?;
    }
    apply_layer(local, dest)
}

/// Mergeable document extensions — same relpath on both sides shares
/// the format.
fn doc_format(path: &Path) -> Option<DocFormat> {
    match path.extension().and_then(|e| e.to_str()) {
        Some("yaml" | "yml") => Some(DocFormat::Yaml),
        Some("json") => Some(DocFormat::Json),
        _ => None,
    }
}

#[derive(Clone, Copy)]
enum DocFormat {
    Yaml,
    Json,
}

fn parse_doc(path: &Path, format: DocFormat) -> Option<Value> {
    let text = fs::read_to_string(path).ok()?;
    match format {
        DocFormat::Yaml => serde_yaml_ng::from_str(&text).ok(),
        DocFormat::Json => serde_json::from_str(&text).ok(),
    }
}

fn write_doc(path: &Path, format: DocFormat, value: &Value) -> Result<(), Error> {
    let text = match format {
        DocFormat::Yaml => serde_yaml_ng::to_string(value)
            .map_err(|e| Error::Failed(format!("serialize {}: {e}", path.display())))?,
        DocFormat::Json => serde_json::to_string_pretty(value)
            .map_err(|e| Error::Failed(format!("serialize {}: {e}", path.display())))?,
    };
    fs::write(path, text).map_err(Error::io(format!("write {}", path.display())))
}

/// Lay one priority layer over `dest` — existing files merge when both
/// sides parse as docs, else the layer's bytes win.
fn apply_layer(layer: &Path, dest: &Path) -> Result<(), Error> {
    for rel in payload_files(layer)? {
        let src = layer.join(&rel);
        let target = dest.join(&rel);
        if target.is_file()
            && let Some(format) = doc_format(&rel)
            && let (Some(incoming), Some(existing)) =
                (parse_doc(&src, format), parse_doc(&target, format))
        {
            // `incoming` is the higher-priority side → local.
            write_doc(&target, format, &merge_documents(&incoming, &existing))?;
            continue;
        }
        let parent = target.parent().unwrap_or(dest);
        fs::create_dir_all(parent).map_err(Error::io(format!("create {}", parent.display())))?;
        fs::copy(&src, &target).map_err(Error::io(format!(
            "copy {} → {}",
            src.display(),
            target.display()
        )))?;
    }
    Ok(())
}
