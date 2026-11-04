//! `peregrin-bundle.{yaml,yml,toml}` — the bundle manifest. YAML is
//! preferred when several exist (the `peregrin-config-file` search
//! convention), TOML stays the permanent fallback.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::Error;

/// Manifest file names tried, in order — YAML first, TOML last.
pub const MANIFEST_NAMES: [&str; 3] = [
    "peregrin-bundle.yaml",
    "peregrin-bundle.yml",
    "peregrin-bundle.toml",
];

/// The bundle manifest.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Manifest {
    /// Bundle name — aliases the installed entry (`name@version`).
    /// Charset `[A-Za-z0-9][A-Za-z0-9._-]*`; `@` and `/` are reserved.
    pub name: String,
    /// Author-declared version — the store label when present.
    pub version: Option<String>,
    /// Free-text description.
    pub description: Option<String>,
    /// Base bundles to compose under this payload — installed store
    /// names (`name` / `name@version`) or installable source refs.
    /// Local files win; earlier includes win over later ones.
    pub include: Vec<String>,
}

impl Manifest {
    /// The manifest file inside `dir`, trying [`MANIFEST_NAMES`] in
    /// order.
    #[must_use]
    pub fn path_in(dir: &Path) -> Option<PathBuf> {
        MANIFEST_NAMES
            .iter()
            .map(|name| dir.join(name))
            .find(|path| path.is_file())
    }

    /// Load + validate the manifest inside `dir`.
    pub fn load(dir: &Path) -> Result<Self, Error> {
        let Some(path) = Self::path_in(dir) else {
            return Err(Error::Failed(format!(
                "no peregrin-bundle.yaml/.yml/.toml in {}",
                dir.display()
            )));
        };
        Self::load_at(&path)
    }

    /// Load + validate a manifest file; syntax dispatched on the
    /// `.yaml`/`.yml` extension (everything else parses as TOML).
    pub fn load_at(path: &Path) -> Result<Self, Error> {
        let text =
            std::fs::read_to_string(path).map_err(Error::io(format!("read {}", path.display())))?;
        let is_yaml = matches!(
            path.extension().and_then(|e| e.to_str()),
            Some("yaml" | "yml")
        );
        let manifest: Self = if is_yaml {
            serde_yaml_ng::from_str::<Self>(&text)
                .map_err(|e| Error::Failed(format!("parse {}: {e}", path.display())))?
        } else {
            toml::from_str(&text)
                .map_err(|e| Error::Failed(format!("parse {}: {e}", path.display())))?
        };
        manifest.validate()?;
        Ok(manifest)
    }

    /// Semantic checks the schema can't express.
    pub fn validate(&self) -> Result<(), Error> {
        let valid_name = !self.name.is_empty()
            && self
                .name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
            && self
                .name
                .bytes()
                .next()
                .is_some_and(|b| b.is_ascii_alphanumeric());
        if !valid_name {
            return Err(Error::Failed(format!(
                "invalid bundle name '{}' — expected [A-Za-z0-9][A-Za-z0-9._-]*",
                self.name
            )));
        }
        if let Some(version) = &self.version {
            if version.is_empty()
                || version
                    .bytes()
                    .any(|b| !(b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-')))
            {
                return Err(Error::Failed(format!(
                    "invalid bundle version '{version}' — expected [A-Za-z0-9._-]+"
                )));
            }
        }
        for include in &self.include {
            if include.trim().is_empty() {
                return Err(Error::Failed(
                    "manifest include entries must be non-empty".to_string(),
                ));
            }
        }
        Ok(())
    }
}
