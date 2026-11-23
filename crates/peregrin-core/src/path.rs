//! Path helpers shared by every driver — the `~` expansion applied at
//! param boundaries (`cwd`, `path`, `repo`, `prompt_file`, …) so a
//! leading `~` means the user's home instead of a literal directory
//! name passed to `spawn`/`fs`.

use std::path::{Path, PathBuf};

/// `~` / `~/…` expansion — `$HOME`, else `$USERPROFILE`, else `/`.
/// Non-`~` paths and non-UTF-8 paths pass through untouched; `~user`
/// forms and `$VAR` are *not* expanded (paths are data — env
/// interpolation is `${env()}`'s job).
#[must_use]
pub fn expand_path(path: &Path) -> PathBuf {
    let Some(text) = path.to_str() else {
        return path.to_path_buf();
    };
    if text == "~" {
        return home_dir();
    }
    if let Some(rest) = text.strip_prefix("~/") {
        return home_dir().join(rest);
    }
    path.to_path_buf()
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map_or_else(|| PathBuf::from("/"), PathBuf::from)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn expand_tilde_and_tilde_slash() {
        let home = home_dir();
        assert_eq!(expand_path(Path::new("~")), home);
        assert_eq!(expand_path(Path::new("~/x/y")), home.join("x/y"));
    }

    #[test]
    fn passthrough_for_non_tilde() {
        assert_eq!(expand_path(Path::new("rel/dir")), PathBuf::from("rel/dir"));
        assert_eq!(
            expand_path(Path::new("/abs/dir")),
            PathBuf::from("/abs/dir")
        );
        // `~user` and `$VAR` spellings are not expanded.
        assert_eq!(expand_path(Path::new("~root/x")), PathBuf::from("~root/x"));
        assert_eq!(expand_path(Path::new("$HOME/x")), PathBuf::from("$HOME/x"));
    }
}
