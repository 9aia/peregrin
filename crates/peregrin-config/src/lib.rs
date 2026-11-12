//! peregrin-config — the config port's shared half: the
//! [`ConfigSource`] trait, the `~` helpers, and
//! [`campaign_from_raw`], the funnel every file adapter takes.
//! File discovery lives in scarab-config (P-017 — the
//! `peregrin-config-file`/`-yaml` crates are gone); TOML parsing
//! rides `scarab-config-toml` inside `peregrin-config-toml`, and
//! YAML goes through `peregrin-node`'s campaign loader (its
//! `include:`/`snippet:` expansion needs the file's directory, so
//! it can't be a text-only `ConfigFormat`).
//!
//! The loaded config is the workflow-shaped node document of
//! ADR-0080 — a [`peregrin_core::Campaign`]. Every file adapter
//! funnels through [`campaign_from_raw`]: require a mapping document,
//! normalize via `Campaign::from_dict`, then expand `~` in the
//! local-path fields so TOML and YAML sources get identical
//! semantics.

use std::path::{Path, PathBuf};

/// A config source — returns the loaded [`peregrin_core::Campaign`].
pub trait ConfigSource: Send + Sync {
    /// Load + validate the config; Err carries a readable message.
    fn load(&self) -> Result<peregrin_core::Campaign, String>;
}

/// `$HOME`, else `$USERPROFILE`, else `/` — the workspace `home_dir`
/// convention (peregrin-git's `expand_path`).
fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map_or_else(|| PathBuf::from("/"), PathBuf::from)
}

/// `~` / `~/…` expansion on a string path — same rules as
/// peregrin-git's `expand_path` (no shellexpand dep). `~user`, plain
/// and absolute paths pass through untouched.
#[must_use]
pub fn expand_tilde(path: &str) -> String {
    if path == "~" {
        return home_dir().to_string_lossy().into_owned();
    }
    if let Some(rest) = path.strip_prefix("~/") {
        return home_dir().join(rest).to_string_lossy().into_owned();
    }
    path.into()
}

/// Expand `~` in the campaign's local-path fields — the `include:` and
/// `import:` file lists. Node `params` and the `config:` map stay
/// literal: driver-owned paths expand at their use sites (driver-git
/// calls `expand_path` itself), and remote-path params keep a
/// meaningful `~` for the remote shell.
pub fn expand_campaign_paths(campaign: &mut peregrin_core::Campaign) {
    for include in &mut campaign.imports {
        *include = expand_tilde(include);
    }
    for import in &mut campaign.python_imports {
        import.file = expand_tilde(&import.file);
    }
}

/// The shared tail of every file adapter: require a mapping document,
/// run `Campaign::from_dict` (the workflow-grammar normalization —
/// `functions:` mappings, `include`/`includes` alias, `'nodes' must be
/// an array` checks), then [`expand_campaign_paths`]. Errors are
/// prefixed `parse <path>:` to match the adapters' own errors.
pub fn campaign_from_raw(
    raw: &serde_json::Value,
    path: &Path,
) -> Result<peregrin_core::Campaign, String> {
    if !raw.is_object() {
        return Err(format!(
            "parse {}: config must be a mapping",
            path.display()
        ));
    }
    let mut campaign = peregrin_core::Campaign::from_dict(raw)
        .map_err(|e| format!("parse {}: {e}", path.display()))?;
    expand_campaign_paths(&mut campaign);
    Ok(campaign)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn expand_tilde_forms() {
        let home = std::env::var("HOME").unwrap();
        assert_eq!(expand_tilde("~"), home);
        assert_eq!(expand_tilde("~/x/y"), format!("{home}/x/y"));
        assert_eq!(expand_tilde("rel/dir"), "rel/dir");
        assert_eq!(expand_tilde("/abs/dir"), "/abs/dir");
        // `~user` and `~x` pass through untouched.
        assert_eq!(expand_tilde("~root/x"), "~root/x");
        assert_eq!(expand_tilde("~x"), "~x");
    }

    /// Re-exec — the `$USERPROFILE`-then-`/` fallbacks need `HOME`
    /// unset, and edition-2024 `remove_var` is unsafe; the child
    /// process env is the clean seam (same pattern as peregrin-git's
    /// `expand_path_homeless`).
    #[test]
    fn expand_tilde_homeless() {
        if std::env::var_os("PERE_HOMELESS").is_some() {
            assert_eq!(expand_tilde("~"), "/");
            assert_eq!(expand_tilde("~/x"), "/x");
            return;
        }
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([std::thread::current().name().unwrap(), "--exact"])
            .env("PERE_HOMELESS", "1")
            .env_remove("HOME")
            .env_remove("USERPROFILE")
            .status()
            .unwrap();
        assert!(status.success());
    }

    #[test]
    fn expand_campaign_paths_only_touches_file_lists() {
        let home = std::env::var("HOME").unwrap();
        let raw = serde_json::json!({
            "include": ["~/bundles/ci.yaml", "ci-gate", "rel/snippet.yaml"],
            "import": [{"file": "~/lib/helpers.py", "as": "h"}, "plain.py"],
            "config": {"dir": "~/kept-literal"},
            "nodes": [{"id": "n", "driver": "git", "params": {"dir": "~/kept-literal"}}],
        });
        let mut campaign = peregrin_core::Campaign::from_dict(&raw).unwrap();
        expand_campaign_paths(&mut campaign);
        assert_eq!(campaign.imports[0], format!("{home}/bundles/ci.yaml"));
        assert_eq!(campaign.imports[1], "ci-gate");
        assert_eq!(campaign.imports[2], "rel/snippet.yaml");
        assert_eq!(
            campaign.python_imports[0].file,
            format!("{home}/lib/helpers.py")
        );
        assert_eq!(campaign.python_imports[0].alias, "h");
        assert_eq!(campaign.python_imports[1].file, "plain.py");
        // Untyped maps are driver-owned — left literal.
        assert_eq!(campaign.config["dir"], "~/kept-literal");
        assert_eq!(campaign.nodes[0].params["dir"], "~/kept-literal");
    }

    #[test]
    fn campaign_from_raw_normalizes_and_expands() {
        let path = Path::new("peregrin.yaml");
        let raw = serde_json::json!({
            "name": "gate",
            "include": ["~/b.yaml"],
            // `functions:` as a mapping — the `from_dict` normalization.
            "functions": {"f": {"driver": "shell"}},
        });
        let c = campaign_from_raw(&raw, path).unwrap();
        assert_eq!(c.name, "gate");
        assert_eq!(c.functions[0].name, "f");
        assert!(c.imports[0].ends_with("/b.yaml"));

        // Non-mapping documents are rejected.
        let e = campaign_from_raw(&serde_json::json!([1, 2]), path).unwrap_err();
        assert!(e.contains("must be a mapping"), "{e}");
        // `from_dict` shape errors surface as parse errors.
        let e = campaign_from_raw(&serde_json::json!({"nodes": {"a": 1}}), path).unwrap_err();
        assert!(e.contains("'nodes' must be an array"), "{e}");
        assert!(e.starts_with("parse "), "{e}");
    }
}
