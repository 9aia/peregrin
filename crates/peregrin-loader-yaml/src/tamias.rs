//! `tamias:` include refs — `tamias:<spec>//<path>` resolves `<path>`
//! inside a tamias bundle's store entry, so a workflow YAML can compose
//! fragments that `tamias install` placed on the host
//! (`~/.local/share/tamias/store/<name>@<label>/`).
//!
//! `<spec>` resolves **store-first**: the store-label spellings
//! (`name`, `name@<label>`, `@scope/name`, `name@<label>`) match
//! installed entries exactly like before — a bare `name` (or
//! `@scope/name`) with several installs picks the most recently
//! installed entry; pin `name@<label>` for determinism. `@<label>` is
//! the store label (`name@0.2.0`, or the sha short form for
//! unversioned installs), matching what `tamias list`/`tamias info`
//! report; it is *not* the install-time source ref (`git:…#branch`).
//!
//! On a store **miss**, a ref-shaped spec is fetched: the spec is handed
//! verbatim to [`tamias_install`]'s pipeline — the same ref grammar
//! `tamias install` accepts (`git:`, `cf:`, `http(s)://`,
//! `<registry>:<name>@<version>` named-registry refs, fs paths) — which
//! resolves the ref, fetches through the content-addressed fetch cache,
//! runs the manifest/driver/signature gates, and commits the store
//! entry with full provenance (`source_ref`, `resolved`, digest,
//! signature/trust) before the include resolves against it. This is a
//! **network-capable load**: includes may fetch. Bundled-driver
//! packages still need recorded consent (or `TAMIAS_ALLOW_DRIVERS=1`)
//! and declared `bins[]` are *not* deployed (`--no-bins` semantics) —
//! the fetch commits the store entry only.
//!
//! Store-name shapes never fetch: tamias's ref grammar has no
//! bare-name or "latest" resolution (`<registry>:<name>@<version>`
//! requires the label and version), so `name`, `name@<label>`, and
//! `@scope/name` misses are hard errors naming an installable ref or
//! `tamias install`. Any fetch failure is a hard error — never a
//! silent fallback.
//!
//! `<spec>` and `<path-inside-bundle>` split at the first `//` that is
//! not part of a `scheme://` — so `http(s)://`/`file://` refs work,
//! but a `git:` ref's own `//subdir` can't be expressed (use the
//! two-segment form `git:org/repo/sub#rev`, where the repo is the
//! first two path segments and the rest is the in-repo path).
//!
//! The store root follows `tamias-cli`'s env chain verbatim
//! (`TAMIAS_HOME`/`XDG_DATA_HOME`/`HOME`, mirrored by
//! `peregrin-cli`'s `OpsEnv`); the fetch cache follows
//! `TAMIAS_CACHE`/`XDG_CACHE_HOME`/`HOME`. [`TamiasRoots`] injects both
//! for embedded hosts and tests. Named registries (`<label>:`) resolve
//! through tamias's effective config — `TAMIAS_CONFIG`/`./tamias.*`
//! over `~/.config/tamias/tamias.*` — same as `tamias install`; the
//! config-file `store.root`/`cache.root` legs are deliberately not
//! consulted (same posture as the `bundle.*` ops).

use std::path::{Path, PathBuf};

use peregrin_loader::LoadError;
use tamias_store_local::Store;

use crate::expand::normalize;

/// Store + fetch-cache roots `tamias:` includes resolve and fetch
/// through — the pair the install pipeline writes.
#[derive(Clone, Debug)]
pub struct TamiasRoots {
    store_root: PathBuf,
    cache_root: PathBuf,
}

impl TamiasRoots {
    /// Explicit roots — embedded hosts and tests.
    #[must_use]
    pub fn new(store_root: impl Into<PathBuf>, cache_root: impl Into<PathBuf>) -> Self {
        Self {
            store_root: store_root.into(),
            cache_root: cache_root.into(),
        }
    }

    /// An explicit store root with the env-derived fetch cache — the
    /// [`YamlLoader::load_*_with_tamias_store`](crate::YamlLoader)
    /// shape.
    pub(crate) fn store_only(store_root: &Path) -> Self {
        Self::new(store_root, cache_root())
    }

    /// Both roots env-derived — the production shape.
    fn from_env() -> Self {
        Self::new(store_root(), cache_root())
    }
}

/// Whether an include path uses the `tamias:` scheme.
pub(crate) fn is_tamias_ref(path: &str) -> bool {
    path.trim_start().starts_with("tamias:")
}

/// Resolve `tamias:<spec>//<inner>` to a file inside a bundle under
/// `roots`. See [`resolve`] for the env-derived entry point.
pub(crate) fn resolve_with_roots(
    roots: &TamiasRoots,
    include_ref: &str,
) -> Result<PathBuf, LoadError> {
    let rest = include_ref
        .trim()
        .strip_prefix("tamias:")
        .unwrap_or_default();
    let Some((spec, inner)) = split_spec(rest) else {
        return Err(LoadError::new(format!(
            "tamias include '{include_ref}' must be 'tamias:<spec>//<path-inside-bundle>'"
        )));
    };
    let spec = spec.trim();
    if spec.is_empty() || inner.trim().is_empty() {
        return Err(LoadError::new(format!(
            "tamias include '{include_ref}' needs a bundle spec and an in-bundle path"
        )));
    }
    let entries = Store::new(&roots.store_root)
        .entries()
        .map_err(|e| LoadError::caused("failed to read the tamias store", e))?;
    let matches: Vec<_> = entries
        .iter()
        .filter(|entry| entry_names(entry).iter().any(|n| n == spec))
        .collect();
    let bundle_root = match matches.as_slice() {
        // Store miss — a ref-shaped spec fetches through the install
        // pipeline; store-name shapes error store-only.
        [] => fetch_bundle(roots, spec, include_ref)?,
        [entry] => normalize(&entry.dir),
        // Unpinned spec, several installs — the most recently installed
        // entry wins (label breaks `installed_at` ties deterministically).
        _ => normalize(
            &matches
                .iter()
                .copied()
                .max_by(|a, b| {
                    a.record
                        .installed_at
                        .cmp(&b.record.installed_at)
                        .then_with(|| a.label().cmp(&b.label()))
                })
                .unwrap_or(matches[0])
                .dir,
        ),
    };
    let candidate = normalize(&bundle_root.join(inner.trim()));
    if !candidate.starts_with(&bundle_root) {
        return Err(LoadError::new(format!(
            "tamias include '{include_ref}': path '{inner}' escapes the bundle root"
        )));
    }
    Ok(candidate)
}

/// Resolve `tamias:<spec>//<inner>` against the env-derived roots
/// ([`TamiasRoots::from_env`]).
///
/// `<spec>` matches a store entry the way tamias's own `include:`
/// base resolution does (`tamias_core::find_store_entry`): the full
/// label (`name@<label>` / `@scope/name@<label>`), the scoped name
/// (`@scope/name`), or the bare name — except that an unpinned spec
/// matching several installs picks the most recently installed entry
/// instead of erroring on ambiguity. On a miss, a ref-shaped spec
/// (`git:`/`cf:`/`http(s)`/`<registry>:name@ver`/path) is installed
/// through `tamias-install` and the include resolves inside the
/// committed entry — see the module docs. The returned path is
/// normalized and guaranteed to stay under the bundle root.
pub(crate) fn resolve(include_ref: &str) -> Result<PathBuf, LoadError> {
    resolve_with_roots(&TamiasRoots::from_env(), include_ref)
}

/// Split `rest` at the `//` separating `<spec>` from `<inner>` — the
/// first `//` not part of a `scheme://`. A `git:` ref's own
/// `//subdir` therefore can't ride this spelling (the split lands
/// inside the spec); the two-segment subdir form
/// (`git:org/repo/sub#rev`) works.
fn split_spec(rest: &str) -> Option<(&str, &str)> {
    let mut offset = 0;
    while let Some(pos) = rest[offset..].find("//") {
        let at = offset + pos;
        // `…://` is the ref's scheme, not the separator.
        if at > 0 && rest.as_bytes()[at - 1] == b':' {
            offset = at + 2;
            continue;
        }
        return Some((&rest[..at], &rest[at + 2..]));
    }
    None
}

/// Cheap shape check mirroring the ref grammar's `claims` arms — no
/// IO, no config reads. Store-name spellings return `false` so a miss
/// stays store-only (tamias has no bare-name/"latest" ref); anything
/// ref-shaped goes to [`tamias_install`], whose own parse is the
/// authoritative well-formedness gate — a malformed ref errors there
/// before any fetch is attempted.
fn fetchable_spec(spec: &str) -> bool {
    spec.contains("://")
        || spec.contains(':')
        || spec.contains('/')
        || matches!(spec, "." | ".." | "~")
}

/// The store-miss error for a spec that isn't ref-shaped — the
/// store-label contract, unchanged from the store-only days.
fn not_installed(spec: &str, include_ref: &str, store_root: &Path) -> LoadError {
    LoadError::new(format!(
        "tamias include '{include_ref}': '{spec}' is not installed in the tamias store \
         ({}); store labels resolve store-only — to fetch, use an install ref \
         (git:/cf:/http(s):///<registry>:<name>@<version>/path) or run \
         'tamias install <ref>' first",
        store_root.display()
    ))
}

/// Store miss → install `spec` through `tamias-install`'s one pipeline
/// (ref parse → plan → fetch/cache → manifest → signature/driver gates
/// → store commit) and return the committed entry's payload dir. The
/// include then resolves inside it exactly like a pre-installed
/// bundle.
///
/// Consent posture matches `tamias serve`'s install op: bundled
/// drivers need recorded consent or `TAMIAS_ALLOW_DRIVERS=1`
/// (`allow_drivers=false` here — the loader never passes the flag),
/// and declared `bins[]` stay undeployed (`deploy_bins=false`) — a
/// load-time fetch commits the store entry, nothing else.
fn fetch_bundle(roots: &TamiasRoots, spec: &str, include_ref: &str) -> Result<PathBuf, LoadError> {
    // `@`-prefixed specs are scoped store names — never refs (the fs
    // fallback would otherwise read them as paths). Same guard
    // `tamias-install`'s own include resolution applies.
    if spec.starts_with('@') || !fetchable_spec(spec) {
        return Err(not_installed(spec, include_ref, &roots.store_root));
    }
    let env =
        tamias_install::Env::for_config(roots.store_root.clone(), roots.cache_root.clone(), None);
    match tamias_install::install_with_progress_opts(
        &env,
        spec,
        false,
        tamias_install::Surface::Rpc,
        &mut |_| {},
        false,
    ) {
        Ok(entry) => Ok(normalize(&entry.dir)),
        Err(e) => Err(LoadError::new(format!(
            "tamias include '{include_ref}': '{spec}' is not installed and the fetch failed — {e}"
        ))),
    }
}

/// The spellings an `include:` spec can match — mirrors
/// `tamias_core::find_store_entry`: full label, scoped name, bare name.
fn entry_names(entry: &tamias_core::StoreEntry) -> [String; 3] {
    let scoped = match &entry.record.scope {
        Some(scope) => format!("@{scope}/{}", entry.record.name),
        None => entry.record.name.clone(),
    };
    [entry.label(), scoped, entry.record.name.clone()]
}

/// The tamias install-store root — `tamias-cli`'s `store_root` env
/// chain (`TAMIAS_HOME` → `XDG_DATA_HOME` → `HOME`), the same one
/// `peregrin-cli`'s `OpsEnv` mirrors for the `bundle.*` ops.
fn store_root() -> PathBuf {
    if let Some(root) = std::env::var_os("TAMIAS_HOME") {
        return PathBuf::from(root).join("store");
    }
    std::env::var_os("XDG_DATA_HOME")
        .map(|d| PathBuf::from(d).join("tamias/store"))
        .or_else(|| {
            std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share/tamias/store"))
        })
        .unwrap_or_else(|| PathBuf::from(".tamias/store"))
}

/// The tamias fetch-cache root — `tamias-cli`'s `cache_root` chain
/// (`TAMIAS_CACHE` → `XDG_CACHE_HOME` → `HOME`), mirrored by
/// `peregrin-cli`'s `OpsEnv`.
fn cache_root() -> PathBuf {
    if let Some(root) = std::env::var_os("TAMIAS_CACHE") {
        return PathBuf::from(root);
    }
    std::env::var_os("XDG_CACHE_HOME")
        .map(|d| PathBuf::from(d).join("tamias/fetch"))
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache/tamias/fetch")))
        .unwrap_or_else(|| PathBuf::from(".tamias/fetch"))
}
