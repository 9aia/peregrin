//! The env surface spawned children may see.
//!
//! Children spawned by drivers (`sh -c`, agent binaries) and template
//! builtins (`${shell()}`) must not inherit the whole process
//! environment: the daemon/CLI env routinely carries secrets —
//! `PEREGRIN_TOKEN`, API keys, values materialized by secret brokers
//! (cuora `run`, direnv, systemd `EnvironmentFile`). An allowlist keeps
//! only what a child needs to function; workflows add their own vars
//! through `params.env` / `params.secrets`.

/// Vars a spawned child keeps — locale, identity, terminal, temp dirs,
/// and proxy config (corporate agents can't route without it). Anything
/// else — every `*_TOKEN`, `*_KEY`, `*_SECRET`, session/agent state —
/// stays in the parent.
pub const CHILD_ENV_ALLOWLIST: &[&str] = &[
    "PATH",
    "HOME",
    "USER",
    "LOGNAME",
    "SHELL",
    "LANG",
    "LC_ALL",
    "LC_CTYPE",
    "TZ",
    "TMPDIR",
    "TERM",
    "COLORTERM",
    // Windows process basics — cmd children fail without these.
    "SystemRoot",
    "SystemDrive",
    "windir",
    "COMSPEC",
    "PATHEXT",
    "USERPROFILE",
    "APPDATA",
    "LOCALAPPDATA",
    "TEMP",
    "TMP",
    "OS",
    "NUMBER_OF_PROCESSORS",
    // Proxy config — not secret-bearing in practice, and agents
    // behind a corporate proxy can't reach anything without it.
    "HTTP_PROXY",
    "HTTPS_PROXY",
    "NO_PROXY",
    "http_proxy",
    "https_proxy",
    "no_proxy",
];

/// Filter a var list down to [`CHILD_ENV_ALLOWLIST`].
pub fn sanitize_env<I, K, V>(vars: I) -> Vec<(String, String)>
where
    I: IntoIterator<Item = (K, V)>,
    K: AsRef<str>,
    V: Into<String>,
{
    vars.into_iter()
        .filter(|(key, _)| CHILD_ENV_ALLOWLIST.contains(&key.as_ref()))
        .map(|(key, value)| (key.as_ref().to_string(), value.into()))
        .collect()
}

/// The current process env, sanitized — the base environment for any
/// child peregrin spawns.
#[must_use]
pub fn child_env() -> Vec<(String, String)> {
    sanitize_env(std::env::vars())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_keeps_allowlist_drops_secrets() {
        let vars = vec![
            ("PATH", "/bin"),
            ("PEREGRIN_TOKEN", "tok"),
            ("AWS_SECRET_ACCESS_KEY", "aws"),
            ("MY_SECRET", "s"),
            ("LANG", "en_US.UTF-8"),
        ];
        let out = sanitize_env(vars);
        assert_eq!(
            out,
            vec![
                ("PATH".to_string(), "/bin".to_string()),
                ("LANG".to_string(), "en_US.UTF-8".to_string()),
            ]
        );
    }

    #[test]
    fn child_env_excludes_known_secret_names() {
        // `child_env` filters a snapshot of this process's own env —
        // nothing on the host may legitimately carry these names.
        let out = child_env();
        for (key, _) in &out {
            assert!(CHILD_ENV_ALLOWLIST.contains(&key.as_str()), "{key}");
        }
    }
}
