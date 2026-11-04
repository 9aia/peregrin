//! `peregrin install` & friends — the ADR-0079 distribution verbs.
//! The orchestration is: `parse_ref` → [`Source::resolve`] → hit the
//! content-addressed fetch cache when the key is known pre-fetch
//! (git sha, pinned tarball digest) → [`Source::fetch`] → `include:`
//! overlay composition → `Store::install` under `<name>@<version>`,
//! recording the source ref + resolved sha + payload digest.
//!
//! Verbs print their report lines like `campaign` does and return them
//! (or the entry) so tests can assert without scraping stdout.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

use peregrin_bundle::{
    FetchCache, Manifest, Store, StoreEntry, StoreRecord, dir_digest, overlay, pack_tgz,
    payload_root,
};
use peregrin_source::{Resolved, Source, SourceRef, parse_ref};
use peregrin_source_fs::FsSource;
use peregrin_source_git::GitSource;
use peregrin_source_http::HttpSource;

use crate::CliError;

/// Default fetch-cache eviction age for `cache clean` — 30 days.
const CACHE_MAX_AGE_DAYS: u64 = 30;

/// Store + cache roots — [`DistEnv::default`] derives them from
/// `PEREGRIN_HOME`/`PEREGRIN_CACHE`/XDG; tests inject tempdirs.
#[derive(Clone, Debug)]
pub struct DistEnv {
    /// `~/.local/share/peregrin/store` by default.
    pub store_root: PathBuf,
    /// `~/.cache/peregrin/fetch` by default.
    pub cache_root: PathBuf,
}

impl DistEnv {
    /// Store/cache under one sandbox root (tests).
    #[must_use]
    pub fn under(root: &Path) -> Self {
        Self {
            store_root: root.join("store"),
            cache_root: root.join("fetch"),
        }
    }
}

impl Default for DistEnv {
    fn default() -> Self {
        Self {
            store_root: peregrin_node::peregrin_home().join("store"),
            cache_root: peregrin_cache_root().join("fetch"),
        }
    }
}

/// `PEREGRIN_CACHE` → `XDG_CACHE_HOME/peregrin` → `~/.cache/peregrin`.
fn peregrin_cache_root() -> PathBuf {
    std::env::var_os("PEREGRIN_CACHE")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("XDG_CACHE_HOME").map(|d| PathBuf::from(d).join("peregrin")))
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache/peregrin")))
        .unwrap_or_else(|| PathBuf::from(".cache/peregrin"))
}

fn cli(e: impl std::fmt::Display) -> CliError {
    CliError(e.to_string())
}

/// Dispatch the parsed ref to its impl — the port stays impl-free, so
/// the wiring lives here.
fn resolve(reference: &SourceRef) -> Result<Resolved, CliError> {
    match reference {
        SourceRef::Fs(_) => FsSource::new().resolve(reference),
        SourceRef::Git(_) => GitSource::new().resolve(reference),
        SourceRef::Http(_) => HttpSource::new().resolve(reference),
    }
    .map_err(cli)
}

fn fetch(resolved: &Resolved, dest: &Path) -> Result<peregrin_source::Fetched, CliError> {
    match resolved {
        Resolved::Fs { .. } => FsSource::new().fetch(resolved, dest),
        Resolved::Git { .. } => GitSource::new().fetch(resolved, dest),
        Resolved::Http { .. } => HttpSource::new().fetch(resolved, dest),
    }
    .map_err(cli)
}

/// resolve → cache-hit or fetch+commit → `(payload_dir, resolved_id)`.
fn fetch_or_cached(env: &DistEnv, resolved: &Resolved) -> Result<(PathBuf, String), CliError> {
    let cache = FetchCache::new(&env.cache_root);
    if let Some((key, resolved_id)) = resolved.prefetch_identity()
        && cache.has(&key)
    {
        return Ok((cache.entry(&key), resolved_id));
    }
    let scratch = cache.scratch().map_err(cli)?;
    match fetch(resolved, &scratch) {
        Err(e) => {
            cache.discard(&scratch);
            Err(e)
        }
        Ok(fetched) => {
            let dir = cache.commit(&scratch, &fetched.content_key).map_err(cli)?;
            Ok((dir, fetched.resolved))
        }
    }
}

/// `install <ref>` — fetch → compose → store, returning the entry.
pub fn install(env: &DistEnv, reference: &str) -> Result<StoreEntry, CliError> {
    let entry = install_ref(env, reference, &mut HashSet::new())?;
    println!(
        "installed {} ({})",
        entry.label(),
        entry.record.resolved_sha
    );
    Ok(entry)
}

/// The recursive include-aware install — `installing` guards cycles
/// across ref-valued `include:` entries.
fn install_ref(
    env: &DistEnv,
    reference: &str,
    installing: &mut HashSet<String>,
) -> Result<StoreEntry, CliError> {
    if !installing.insert(reference.to_string()) {
        return Err(CliError(format!("include cycle at '{reference}'")));
    }
    let result = install_ref_inner(env, reference, installing);
    installing.remove(reference);
    result
}

fn install_ref_inner(
    env: &DistEnv,
    reference: &str,
    installing: &mut HashSet<String>,
) -> Result<StoreEntry, CliError> {
    let parsed = parse_ref(reference).map_err(cli)?;
    let resolved = resolve(&parsed)?;
    let (payload_dir, resolved_id) = fetch_or_cached(env, &resolved)?;
    let payload = payload_root(&payload_dir).map_err(cli)?;
    let manifest = Manifest::load(&payload).map_err(cli)?;
    let payload = if manifest.include.is_empty() {
        payload
    } else {
        compose(env, &manifest, &payload, installing)?
    };
    let store = Store::new(&env.store_root);
    store
        .install(
            &payload,
            StoreRecord {
                name: manifest.name.clone(),
                version: manifest.version.clone(),
                source_ref: reference.to_string(),
                resolved_sha: resolved_id,
                digest: String::new(),
                installed_at: 0,
            },
        )
        .map_err(cli)
}

/// Overlay `include:` bases under the local payload — store names
/// resolve to their entry dirs, refs install recursively. The composed
/// payload commits to the fetch cache under its own digest.
fn compose(
    env: &DistEnv,
    manifest: &Manifest,
    payload: &Path,
    installing: &mut HashSet<String>,
) -> Result<PathBuf, CliError> {
    let store = Store::new(&env.store_root);
    let cache = FetchCache::new(&env.cache_root);
    let mut bases = Vec::with_capacity(manifest.include.len());
    for include in &manifest.include {
        let dir = if parse_ref(include).is_ok() {
            install_ref(env, include, installing)?.dir
        } else {
            store
                .find(include)
                .map_err(|e| CliError(format!("include '{include}': {e}")))?
                .dir
        };
        bases.push(dir);
    }
    let scratch = cache.scratch().map_err(cli)?;
    let refs: Vec<&Path> = bases.iter().map(PathBuf::as_path).collect();
    if let Err(e) = overlay(&refs, payload, &scratch) {
        cache.discard(&scratch);
        return Err(cli(e));
    }
    let digest = dir_digest(&scratch).map_err(cli)?;
    let dir = cache
        .commit(&scratch, &format!("sha256-{digest}"))
        .map_err(cli)?;
    payload_root(&dir).map_err(cli)
}

/// `remove <name>` — drop the store entry; the cache is untouched.
pub fn remove(env: &DistEnv, name: &str) -> Result<StoreRecord, CliError> {
    let record = Store::new(&env.store_root).remove(name).map_err(cli)?;
    println!("removed {}@{}", record.name, record.resolved_sha);
    Ok(record)
}

/// `list` — `name@label  sha  ref` per entry; also returns the lines.
pub fn list(env: &DistEnv) -> Result<Vec<String>, CliError> {
    let entries = Store::new(&env.store_root).entries().map_err(cli)?;
    let lines: Vec<String> = entries
        .iter()
        .map(|e| {
            format!(
                "{:<24} {:<14} {}",
                e.label(),
                short_sha(&e.record.resolved_sha),
                e.record.source_ref
            )
        })
        .collect();
    if lines.is_empty() {
        println!("no bundles installed");
    } else {
        for line in &lines {
            println!("{line}");
        }
    }
    Ok(lines)
}

/// `info <name>` — manifest + provenance + size.
pub fn info(env: &DistEnv, name: &str) -> Result<Vec<String>, CliError> {
    let entry = Store::new(&env.store_root).find(name).map_err(cli)?;
    let mut lines = vec![
        format!("name:     {}", entry.record.name),
        format!("label:    {}", entry.label()),
        format!("source:   {}", entry.record.source_ref),
        format!("resolved: {}", entry.record.resolved_sha),
        format!("digest:   {}", entry.record.digest),
        format!("installed: {}", entry.record.installed_at),
        format!("size:     {} bytes", entry.size().map_err(cli)?),
        format!("dir:      {}", entry.dir.display()),
    ];
    match entry.manifest() {
        Ok(manifest) => {
            if let Some(description) = &manifest.description {
                lines.push(format!("descr:    {description}"));
            }
            if !manifest.include.is_empty() {
                lines.push(format!("include:  {}", manifest.include.join(", ")));
            }
        }
        Err(e) => lines.push(format!("manifest: unreadable ({e})")),
    }
    for line in &lines {
        println!("{line}");
    }
    Ok(lines)
}

/// `update <name>` / `update --all` — re-resolve each recorded source
/// ref; a differing digest means the upstream moved and the entry
/// lands reinstalled.
pub fn update(env: &DistEnv, name: Option<&str>, all: bool) -> Result<Vec<String>, CliError> {
    let store = Store::new(&env.store_root);
    let targets = if all {
        store.entries().map_err(cli)?
    } else {
        let name = name.ok_or_else(|| CliError("update needs a name or --all".into()))?;
        vec![store.find(name).map_err(cli)?]
    };
    let mut lines = Vec::new();
    for entry in targets {
        let old = entry.label();
        let old_sha = entry.record.resolved_sha.clone();
        let old_digest = entry.record.digest.clone();
        let installed = install_ref(env, &entry.record.source_ref, &mut HashSet::new())?;
        let line = if installed.record.digest == old_digest {
            format!("{old}: already current ({})", short_sha(&old_sha))
        } else {
            format!(
                "{} → {} ({})",
                old,
                installed.label(),
                short_sha(&installed.record.resolved_sha)
            )
        };
        if installed.dir != entry.dir && entry.dir.exists() {
            // The retarget landed under a new label — drop the stale dir.
            let _ = store.remove(&old);
        }
        println!("{line}");
        lines.push(line);
    }
    Ok(lines)
}

/// `pin <name> <ref>` — retarget the recorded source ref; the refetch
/// happens inside install and lands only when the digest differs.
pub fn pin(env: &DistEnv, name: &str, reference: &str) -> Result<String, CliError> {
    let store = Store::new(&env.store_root);
    let entry = store.find(name).map_err(cli)?;
    let installed = install_ref(env, reference, &mut HashSet::new())?;
    if installed.dir != entry.dir && entry.dir.exists() {
        let _ = store.remove(&entry.label());
    }
    let line = if installed.record.digest == entry.record.digest {
        format!("{}: pinned {reference} (unchanged)", entry.record.name)
    } else {
        format!(
            "{}: pinned {reference} — {} ({})",
            entry.record.name,
            installed.label(),
            short_sha(&installed.record.resolved_sha)
        )
    };
    println!("{line}");
    Ok(line)
}

/// `verify <name>` — recompute the payload digest against the record.
pub fn verify(env: &DistEnv, name: &str) -> Result<peregrin_bundle::VerifyReport, CliError> {
    let report = Store::new(&env.store_root).verify(name).map_err(cli)?;
    if report.ok {
        println!("{}: ok ({})", report.name, short_sha(&report.recorded));
        Ok(report)
    } else {
        Err(CliError(format!(
            "{}: digest mismatch — recorded {}, computed {}",
            report.name,
            short_sha(&report.recorded),
            short_sha(&report.computed)
        )))
    }
}

/// `cache clean [--all] [--days N]` — evict by age, or everything.
pub fn cache_clean(env: &DistEnv, all: bool, days: Option<u64>) -> Result<usize, CliError> {
    let cache = FetchCache::new(&env.cache_root);
    let evicted = if all {
        cache.clean_all().map_err(cli)?
    } else {
        cache
            .clean(Duration::from_secs(
                days.unwrap_or(CACHE_MAX_AGE_DAYS) * 86_400,
            ))
            .map_err(cli)?
    };
    println!("evicted {} cache entries", evicted.len());
    Ok(evicted.len())
}

/// `bundle pack <dir> [--out file]` — dir + manifest → a `.tgz`.
pub fn bundle_pack(dir: &Path, out: Option<&Path>) -> Result<PathBuf, CliError> {
    let manifest = Manifest::load(dir).map_err(cli)?;
    let default_name = manifest.version.as_ref().map_or_else(
        || format!("{}.tgz", manifest.name),
        |v| format!("{}-{v}.tgz", manifest.name),
    );
    let out = out.map_or_else(|| PathBuf::from(&default_name), Path::to_path_buf);
    let packed = pack_tgz(dir, &out).map_err(cli)?;
    println!("packed {} → {}", manifest.name, packed.display());
    Ok(packed)
}

/// Short form for display — `sha256:` prefix stripped, 12 chars.
fn short_sha(sha: &str) -> &str {
    let sha = sha.strip_prefix("sha256:").unwrap_or(sha);
    sha.get(..12).unwrap_or(sha)
}
