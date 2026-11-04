//! The local bundle store — `<root>/<name>@<version-or-sha>/` unpacked
//! payloads, each carrying a [`RECORD_FILE`] with the source ref,
//! resolved sha, content digest and install timestamp. `name` aliases
//! the entry so campaigns resolve `bundle:<name>` refs.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::digest::{dir_digest, dir_size, payload_files};
use crate::{Error, Manifest};

/// Per-entry install record inside the store dir.
pub const RECORD_FILE: &str = ".peregrin-store.json";

/// What an install records — the provenance `list`/`info`/`verify`
/// report on.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoreRecord {
    /// Bundle name (from the payload manifest).
    pub name: String,
    /// Author-declared version, when the manifest carries one.
    pub version: Option<String>,
    /// The exact source ref the install was invoked with — `update`
    /// re-resolves it, `pin` retargets it.
    pub source_ref: String,
    /// What the source resolved to — commit sha or `sha256:` digest.
    pub resolved_sha: String,
    /// sha256 of the unpacked payload — `verify` recomputes it.
    pub digest: String,
    /// Unix seconds at install time.
    pub installed_at: u64,
}

/// A store entry — dir plus its parsed record.
#[derive(Clone, Debug)]
pub struct StoreEntry {
    /// `<root>/<name>@<label>` on disk.
    pub dir: PathBuf,
    /// Parsed [`RECORD_FILE`].
    pub record: StoreRecord,
}

impl StoreEntry {
    /// `name@version`, or `name@<short-sha>` when unversioned.
    #[must_use]
    pub fn label(&self) -> String {
        format!("{}@{}", self.record.name, version_label(&self.record))
    }

    /// Total payload bytes.
    pub fn size(&self) -> Result<u64, Error> {
        dir_size(&self.dir)
    }

    /// The payload manifest.
    pub fn manifest(&self) -> Result<Manifest, Error> {
        Manifest::load(&self.dir)
    }
}

/// `version` when declared, else the resolved sha's short form —
/// matches how the entry dir was named at install.
fn version_label(record: &StoreRecord) -> String {
    record.version.clone().unwrap_or_else(|| {
        let sha = record
            .resolved_sha
            .strip_prefix("sha256:")
            .unwrap_or(&record.resolved_sha);
        sha.chars().take(12).collect()
    })
}

/// `verify` output — recorded vs recomputed payload digest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifyReport {
    /// `name@label` of the checked entry.
    pub name: String,
    /// Digest the store record carries.
    pub recorded: String,
    /// Digest recomputed over the store dir.
    pub computed: String,
    /// Whether they match.
    pub ok: bool,
}

/// The store — one per `<peregrin-home>/store` root.
pub struct Store {
    root: PathBuf,
}

impl Store {
    /// A store under `root` (created lazily on first install).
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// The root dir.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Every entry — dirs carrying a parseable [`RECORD_FILE`], sorted
    /// by `name@label`. A missing root reads as empty.
    pub fn entries(&self) -> Result<Vec<StoreEntry>, Error> {
        let mut entries = Vec::new();
        if self.root.is_dir() {
            let read = fs::read_dir(&self.root)
                .map_err(Error::io(format!("read {}", self.root.display())))?;
            for entry in read {
                let dir = entry.map_err(Error::io("read store entry"))?.path();
                let record_path = dir.join(RECORD_FILE);
                if !record_path.is_file() {
                    continue;
                }
                let text = fs::read_to_string(&record_path)
                    .map_err(Error::io(format!("read {}", record_path.display())))?;
                let record: StoreRecord = serde_json::from_str(&text)
                    .map_err(|e| Error::Failed(format!("parse {}: {e}", record_path.display())))?;
                entries.push(StoreEntry { dir, record });
            }
        }
        entries.sort_by_key(StoreEntry::label);
        Ok(entries)
    }

    /// Resolve `name` or `name@label` to one entry — a bare name with
    /// several installed versions is an ambiguity error naming them.
    pub fn find(&self, name_or_label: &str) -> Result<StoreEntry, Error> {
        let entries = self.entries()?;
        if name_or_label.contains('@') {
            return entries
                .into_iter()
                .find(|e| e.label() == name_or_label)
                .ok_or_else(|| {
                    Error::Failed(format!("bundle '{name_or_label}' is not installed"))
                });
        }
        let mut matches: Vec<StoreEntry> = entries
            .into_iter()
            .filter(|e| e.record.name == name_or_label)
            .collect();
        match matches.len() {
            0 => Err(Error::Failed(format!(
                "bundle '{name_or_label}' is not installed"
            ))),
            1 => Ok(matches.swap_remove(0)),
            _ => {
                let labels: Vec<String> = matches.iter().map(StoreEntry::label).collect();
                Err(Error::Failed(format!(
                    "bundle '{name_or_label}' has several installs — pick one: {}",
                    labels.join(", ")
                )))
            }
        }
    }

    /// Unpack `payload` into `<name>@<label>` — replacing an existing
    /// entry (re-install is the upgrade path). The record's `digest`
    /// and `installed_at` are filled here from the materialized dir.
    pub fn install(&self, payload: &Path, mut record: StoreRecord) -> Result<StoreEntry, Error> {
        let dir = self
            .root
            .join(format!("{}@{}", record.name, version_label(&record)));
        if dir.exists() {
            fs::remove_dir_all(&dir).map_err(Error::io(format!("clear {}", dir.display())))?;
        }
        copy_tree(payload, &dir)?;
        record.digest = dir_digest(&dir)?;
        record.installed_at = unix_now();
        let record_path = dir.join(RECORD_FILE);
        let json = serde_json::to_string_pretty(&record)
            .map_err(|e| Error::Failed(format!("serialize record: {e}")))?;
        fs::write(&record_path, json)
            .map_err(Error::io(format!("write {}", record_path.display())))?;
        Ok(StoreEntry { dir, record })
    }

    /// Drop a store entry — the fetch cache is untouched.
    pub fn remove(&self, name_or_label: &str) -> Result<StoreRecord, Error> {
        let entry = self.find(name_or_label)?;
        fs::remove_dir_all(&entry.dir)
            .map_err(Error::io(format!("remove {}", entry.dir.display())))?;
        Ok(entry.record)
    }

    /// Recompute the entry's payload digest against the recorded one.
    pub fn verify(&self, name_or_label: &str) -> Result<VerifyReport, Error> {
        let entry = self.find(name_or_label)?;
        let computed = dir_digest(&entry.dir)?;
        Ok(VerifyReport {
            name: entry.label(),
            recorded: entry.record.digest.clone(),
            computed: computed.clone(),
            ok: computed == entry.record.digest,
        })
    }
}

/// Depth-first file copy — `.git/` dirs and the root [`RECORD_FILE`]
/// are skipped, so payloads stage cleanly from checkouts and store
/// dirs alike.
pub fn copy_tree(src: &Path, dest: &Path) -> Result<(), Error> {
    for rel in payload_files(src)? {
        let from = src.join(&rel);
        let to = dest.join(&rel);
        let parent = to.parent().unwrap_or(dest);
        fs::create_dir_all(parent).map_err(Error::io(format!("create {}", parent.display())))?;
        fs::copy(&from, &to).map_err(Error::io(format!(
            "copy {} → {}",
            from.display(),
            to.display()
        )))?;
    }
    Ok(())
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}
