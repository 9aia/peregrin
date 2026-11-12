//! The ops surface — observability verbs plus the forge webhook-admin
//! helpers `provision hooks` shares:
//!
//! `runs` / `explain` / `log` read the durable JSONL
//! event log (`peregrin-store-json`'s format — the same file
//! `campaign run --history` appends to) plus the node's own health
//! checks. `register_hook` / `remove_hook` upsert/delete repo
//! webhooks via the GitHub REST API (exact URL match; token +
//! per-repo secrets resolve by NAME through the env → keyring →
//! env-file chain — values never sit in config).
//!
//! Verbs print report lines and return them so tests assert without
//! scraping stdout.

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::{Value, json};

use hypha_dotenv::DotenvSecrets;
use hypha_keyring::KeyringSecrets;
use hypha_secret::Secrets;
use peregrin_config::ConfigSource;
use peregrin_core::Campaign;

use crate::CliError;

// --- environment ---------------------------------------------------------------

/// The webhook node's default hook path (`params.path`).
pub(crate) const DEFAULT_HOOK_PATH: &str = "/hook";
/// The default forge REST endpoint (`params.api_base`).
pub(crate) const DEFAULT_API_BASE: &str = "https://api.github.com";
/// The default forge-token secret NAME (`config.token_env`).
pub(crate) const DEFAULT_TOKEN_ENV: &str = "GITHUB_TOKEN";
/// The default worktree base (`config.worktree_base`).
const DEFAULT_WORKTREE_BASE: &str = "~/.local/share/peregrin/worktrees";

/// Store/config/unit/log roots — [`OpsEnv::default`] derives them
/// from `PEREGRIN_HOME`/XDG/`HOME`; tests inject tempdirs via
/// [`OpsEnv::under`].
#[derive(Clone, Debug)]
pub struct OpsEnv {
    /// The tamias install store (`$TAMIAS_HOME/store` →
    /// `XDG_DATA_HOME/tamias/store` → `~/.local/share/tamias/store`) —
    /// bundles are tamias packages; the `bundle.*` ops and
    /// `campaign --bundle` resolve against the same store `tamias
    /// install` writes.
    pub store_root: PathBuf,
    /// The tamias fetch-cache root (`$TAMIAS_CACHE` →
    /// `XDG_CACHE_HOME/tamias/fetch` → `~/.cache/tamias/fetch`) the
    /// bundle verbs' install pipeline writes through.
    pub cache_root: PathBuf,
    /// `~/.config/peregrin` — node configs + the `env` secrets file.
    pub config_dir: PathBuf,
    /// `~/.config/systemd/user` — per-user units.
    pub unit_dir: PathBuf,
    /// `PEREGRIN_HOME` (`~/.local/share/peregrin`) — durable logs.
    pub home: PathBuf,
}

impl OpsEnv {
    /// All roots under one sandbox dir (tests).
    #[must_use]
    pub fn under(root: &Path) -> Self {
        Self {
            store_root: root.join("store"),
            cache_root: root.join("cache"),
            config_dir: root.join("config"),
            unit_dir: root.join("units"),
            home: root.join("home"),
        }
    }

    /// The secrets env file the unit's `EnvironmentFile` reads and
    /// the secret chain's file leg resolves from.
    #[must_use]
    pub fn env_file(&self) -> PathBuf {
        self.config_dir.join("env")
    }

    /// The durable event log `runs`/`explain`/`log` read — the file
    /// `campaign run --history` appends to.
    #[must_use]
    pub fn history_path(&self) -> PathBuf {
        self.home.join("history.jsonl")
    }
}

impl Default for OpsEnv {
    fn default() -> Self {
        Self {
            store_root: tamias_store_root(),
            cache_root: tamias_cache_root(),
            config_dir: config_dir(),
            unit_dir: hypha_systemd::default_unit_dir(),
            home: peregrin_node::peregrin_home(),
        }
    }
}

/// The tamias install-store root — `tamias-cli`'s `store_root` chain
/// verbatim (`TAMIAS_HOME`/`XDG_DATA_HOME`/`HOME`), so `bundle.*` ops
/// see exactly what `tamias install` committed.
fn tamias_store_root() -> PathBuf {
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
/// verbatim (`TAMIAS_CACHE`/`XDG_CACHE_HOME`/`HOME`).
fn tamias_cache_root() -> PathBuf {
    if let Some(root) = std::env::var_os("TAMIAS_CACHE") {
        return PathBuf::from(root);
    }
    std::env::var_os("XDG_CACHE_HOME")
        .map(|d| PathBuf::from(d).join("tamias/fetch"))
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache/tamias/fetch")))
        .unwrap_or_else(|| PathBuf::from(".tamias/fetch"))
}

/// `XDG_CONFIG_HOME/peregrin` → `~/.config/peregrin` (the
/// scarab-config discovery dir).
fn config_dir() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(|d| PathBuf::from(d).join("peregrin"))
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config/peregrin")))
        .or_else(|| {
            std::env::var_os("USERPROFILE").map(|h| PathBuf::from(h).join(".config/peregrin"))
        })
        .unwrap_or_else(|| PathBuf::from(".config/peregrin"))
}

fn cli(e: impl std::fmt::Display) -> CliError {
    CliError(e.to_string())
}

// --- secrets ---------------------------------------------------------------------

/// The secret chain — environment → cuora broker → OS keyring →
/// the config-dir env file (the bupha/mimus order: cuora is the
/// canonical store, keyring + env file are legacy read-throughs
/// kept during the migration). Verbs read secret *names* from
/// config; values only ever resolve through here (the per-impl
/// binding rule).
pub(crate) fn secret(env: &OpsEnv, name: &str) -> Option<String> {
    secret_with(
        env,
        name,
        peregrin_secret_cuora::CuoraSecrets::shared_default(),
    )
}

/// [`secret`] with an explicit cuora leg — the test seam (a stub
/// `cuora` binary on `CuoraSecrets.bin`).
fn secret_with(
    env: &OpsEnv,
    name: &str,
    cuora: &peregrin_secret_cuora::CuoraSecrets,
) -> Option<String> {
    std::env::var(name)
        .ok()
        .filter(|v| !v.is_empty())
        .or_else(|| cuora.get(name))
        .or_else(|| KeyringSecrets::default().get(name))
        .or_else(|| {
            DotenvSecrets {
                path: env.env_file(),
            }
            .get(name)
        })
}

/// The unresolved-secret error — names the file the value could land in.
pub(crate) fn missing_secret(env: &OpsEnv, name: &str) -> CliError {
    CliError(format!(
        "secret '{name}' unresolved — set it in the environment, cuora \
         (cuora://peregrin/{name}), the OS keyring (service 'peregrin') \
         or {}",
        env.env_file().display()
    ))
}

// --- node config -------------------------------------------------------------------

/// Resolve the node-config path: `--config`, else the
/// scarab-config file discovery over `[".", env.config_dir]` —
/// `peregrin.{yaml,yml,toml}`, YAML preferred within each dir
/// (P-017; dir-major order, the org convention — the old hand-rolled
/// loop was format-major across the two dirs). `Value` is only the
/// candidate carrier: `locate()` returns a path and `load_config`
/// does the real parse (YAML via the campaign loader's include
/// expansion, TOML via `TomlSource`).
pub fn find_config(env: &OpsEnv, explicit: Option<&Path>) -> Result<PathBuf, CliError> {
    if let Some(path) = explicit {
        if path.exists() {
            return Ok(path.to_path_buf());
        }
        return Err(CliError(format!(
            "config {} does not exist",
            path.display()
        )));
    }
    let source = scarab_config_file::FileSource::<Value>::new(
        scarab_config::Discovery::bare("peregrin")
            .dirs([PathBuf::from("."), env.config_dir.clone()]),
    );
    source.locate().map_err(|_| {
        CliError(format!(
            "no peregrin.{{yaml,yml,toml}} found (search: ./, {}) — pass --config",
            env.config_dir.display()
        ))
    })
}

/// Load the node config — yaml paths expand `include:`/snippets via
/// the campaign loader; TOML goes through `peregrin-config-toml`.
fn load_config(path: &Path) -> Result<Campaign, CliError> {
    let is_yaml = matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("yaml" | "yml")
    );
    if is_yaml {
        peregrin_node::campaign::load(path).map_err(cli)
    } else {
        peregrin_config_toml::TomlSource {
            path: path.to_path_buf(),
        }
        .load()
        .map_err(CliError)
    }
}

/// The node config's `config.<key>` as a literal string.
fn config_str(campaign: &Campaign, key: &str) -> Option<String> {
    campaign
        .config
        .get(key)
        .and_then(Value::as_str)
        .filter(|v| !v.is_empty())
        .map(str::to_string)
}

// --- github hook admin ---------------------------------------------------------------

/// Outbound GitHub API calls are bounded — a stalled API can't hang
/// the verb (bupha's 15s).
fn gh_agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(15)))
        .build()
        .new_agent()
}

/// `GET /repos/{repo}/hooks` → the id of the hook whose `config.url`
/// matches `hook_url` exactly (trailing slash normalized) — the
/// takeover rule. Auth/scope failures are errors, not "no hook"
/// (they'd otherwise fall through to a confusing POST).
fn find_hook(
    agent: &ureq::Agent,
    api_base: &str,
    repo: &str,
    hook_url: &str,
    token: &str,
) -> Result<Option<u64>, CliError> {
    let list = agent
        .get(format!("{api_base}/repos/{repo}/hooks"))
        .header("Authorization", &format!("Bearer {token}"))
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "peregrin")
        .call();
    match list {
        Ok(mut response) => {
            let body: Value = response
                .body_mut()
                .read_json()
                .unwrap_or_else(|_| json!([]));
            Ok(body.as_array().and_then(|hooks| {
                hooks.iter().find_map(|hook| {
                    let id = hook.get("id")?.as_u64()?;
                    let url = hook.pointer("/config/url")?.as_str()?;
                    (url.trim_end_matches('/') == hook_url).then_some(id)
                })
            }))
        }
        Err(e) => {
            if e.to_string().contains("401") || e.to_string().contains("403") {
                Err(CliError(format!(
                    "{e} — the token needs repo + admin:repo_hook scopes"
                )))
            } else {
                Ok(None)
            }
        }
    }
}

/// Register or update the repo's webhook pointing at `hook_url`
/// (`POST`/`PATCH /repos/{repo}/hooks`) — the upsert rule:
/// only a hook already pointing at this exact URL is taken over.
/// `events` is the hook's subscribed event-type list. Returns the
/// human message.
pub fn register_hook(
    token: &str,
    repo: &str,
    hook_url: &str,
    secret: &str,
    api_base: &str,
    events: &[&str],
) -> Result<String, CliError> {
    let agent = gh_agent();
    let existing = find_hook(&agent, api_base, repo, hook_url, token)?;
    let config = json!({"url": hook_url, "content_type": "json", "secret": secret});
    let events = events.to_vec();
    if let Some(id) = existing {
        agent
            .patch(format!("{api_base}/repos/{repo}/hooks/{id}"))
            .header("Authorization", &format!("Bearer {token}"))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "peregrin")
            .send_json(json!({"config": config, "events": events, "active": true}))
            .map_err(|e| CliError(format!("update hook {id} on {repo}: {e}")))?;
        Ok(format!("{repo}: updated hook {id} -> {hook_url}"))
    } else {
        agent
            .post(format!("{api_base}/repos/{repo}/hooks"))
            .header("Authorization", &format!("Bearer {token}"))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "peregrin")
            .send_json(json!({
                "name": "web", "config": config,
                "events": events, "active": true}))
            .map_err(|e| CliError(format!("create hook on {repo}: {e}")))?;
        Ok(format!("{repo}: created hook -> {hook_url}"))
    }
}

/// Delete the repo's webhook whose `config.url` matches `hook_url`
/// (`DELETE /repos/{repo}/hooks/{id}`) — `bupha destroy webhook`'s
/// exact-match rule. Returns the human message.
pub fn remove_hook(
    token: &str,
    repo: &str,
    hook_url: &str,
    api_base: &str,
) -> Result<String, CliError> {
    let agent = gh_agent();
    match find_hook(&agent, api_base, repo, hook_url, token)? {
        Some(id) => {
            agent
                .delete(format!("{api_base}/repos/{repo}/hooks/{id}"))
                .header("Authorization", &format!("Bearer {token}"))
                .header("Accept", "application/vnd.github+json")
                .header("User-Agent", "peregrin")
                .call()
                .map_err(|e| CliError(format!("delete hook {id} on {repo}: {e}")))?;
            Ok(format!("{repo}: deleted hook {id} ({hook_url})"))
        }
        None => Ok(format!("{repo}: no hook for {hook_url}")),
    }
}

/// The worktree base — `config.worktree_base` when a config loads,
/// else the driver's own default. A missing config is not an error
/// here (teardown works without one); an unreadable one is.
fn worktree_base(env: &OpsEnv, config: Option<&Path>) -> Result<PathBuf, CliError> {
    match find_config(env, config) {
        Ok(path) => {
            let campaign = load_config(&path)?;
            let base = config_str(&campaign, "worktree_base")
                .unwrap_or_else(|| DEFAULT_WORKTREE_BASE.to_string());
            Ok(PathBuf::from(peregrin_config::expand_tilde(&base)))
        }
        Err(e) => {
            if config.is_some() {
                Err(e)
            } else {
                Ok(PathBuf::from(peregrin_config::expand_tilde(
                    DEFAULT_WORKTREE_BASE,
                )))
            }
        }
    }
}

// --- observability ---------------------------------------------------------------------

/// Read the JSONL log — one `serde_json::Value` per non-empty line;
/// malformed lines skip (bupha's `continue`).
fn read_log(path: &Path) -> Result<Vec<Value>, CliError> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(CliError(format!("read {}: {e}", path.display()))),
    };
    Ok(text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect())
}

/// An event's push identity — `payload.sha`, else the sturn-style
/// `payload.after`.
fn event_sha(event: &Value) -> Option<&str> {
    for key in ["sha", "after"] {
        if let Some(sha) = event
            .pointer(&format!("/payload/{key}"))
            .and_then(Value::as_str)
        {
            if !sha.is_empty() {
                return Some(sha);
            }
        }
    }
    None
}

fn event_kind(event: &Value) -> &str {
    event
        .get("type")
        .or_else(|| event.get("kind"))
        .and_then(Value::as_str)
        .unwrap_or("")
}

fn event_ts(event: &Value) -> i64 {
    event.get("ts").and_then(Value::as_i64).unwrap_or(0)
}

/// A per-push run record — the group of events sharing a `payload.sha`
/// (bupha's one-history-row-per-push parity).
pub struct RunRecord {
    /// The pushed sha.
    pub sha: String,
    /// The repo the first sha-carrying event names (`payload.repo`).
    pub repo: String,
    /// done | failed | cancelled | superseded | running.
    pub status: String,
    /// First event ts (ms).
    pub first_ts: i64,
    /// Last event ts (ms).
    pub last_ts: i64,
    /// The group's events (in log order).
    pub events: Vec<Value>,
}

/// The event kind's trailing type segment — `ci-gate.done` → `done`
/// (the verdict marker the routed-type convention carries).
fn type_segment(kind: &str) -> &str {
    kind.rsplit('.').next().unwrap_or(kind)
}

/// The run status — the last verdict-type event in the group wins:
/// `*.failed`/`workflow.failed`/`node.failed` → `failed`,
/// `workflow.cancelled`/`*cancelled` → `cancelled`,
/// `*.done`/`workflow.done` → `done`, `*.superseded` → `superseded`
/// (the queue's displaced-pending marker — the trigger never ran),
/// else `running`.
fn run_status(events: &[Value]) -> String {
    let mut status = "running";
    for event in events {
        status = match type_segment(event_kind(event)) {
            "cancelled" => "cancelled",
            "failed" => "failed",
            "done" => "done",
            "superseded" => "superseded",
            _ => status,
        };
    }
    status.to_string()
}

/// Group the log's sha-carrying events into run records, first-seen
/// order preserved. Events without a sha (start, listening,
/// workflow-level) are run-level, not run records.
fn collect_runs(events: &[Value]) -> Vec<RunRecord> {
    let mut runs: Vec<RunRecord> = Vec::new();
    for event in events {
        let Some(sha) = event_sha(event) else {
            continue;
        };
        if let Some(rec) = runs.iter_mut().find(|rec| rec.sha == sha) {
            rec.last_ts = event_ts(event).max(rec.last_ts);
            rec.events.push(event.clone());
        } else {
            let repo = event
                .pointer("/payload/repo")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            runs.push(RunRecord {
                sha: sha.to_string(),
                repo,
                status: String::new(),
                first_ts: event_ts(event),
                last_ts: event_ts(event),
                events: vec![event.clone()],
            });
        }
    }
    for rec in &mut runs {
        rec.status = run_status(&rec.events);
        if rec.repo.is_empty() {
            if let Some(repo) = rec
                .events
                .iter()
                .find_map(|e| e.pointer("/payload/repo").and_then(Value::as_str))
            {
                rec.repo = repo.to_string();
            }
        }
    }
    runs
}

fn short(sha: &str) -> &str {
    sha.get(..12).unwrap_or(sha)
}

/// `runs` — the run records newest-first (`{sha, repo, status, wall,
/// events}` columns).
pub fn runs(
    env: &OpsEnv,
    history: Option<&Path>,
    repo: Option<&str>,
    limit: Option<usize>,
) -> Result<Vec<String>, CliError> {
    let path = history.map_or_else(|| env.history_path(), Path::to_path_buf);
    let events = read_log(&path)?;
    if events.is_empty() {
        println!("no runs yet ({})", path.display());
        return Ok(Vec::new());
    }
    let mut recs: Vec<RunRecord> = collect_runs(&events)
        .into_iter()
        .filter(|rec| {
            repo.is_none_or(|want| rec.repo == want || rec.repo.starts_with(&format!("{want}:")))
        })
        .collect();
    recs.sort_by_key(|rec| std::cmp::Reverse(rec.last_ts));
    if let Some(limit) = limit {
        recs.truncate(limit);
    }
    let mut lines = vec![format!(
        "{:<12} {:<30} {:<9} {:>6} {:>6}",
        "sha", "repo", "status", "wall", "events"
    )];
    for rec in &recs {
        lines.push(format!(
            "{:<12} {:<30} {:<9} {:>5}s {:>6}",
            short(&rec.sha),
            rec.repo,
            rec.status,
            (rec.last_ts - rec.first_ts).max(0) / 1000,
            rec.events.len()
        ));
    }
    for line in &lines {
        println!("{line}");
    }
    Ok(lines)
}

/// `explain <sha|id>` — dump a run's events + outcome. Matches a sha
/// prefix first (the whole push group), then an event-id prefix, then
/// a leftover worktree dir — mirroring `bupha explain`'s
/// "ran?/queued?/died?/never seen" ladder.
pub fn explain(
    env: &OpsEnv,
    history: Option<&Path>,
    config: Option<&Path>,
    needle: &str,
) -> Result<Vec<String>, CliError> {
    let path = history.map_or_else(|| env.history_path(), Path::to_path_buf);
    let events = read_log(&path)?;
    let mut lines = Vec::new();
    // 1. A run record whose sha starts with the needle.
    for rec in collect_runs(&events) {
        if rec.sha.starts_with(needle) {
            lines.push(format!(
                "run {} — {} ({}) in {}s, {} events",
                short(&rec.sha),
                rec.status,
                if rec.repo.is_empty() { "?" } else { &rec.repo },
                (rec.last_ts - rec.first_ts).max(0) / 1000,
                rec.events.len()
            ));
            for event in &rec.events {
                lines.push(format!(
                    "  {} {} {}",
                    event_ts(event),
                    event.get("source").and_then(Value::as_str).unwrap_or("?"),
                    event_kind(event)
                ));
            }
            let verdict =
                rec.events.iter().rev().find(|e| {
                    matches!(type_segment(event_kind(e)), "done" | "failed" | "cancelled")
                });
            if let Some(event) = verdict {
                lines.push(format!(
                    "outcome: {} {}",
                    event_kind(event),
                    event
                        .get("payload")
                        .map_or_else(|| "{}".into(), Value::to_string)
                ));
            }
            for line in &lines {
                println!("{line}");
            }
            return Ok(lines);
        }
    }
    // 2. Events whose own id matches — a partial run.
    let matched: Vec<&Value> = events
        .iter()
        .filter(|e| {
            e.get("id")
                .and_then(Value::as_str)
                .is_some_and(|id| id.starts_with(needle))
        })
        .collect();
    if !matched.is_empty() {
        lines.push(format!(
            "{} event(s) matching id '{needle}':",
            matched.len()
        ));
        for event in matched {
            lines.push(format!(
                "  {} {} {}",
                event_ts(event),
                event.get("source").and_then(Value::as_str).unwrap_or("?"),
                event_kind(event)
            ));
        }
        for line in &lines {
            println!("{line}");
        }
        return Ok(lines);
    }
    // 3. A worktree dir named `*-<sha>` — a run that died mid-flight.
    if let Ok(base) = worktree_base(env, config)
        && base.is_dir()
        && let Ok(rd) = std::fs::read_dir(&base)
    {
        for entry in rd.flatten() {
            if entry.file_name().to_string_lossy().ends_with(needle) {
                lines.push(format!(
                    "worktree exists ({}), no result — run died mid-flight",
                    entry.path().display()
                ));
                println!("{}", lines[0]);
                return Ok(lines);
            }
        }
    }
    lines.push(format!(
        "{needle}: never seen — not in history or worktrees"
    ));
    println!("{}", lines[0]);
    Ok(lines)
}

/// `log` — the last `n` raw lines of the JSONL store (bupha's
/// `log -n` shape).
pub fn log(env: &OpsEnv, history: Option<&Path>, n: usize) -> Result<Vec<String>, CliError> {
    let path = history.map_or_else(|| env.history_path(), Path::to_path_buf);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            println!("no log yet ({})", path.display());
            return Ok(Vec::new());
        }
        Err(e) => return Err(CliError(format!("read {}: {e}", path.display()))),
    };
    let lines: Vec<String> = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(str::to_string)
        .collect();
    let tail: Vec<String> = lines.iter().rev().take(n).rev().cloned().collect();
    for line in &tail {
        println!("{line}");
    }
    Ok(tail)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    #[test]
    fn run_status_marks_superseded_records() {
        let event = |kind: &str, sha: &str| json!({"type": kind, "payload": {"sha": sha}});
        // A displaced pending trigger: only its `*.superseded` marker
        // ever lands in the log → status `superseded`.
        let events = vec![event("push.superseded", "abc123")];
        assert_eq!(run_status(&events), "superseded");
        // A real run's verdict still wins over an earlier marker for
        // the same sha (re-push of the same commit).
        let events = vec![
            event("push.superseded", "abc123"),
            event("push", "abc123"),
            event("ci-gate.done", "abc123"),
        ];
        assert_eq!(run_status(&events), "done");
        // And `collect_runs` groups the marker under its sha/repo.
        let recs = collect_runs(&[json!({
            "type": "push.superseded",
            "ts": 1,
            "payload": {"sha": "abc123", "repo": "o/r", "superseded": true},
        })]);
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].sha, "abc123");
        assert_eq!(recs[0].repo, "o/r");
        assert_eq!(recs[0].status, "superseded");
    }

    /// A stub `cuora` binary — `get` prints `value`, anything else
    /// exits 1. `fail` makes every call exit 1.
    fn stub_cuora(dir: &Path, value: &str, fail: bool) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let script = if fail {
            "#!/bin/sh\nexit 1\n".to_string()
        } else {
            format!(
                "#!/bin/sh\nprintf '%s\\n' '{}'\n",
                value.replace('\'', "'\\''")
            )
        };
        let bin = dir.join("cuora-stub.sh");
        std::fs::write(&bin, script).unwrap();
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
        bin
    }

    /// The secret chain resolves cuora ahead of the env file: the
    /// same name in both stores answers with the cuora value, and a
    /// failing cuora falls through to the file. (The env leg can't
    /// be exercised — edition-2024 forbids `env::set_var` — and the
    /// keyring leg can't be stubbed without PATH fakery; the order
    /// env → cuora → keyring → file is the code's `.or_else` chain.)
    #[test]
    fn secret_chain_prefers_cuora_then_falls_through() {
        let dir = tempfile::tempdir().unwrap();
        let env = OpsEnv::under(dir.path());
        std::fs::create_dir_all(env.config_dir.clone()).unwrap();
        let name = "PEREGRIN_SECRET_CHAIN_TEST";
        std::fs::write(env.env_file(), format!("{name}=from-env-file\n")).unwrap();

        let cuora = peregrin_secret_cuora::CuoraSecrets::with_bin(stub_cuora(
            dir.path(),
            "from-cuora",
            false,
        ));
        assert_eq!(
            secret_with(&env, name, &cuora).as_deref(),
            Some("from-cuora")
        );

        let failing =
            peregrin_secret_cuora::CuoraSecrets::with_bin(stub_cuora(dir.path(), "", true));
        assert_eq!(
            secret_with(&env, name, &failing).as_deref(),
            Some("from-env-file")
        );
    }
}
