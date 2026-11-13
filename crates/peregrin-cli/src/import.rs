//! `peregrin import` — foreign config → peregrin node-config workflow
//! conversion (ADR-0081: "bupha.toml/bupha.yaml → campaign-param
//! importer"). `import bupha <path>` reads a bupha config (TOML or
//! YAML — the file extension picks the parser; serde_norway for YAML,
//! toml for TOML) and emits a node-config workflow: an ingress
//! `github-webhook` node carrying the `repos`+`secret_env` watch list,
//! `sturn-poll` nodes for `[trigger.sturn]`-opted repos, a router
//! node, one gate pipeline per repo/job, and the campaign `supersede:`
//! block.
//!
//! Mapping (ADR-0081's locked table):
//!
//! - `repos:` → the webhook node's `params.repos` watch list
//!   (`{name, secret_env}` — secret NAMES pass through, never values).
//! - `jobs:`/`branch:` → each job's `on:` subscription
//!   (`push.<name>.<branch>.**.*` over the routed-push contract — see
//!   below) plus driver params.
//! - `runner.cmd` (kind `script`/`shell`) → the git driver's inner
//!   shell `cmd`. Other runner kinds (dagger, ssh, http, …) have no
//!   peregrin driver — the job is emitted with a loudly-failing stub
//!   cmd and a warning.
//! - `cancel_running` → `supersede.cancel_running`; `queue = "all"` →
//!   `mode: fifo` (a mixed fleet warns — supersede is per-campaign).
//! - `secret_env` → webhook `repos[].secret_env` (repo value, else the
//!   trigger fallback, else bupha's `BUPHA_WEBHOOK_SECRET` default).
//! - `dir` → dropped — driver-git clones from the event's `clone_url`
//!   (warned when the dir's basename differs from the repo's, and used
//!   as the `sturn-poll` watch `repo` for sturn-opted repos).
//! - `status_repo` → the sink's `status_repo` param; `status.context`
//!   composes per bupha (`<context>` implicit, `<context>/<job>` for
//!   named jobs, `context` override wins).
//! - `worktree.base` → driver-git `base`; `runner.token_env` → the git
//!   `token_env` + sink `token_env` names.
//! - `tunnel`, `cache`, `access`, `audit`, `telemetry`, `attest`,
//!   `queue`, `worker`, `agent` → dropped with warnings (no peregrin
//!   surface yet; cloudflared stays external per ADR-0080).
//!
//! ## The routed-push contract
//!
//! `on:` matches event *types* only (no payload expressions — the
//! ADR-0081 glob ruling), so `ci.route` re-emits every verified push
//! as `push.<repo>.<ref>.<clone_url>.<sha>` — the payload fields ride
//! the type via a template-resolved `emit` value — and each job's git
//! node subscribes `push.<name>.<branch>.**.*`, recovering `clone_url`
//! (`${event.match.1}` — `**` re-joins the `.`-separated segments) and
//! `sha` (`${event.match.2}`). This is the same mechanics
//! the retired `ci-gate` bundle documented; the importer emits concrete patterns
//! instead of `${config.*}` indirection because `on:` is load-time
//! fixed.

use std::collections::BTreeSet;
use std::path::Path;

use serde::Deserialize;
use serde_json::{Map, Value, json};

use crate::CliError;

/// bupha's `default_branch`.
const DEFAULT_BRANCH: &str = "refs/heads/main";
/// bupha's `default_secret_env` (`[trigger] secret_env` fallback).
const DEFAULT_SECRET_ENV: &str = "BUPHA_WEBHOOK_SECRET";
/// bupha's `default_listen`.
const DEFAULT_LISTEN: &str = "127.0.0.1:29173";
/// bupha's `default_context` (`[status] context`).
const DEFAULT_CONTEXT: &str = "ci/bupha";
/// Worktree base emitted for every job — driver-git's own default;
/// bupha's `worktree.base` is its runtime dir and never imported.
const DEFAULT_WORKTREE_BASE: &str = "~/.local/share/peregrin/worktrees";
/// Runner kinds that map onto the inner `driver: shell`.
const SHELL_RUNNER_KINDS: &[&str] = &["script", "shell"];
/// Runner-extra keys with their own warning text — anything else gets
/// the generic "unmapped" warning. (`isolate`/`memory_max` are absent:
/// they map onto driver-shell's systemd-run scope and aren't dropped.)
const KNOWN_DROPPED_RUNNER_KEYS: &[&str] = &[
    "sccache_conf",
    "cli_source",
    "function",
    "shards",
    "jobs",
    "dogfood_jobs",
    "sequential",
];

// --- input schema --------------------------------------------------------------

/// `[trigger.sturn]` — the WAL poller source.
#[derive(Debug, Default, Deserialize)]
struct SturnTriggerCfg {
    #[serde(default)]
    remote: Option<String>,
    #[serde(default)]
    poll_ms: Option<u64>,
}

/// `[trigger]` — listener + forge + optional sturn poller.
#[derive(Debug, Default, Deserialize)]
struct TriggerCfg {
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    listen: Option<String>,
    #[serde(default)]
    secret_env: Option<String>,
    #[serde(default)]
    api_base: Option<String>,
    #[serde(default)]
    sturn: Option<SturnTriggerCfg>,
}

/// `[runner]` / `[repos.runner]` / `[repos.jobs.runner]` — `extra`
/// swallows the impl-owned fields (`cmd`, dagger knobs, isolation).
#[derive(Clone, Debug, Default, Deserialize)]
struct RunnerCfg {
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    token_env: Option<String>,
    #[serde(default)]
    cancel_running: Option<bool>,
    #[serde(flatten)]
    extra: Map<String, Value>,
}

/// `[[repos.jobs]]` — one pipeline inside a watched repo.
#[derive(Clone, Debug, Default, Deserialize)]
struct JobEntry {
    #[serde(default)]
    name: String,
    #[serde(default)]
    branch: Option<String>,
    #[serde(default)]
    runner: Option<RunnerCfg>,
    #[serde(default)]
    queue: Option<String>,
    #[serde(default)]
    context: Option<String>,
    #[serde(default)]
    runs_on: Option<Vec<String>>,
    #[serde(default)]
    retry: Option<u32>,
    #[serde(flatten)]
    extra: Map<String, Value>,
}

/// `[[repos]]` (bupha's `[[repo]]` alias accepted too).
#[derive(Debug, Default, Deserialize)]
struct RepoEntry {
    #[serde(default)]
    name: String,
    #[serde(default)]
    dir: Option<String>,
    #[serde(default)]
    branch: Option<String>,
    #[serde(default)]
    status_repo: Option<String>,
    #[serde(default)]
    runner: Option<RunnerCfg>,
    #[serde(default)]
    queue: Option<String>,
    #[serde(default)]
    secret_env: Option<String>,
    #[serde(default)]
    runs_on: Vec<String>,
    #[serde(default)]
    retry: u32,
    #[serde(default)]
    sturn: bool,
    #[serde(default)]
    jobs: Vec<JobEntry>,
    #[serde(flatten)]
    extra: Map<String, Value>,
}

/// `[status]`.
#[derive(Debug, Default, Deserialize)]
struct StatusCfg {
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    repo: Option<String>,
    #[serde(default)]
    context: Option<String>,
}

/// `[worktree]`.
#[derive(Debug, Default, Deserialize)]
struct WorktreeCfg {
    #[serde(default)]
    base: Option<String>,
}

/// The whole `bupha.toml`/`bupha.yaml` — every section bupha knows is
/// either a typed field above or lands in `extra` for a
/// dropped-with-warning note (cache/access/audit/telemetry/attest/
/// queue/worker/agent).
#[derive(Debug, Default, Deserialize)]
struct BuphaConfig {
    #[serde(default, alias = "repo")]
    repos: Vec<RepoEntry>,
    #[serde(default)]
    trigger: Option<TriggerCfg>,
    #[serde(default)]
    runner: Option<RunnerCfg>,
    #[serde(default)]
    status: Option<StatusCfg>,
    #[serde(default)]
    tunnel: Option<Value>,
    #[serde(default)]
    worktree: Option<WorktreeCfg>,
    #[serde(flatten)]
    extra: Map<String, Value>,
}

// --- resolved view (bupha's WatchedRepo, trimmed to what peregrin emits) ---------

/// One repo(+job) pair after job → repo → global inheritance.
struct Watched<'a> {
    repo: &'a RepoEntry,
    /// `name` for implicit, `name:job` for named — bupha's queue key.
    key: String,
    branch: String,
    context: String,
    runner: Option<&'a RunnerCfg>,
    queue_all: bool,
}

/// What the verb hands back — the emitted yaml plus the import's
/// observable facts (tests assert on these; the CLI prints them).
#[derive(Debug)]
pub struct ImportReport {
    /// The emitted node-config workflow.
    pub yaml: String,
    /// Conversion notes — dropped fields, unconvertible kinds,
    /// supersede conflicts. Printed to stderr by the verb.
    pub warnings: Vec<String>,
    /// Watched repos imported.
    pub repos: usize,
    /// Pipelines emitted (implicit + named jobs).
    pub jobs: usize,
    /// Nodes in the emitted workflow.
    pub nodes: usize,
}

/// Read + parse a bupha config — `.yaml`/`.yml` → serde_norway, else
/// TOML. The config-yaml transcode convention: YAML goes through
/// `serde_norway::Value` into `serde_json::Value` so both syntaxes
/// funnel into the same `BuphaConfig` deserialize.
fn load_bupha(path: &Path) -> Result<BuphaConfig, CliError> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| CliError(format!("read {}: {e}", path.display())))?;
    let is_yaml = matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("yaml" | "yml")
    );
    let raw: Value = if is_yaml {
        let value: serde_norway::Value = serde_norway::from_str(&text)
            .map_err(|e| CliError(format!("parse {}: {e}", path.display())))?;
        serde_norway::from_value(value)
            .map_err(|e| CliError(format!("parse {}: {e}", path.display())))?
    } else {
        let value: toml::Value = toml::from_str(&text)
            .map_err(|e| CliError(format!("parse {}: {e}", path.display())))?;
        serde_json::to_value(value)
            .map_err(|e| CliError(format!("parse {}: {e}", path.display())))?
    };
    if !raw.is_object() {
        return Err(CliError(format!(
            "parse {}: config must be a mapping",
            path.display()
        )));
    }
    serde_json::from_value(raw).map_err(|e| CliError(format!("parse {}: {e}", path.display())))
}

/// `~/…` expansion — the `peregrin_config::expand_tilde` convention
/// (driver-git expands `~` itself, but dir-vs-repo comparisons and
/// `on:` literals want the concrete path).
fn expand_tilde(path: &str) -> String {
    if path == "~" {
        return home();
    }
    if let Some(rest) = path.strip_prefix("~/") {
        return format!("{home}/{rest}", home = home());
    }
    path.into()
}

fn home() -> String {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map_or_else(|| "/".into(), |d| d.to_string_lossy().into_owned())
}

/// A node-id-safe slug: `9aia/myco:docs` → `9aia-myco-docs`.
fn slug(key: &str) -> String {
    let out: String = key
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') {
                c
            } else {
                '-'
            }
        })
        .collect();
    out.trim_matches('-').to_string()
}

/// Non-empty string helper over `Option<String>` fields.
fn non_empty(value: Option<&String>) -> Option<&str> {
    value.map(String::as_str).filter(|v| !v.is_empty())
}

/// Recursively collect every `secret_env`/`token_env` param value in a
/// params subtree — the names `capabilities.secrets` must grant.
fn collect_secrets(value: &Value, out: &mut BTreeSet<String>) {
    match value {
        Value::Object(map) => {
            for (k, v) in map {
                if matches!(k.as_str(), "secret_env" | "token_env")
                    && let Some(s) = v.as_str()
                {
                    out.insert(s.to_string());
                } else {
                    collect_secrets(v, out);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_secrets(item, out);
            }
        }
        _ => {}
    }
}

/// The effective webhook-secret name for a repo — repo override, then
/// the trigger fallback, then bupha's default. Secret *names* pass
/// through; values never enter the emitted config.
fn repo_secret_env(repo: &RepoEntry, trigger: Option<&TriggerCfg>) -> String {
    non_empty(repo.secret_env.as_ref())
        .or_else(|| trigger.and_then(|t| non_empty(t.secret_env.as_ref())))
        .unwrap_or(DEFAULT_SECRET_ENV)
        .to_string()
}

/// The status context bupha would post under: `status.context` for the
/// implicit pipeline, `<context>/<job>` for named jobs, `context`
/// override winning.
fn job_context(status: Option<&StatusCfg>, job: Option<&JobEntry>) -> String {
    let base = status
        .and_then(|s| non_empty(s.context.as_ref()))
        .unwrap_or(DEFAULT_CONTEXT);
    job.and_then(|j| non_empty(j.context.as_ref()).map(str::to_string))
        .or_else(|| job.map(|j| format!("{base}/{}", j.name)))
        .unwrap_or_else(|| base.to_string())
}

/// Resolve one repo(+job) — bupha's `watch_one` chain, trimmed to the
/// fields peregrin emits.
fn watch<'a>(
    config: &'a BuphaConfig,
    repo: &'a RepoEntry,
    job: Option<&'a JobEntry>,
) -> Watched<'a> {
    let runner = job
        .and_then(|j| j.runner.as_ref())
        .or(repo.runner.as_ref())
        .or(config.runner.as_ref());
    let queue = job
        .and_then(|j| j.queue.as_deref())
        .or(repo.queue.as_deref())
        .unwrap_or("latest");
    Watched {
        repo,
        key: job.map_or_else(
            || repo.name.clone(),
            |j| format!("{}:{}", repo.name, j.name),
        ),
        branch: job
            .and_then(|j| j.branch.clone())
            .or_else(|| repo.branch.clone())
            .unwrap_or_else(|| DEFAULT_BRANCH.to_string()),
        context: job_context(config.status.as_ref(), job),
        runner,
        queue_all: queue == "all",
    }
}

/// `on:` patterns for a job — the webhook identity (`repo` =
/// `repository.full_name` = `name`) plus, for sturn-opted repos, the
/// local-mode identity (`repo` = the watched dir) and a URL-shaped
/// `remote` when that's the clone identity. Remote *names*
/// (`origin`) resolve server-side — a warning asks the operator to
/// add the pattern.
fn on_patterns(
    watched: &Watched,
    warnings: &mut Vec<String>,
    sturn: Option<&SturnTriggerCfg>,
) -> Vec<Value> {
    let mut patterns = vec![format!(
        "push.{}.{}.**.*",
        watched.repo.name, watched.branch
    )];
    if watched.repo.sturn {
        if let Some(dir) = non_empty(watched.repo.dir.as_ref()) {
            patterns.push(format!(
                "push.{}.{}.**.*",
                expand_tilde(dir),
                watched.branch
            ));
        }
        let remote = sturn.and_then(|s| non_empty(s.remote.as_ref()));
        match remote {
            Some(r) if r.contains('/') || r.contains(':') => {
                patterns.push(format!("push.{r}.{}.**.*", watched.branch));
            }
            Some(name) => warnings.push(format!(
                "repo {}: [trigger.sturn] remote '{name}' resolves to its URL at \
                 runtime — add a `push.<resolved-url>.{}.**.*` on: pattern to the \
                 '{}' job if it differs",
                watched.repo.name, watched.branch, watched.key
            )),
            None => {}
        }
    }
    patterns.into_iter().map(Value::String).collect()
}

/// The three gate nodes for one repo(+job): `driver: git` build +
/// `github-status` pending + verdict sinks.
fn job_nodes(
    watched: &Watched,
    worktree_base: &str,
    api_base: Option<&str>,
    warnings: &mut Vec<String>,
    on: &[Value],
) -> Vec<Value> {
    let slug = slug(&watched.key);
    let done = format!("ci.{slug}.done");
    let failed = format!("ci.{slug}.failed");
    let id = format!("ci.{slug}");

    // runner → inner cmd (or a loudly-failing stub for kinds with no
    // peregrin driver — the job still gets its `on:` entry, visibly).
    let runner = watched.runner;
    let kind = runner
        .and_then(|r| non_empty(r.kind.as_ref()))
        .unwrap_or("script");
    let cmd = runner
        .and_then(|r| r.extra.get("cmd"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    let inner_cmd = if SHELL_RUNNER_KINDS.contains(&kind) {
        if cmd.is_empty() {
            warnings.push(format!(
                "job '{}': runner kind '{kind}' has no cmd — emitted a failing stub",
                watched.key
            ));
            format!(
                "echo 'import: job \"{}\" runner had no cmd' >&2; exit 127",
                watched.key
            )
        } else {
            cmd.to_string()
        }
    } else {
        warnings.push(format!(
            "job '{}': runner kind '{kind}' has no peregrin driver — emitted a \
             failing stub; convert manually",
            watched.key
        ));
        format!("echo 'import: runner kind \"{kind}\" is not convertible' >&2; exit 127")
    };
    for key in runner.map_or_else(Vec::new, |r| r.extra.keys().cloned().collect()) {
        if key == "cmd" {
            continue;
        }
        // Carried into the inner shell params below — not dropped.
        if key == "isolate" || key == "memory_max" {
            continue;
        }
        let why = if KNOWN_DROPPED_RUNNER_KEYS.contains(&key.as_str()) {
            "a dagger/impl-specific knob with no shell equivalent"
        } else {
            "unmapped runner key"
        };
        warnings.push(format!(
            "job '{}': runner.{key} dropped — {why}",
            watched.key
        ));
    }
    let token_env = runner
        .and_then(|r| non_empty(r.token_env.as_ref()))
        .unwrap_or("BUPHA_TOKEN")
        .to_string();

    let mut git_params = Map::new();
    git_params.insert("url".into(), json!("${event.match.1}"));
    git_params.insert("sha".into(), json!("${event.match.2}"));
    git_params.insert("base".into(), json!(worktree_base));
    git_params.insert("token_env".into(), json!(token_env));
    // `isolate`/`memory_max` now map onto driver-shell's systemd-run
    // scope — carry them into the inner params instead of dropping.
    let mut inner_params = Map::new();
    inner_params.insert("cmd".into(), json!(inner_cmd));
    inner_params.insert("timeout".into(), json!(600));
    if let Some(r) = runner {
        if r.extra.get("isolate").is_some_and(peregrin_core::js_truthy) {
            inner_params.insert("isolate".into(), json!(true));
        }
        if let Some(max) = r
            .extra
            .get("memory_max")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            inner_params.insert("memory_max".into(), json!(max));
        }
    }
    git_params.insert(
        "inner".into(),
        json!({
            "driver": "shell",
            // driver-shell defaults to 30s and caps at 600s — bupha
            // runs have no such bound, so gate cmds get the cap.
            "params": Value::Object(inner_params),
        }),
    );
    git_params.insert("emit".into(), json!({ "0": done, "default": failed }));

    let repo_field = watched.repo.name.clone();
    let mut sink_base = Map::new();
    sink_base.insert("repo".into(), json!(repo_field));
    if let Some(status_repo) = non_empty(watched.repo.status_repo.as_ref()) {
        sink_base.insert("status_repo".into(), json!(status_repo));
    }
    sink_base.insert("context".into(), json!(watched.context));
    sink_base.insert("token_env".into(), json!(token_env));
    if let Some(api) = api_base {
        sink_base.insert("api_base".into(), json!(api));
    }

    let mut pending_params = sink_base.clone();
    pending_params.insert("sha".into(), json!("${event.match.2}"));
    pending_params.insert("state".into(), json!("pending"));

    let mut verdict_params = sink_base;
    verdict_params.insert(
        "sha".into(),
        json!("${event.payload.sha:-${event.match.1}}"),
    );

    vec![
        json!({
            "id": id,
            "driver": "git",
            "on": on,
            "max_fires": "unlimited",
            "emits": [done, failed],
            "params": Value::Object(git_params),
        }),
        json!({
            "id": format!("{id}.pending"),
            "driver": "github-status",
            "on": on,
            "max_fires": "unlimited",
            "emits": [format!("{id}.status-posted")],
            "params": Value::Object(pending_params),
        }),
        json!({
            "id": format!("{id}.status"),
            "driver": "github-status",
            "on": [done.clone(), failed.clone(), format!("{failed}.*")],
            "max_fires": "unlimited",
            "emits": [format!("{id}.status-posted")],
            "params": Value::Object(verdict_params),
        }),
    ]
}

/// `bupha.toml`/`bupha.yaml` → the node-config workflow.
/// `warnings` collects the lossy half of the mapping.
fn convert(path: &Path, config: &BuphaConfig) -> (Value, Vec<String>, usize, usize) {
    let mut warnings: Vec<String> = Vec::new();
    let mut nodes: Vec<Value> = Vec::new();
    let trigger = config.trigger.as_ref();

    if config.repos.is_empty() {
        warnings.push("no [[repos]] entries — the emitted gate watches nothing".into());
    }
    {
        let mut seen = BTreeSet::new();
        for repo in &config.repos {
            if repo.name.is_empty() {
                warnings.push("a [[repos]] entry is missing 'name' — skipped".into());
                continue;
            }
            if !seen.insert(repo.name.clone()) {
                warnings.push(format!(
                    "repo {}: declared more than once — bupha rejects this; the \
                     last entry wins here",
                    repo.name
                ));
            }
            if repo.dir.is_none() {
                warnings.push(format!(
                    "repo {}: no 'dir' — only needed for sturn polling, fine for \
                     webhook watches",
                    repo.name
                ));
            }
        }
    }

    // --- supersede: per-campaign (mixed policies can't coexist) ------
    let mut any_cancel = false;
    let mut any_no_cancel = false;
    let mut any_fifo = false;
    let mut all_fifo = !config.repos.is_empty();
    for repo in &config.repos {
        if repo.name.is_empty() {
            continue;
        }
        let jobs: Vec<Option<&JobEntry>> = if repo.jobs.is_empty() {
            vec![None]
        } else {
            repo.jobs.iter().map(Some).collect()
        };
        for job in jobs {
            let w = watch(config, repo, job);
            let cancel = w.runner.and_then(|r| r.cancel_running).unwrap_or(false);
            any_cancel |= cancel;
            any_no_cancel |= !cancel;
            all_fifo &= w.queue_all;
            any_fifo |= w.queue_all;
            if w.queue_all && cancel {
                warnings.push(format!(
                    "job '{}': queue = \"all\" conflicts with runner.cancel_running \
                     — bupha rejects this at load; both land in supersede as a \
                     warning here",
                    w.key
                ));
            }
        }
    }
    let mode = if all_fifo { "fifo" } else { "latest-wins" };
    if any_fifo && !all_fifo {
        warnings.push(
            "mixed queue policies — `queue = \"all\"` jobs run under the \
             campaign-wide `mode: latest-wins` (supersede is per-campaign)"
                .into(),
        );
    }
    if any_cancel && any_no_cancel {
        warnings.push(
            "mixed cancel_running across jobs — the campaign supersede is \
             single-valued; emitted cancel_running: true so gates keep \
             bupha's kill-on-supersede for the jobs that had it (others \
             now cancel too — split the file if that matters)"
                .into(),
        );
    }
    let supersede = json!({
        "key": "{repo}:{ref}",
        "mode": mode,
        "cancel_running": any_cancel,
    });

    // --- ingress: github-webhook --------------------------------------
    let ingress_mark = nodes.len();
    let kind = trigger
        .and_then(|t| non_empty(t.kind.as_ref()))
        .unwrap_or("");
    let github_repos: Vec<Value> = config
        .repos
        .iter()
        .filter(|r| !r.name.is_empty())
        .map(|r| {
            json!({
                "name": r.name,
                "secret_env": repo_secret_env(r, trigger),
            })
        })
        .collect();
    let repo_count = github_repos.len();
    match kind {
        "github" => {
            let mut params = Map::new();
            params.insert(
                "listen".into(),
                json!(
                    trigger
                        .and_then(|t| non_empty(t.listen.as_ref()))
                        .unwrap_or(DEFAULT_LISTEN)
                ),
            );
            if let Some(api) = trigger.and_then(|t| non_empty(t.api_base.as_ref())) {
                params.insert("api_base".into(), json!(api));
            }
            params.insert("repos".into(), Value::Array(github_repos));
            nodes.push(json!({
                "id": "ci.ingress.webhook",
                "driver": "github-webhook",
                "on": ["start"],
                "emits": ["ci.ingress.webhook.listening", "push"],
                "params": Value::Object(params),
            }));
        }
        "" => {
            if trigger.and_then(|t| t.sturn.as_ref()).is_none() {
                warnings.push(
                    "trigger.kind is empty and no [trigger.sturn] — emitted \
                     workflow has no ingress; pushes never arrive"
                        .into(),
                );
            } else {
                warnings.push(
                    "trigger.kind is empty — sturn-only ingress (bupha's \
                     single-box mode); no webhook listener emitted"
                        .into(),
                );
            }
        }
        other => warnings.push(format!(
            "trigger.kind '{other}' has no peregrin ingress — only 'github' \
             (+sturn) maps; no listener node emitted"
        )),
    }

    // --- ingress: sturn-poll (per sturn:true repo) ---------------------
    let sturn_cfg = trigger.and_then(|t| t.sturn.as_ref());
    for repo in config
        .repos
        .iter()
        .filter(|r| r.sturn && !r.name.is_empty())
    {
        let Some(dir) = non_empty(repo.dir.as_ref()) else {
            warnings.push(format!(
                "repo {}: sturn = true needs a 'dir' to poll — skipped the \
                 sturn-poll node",
                repo.name
            ));
            continue;
        };
        let mut params = Map::new();
        params.insert("repo".into(), json!(expand_tilde(dir)));
        if let Some(sturn) = sturn_cfg {
            if let Some(remote) = non_empty(sturn.remote.as_ref()) {
                params.insert("remote".into(), json!(remote));
            }
            if let Some(poll_ms) = sturn.poll_ms {
                params.insert("poll_ms".into(), json!(poll_ms));
            }
        }
        nodes.push(json!({
            "id": format!("ci.ingress.sturn.{}", slug(&repo.name)),
            "driver": "sturn-poll",
            "on": ["start"],
            "emits": [format!("ci.ingress.sturn.{}.watching", slug(&repo.name)), "push", "wal.update"],
            "params": Value::Object(params),
        }));
    }
    if sturn_cfg.is_some() && !config.repos.iter().any(|r| r.sturn) {
        warnings.push(
            "[trigger.sturn] is configured but no repo sets `sturn = true` — \
             no poller node emitted"
                .into(),
        );
    }

    // --- the router ---------------------------------------------------
    if nodes.len() > ingress_mark {
        nodes.push(json!({
            "id": "ci.route",
            "driver": "shell",
            "on": ["push"],
            "max_fires": "unlimited",
            "emits": ["push.**", "route.unmatched"],
            "params": {
                "cmd": "true",
                "emit": {
                    "0": "push.${event.payload.repo}.${event.payload.ref}.${event.payload.clone_url}.${event.payload.sha}",
                    "default": "route.unmatched",
                },
            },
        }));
    }

    // --- one gate pipeline per job -------------------------------------
    // bupha's `worktree.base` is *its* runtime dir — importing it
    // verbatim makes peregrin share (and `bupha destroy worktrees`
    // delete) bupha's trees. driver-git's own default is the right
    // base; an explicit bupha base is dropped with a warning.
    if config
        .worktree
        .as_ref()
        .and_then(|w| non_empty(w.base.as_ref()))
        .is_some()
    {
        warnings.push(
            "worktree.base dropped — bupha's runtime dir isn't portable; \
             driver-git uses ~/.local/share/peregrin/worktrees"
                .to_string(),
        );
    }
    let worktree_base = DEFAULT_WORKTREE_BASE;
    let api_base = trigger.and_then(|t| non_empty(t.api_base.as_ref()));
    let status_kind = config
        .status
        .as_ref()
        .and_then(|s| non_empty(s.kind.as_ref()))
        .unwrap_or("github");
    let emit_sinks = match status_kind {
        "github" => true,
        "log" | "none" => {
            warnings.push(format!(
                "status.kind '{status_kind}' — sink nodes dropped; verdicts stay \
                 in the event log"
            ));
            false
        }
        other => {
            warnings.push(format!(
                "status.kind '{other}' unknown — emitting github-status sinks \
                 anyway (bupha would reject it)"
            ));
            true
        }
    };
    let mut jobs = 0;
    for repo in &config.repos {
        if repo.name.is_empty() {
            continue;
        }
        if let Some(dir) = non_empty(repo.dir.as_ref()) {
            let expanded = expand_tilde(dir);
            let dir_base = Path::new(&expanded)
                .file_name()
                .map(|f| f.to_string_lossy().into_owned())
                .unwrap_or_default();
            let repo_base = repo.name.rsplit('/').next().unwrap_or(&repo.name);
            if dir_base != repo_base {
                warnings.push(format!(
                    "repo {}: dir '{dir}' dropped — driver-git clones from the \
                     event's clone_url (basename '{dir_base}' ≠ '{repo_base}' is \
                     the meaningful difference)",
                    repo.name
                ));
            }
        }
        let job_list: Vec<Option<&JobEntry>> = if repo.jobs.is_empty() {
            vec![None]
        } else {
            repo.jobs.iter().map(Some).collect()
        };
        for job in job_list {
            let w = watch(config, repo, job);
            jobs += 1;
            let on = on_patterns(&w, &mut warnings, sturn_cfg);
            let mut pipeline = job_nodes(&w, worktree_base, api_base, &mut warnings, &on);
            if !emit_sinks {
                pipeline.retain(|n| n["driver"].as_str() == Some("git"));
            }
            nodes.extend(pipeline);
        }
    }

    // --- failmap: materialization node.failed → <node>.failed.<sha> ----
    nodes.push(json!({
        "id": "ci.failmap",
        "driver": "shell",
        "on": ["node.failed"],
        "max_fires": "unlimited",
        "emits": ["ci.unmapped"],
        "params": {
            "cmd": "test -n \"${event.payload.node}\" && test -n \"${event.payload.sha}\"",
            "emit": {
                "0": "${event.payload.node}.failed.${event.payload.sha}",
                "default": "ci.unmapped",
            },
        },
    }));

    // --- dropped sections ------------------------------------------------
    if config.tunnel.is_some() {
        warnings.push(
            "[tunnel] dropped — cloudflared stays external (ADR-0080 service \
             scoping); expose the listener's `listen` address yourself"
                .into(),
        );
    }
    if let Some(status) = &config.status
        && let Some(repo) = non_empty(status.repo.as_ref())
    {
        warnings.push(format!(
            "status.repo '{repo}' dropped — per-repo statuses post to each \
             entry's name/status_repo (bupha's own multi-repo rule)"
        ));
    }
    for section in config.extra.keys() {
        warnings.push(format!(
            "[{section}] dropped — no peregrin equivalent yet (remote drivers \
             cover worker/agent; the rest are open substrate items)"
        ));
    }
    for repo in &config.repos {
        if !repo.runs_on.is_empty()
            || repo
                .jobs
                .iter()
                .any(|j| j.runs_on.as_ref().is_some_and(|r| !r.is_empty()))
        {
            warnings.push(format!(
                "repo {}: runs_on dropped — fleet labels are a remote-driver \
                 concern (the gate runs in-process)",
                repo.name
            ));
        }
        if repo.retry > 0 || repo.jobs.iter().any(|j| j.retry.is_some_and(|n| n > 0)) {
            warnings.push(format!(
                "repo {}: retry dropped — wire a resilience fragment \
                 (libraries/retry) on the job nodes",
                repo.name
            ));
        }
        for key in repo.extra.keys() {
            warnings.push(format!(
                "repo {}: key '{key}' unmapped — dropped",
                repo.name
            ));
        }
        for job in &repo.jobs {
            for key in job.extra.keys() {
                warnings.push(format!(
                    "job '{}:{}': key '{key}' unmapped — dropped",
                    repo.name, job.name
                ));
            }
        }
    }

    // `capabilities.secrets` — the webhook/status drivers deny any
    // `secret_env`/`token_env` name the grant doesn't declare, so the
    // emitted workflow must list every name it references or ingress
    // registers zero routes.
    let mut secret_names = BTreeSet::new();
    for node in &nodes {
        if let Some(params) = node.get("params") {
            collect_secrets(params, &mut secret_names);
        }
    }
    let mut workflow = json!({
        "name": "ci-gate",
        "annotations": {
            "generated_by": "peregrin import bupha",
            "source": path.display().to_string(),
        },
        "supersede": supersede,
        "nodes": Value::Array(nodes.clone()),
    });
    if !secret_names.is_empty() {
        workflow["capabilities"] = json!({
            "secrets": secret_names.iter().cloned().collect::<Vec<_>>(),
        });
    }
    (workflow, warnings, repo_count, jobs)
}

/// `peregrin import bupha <path>` — convert and print (or `--out`)
/// the emitted node-config workflow. Returns the report for tests.
pub fn import_bupha(path: &Path, out: Option<&Path>) -> Result<ImportReport, CliError> {
    let config = load_bupha(path)?;
    let (workflow, warnings, repos, jobs) = convert(path, &config);
    let yaml =
        serde_yaml_ng::to_string(&workflow).map_err(|e| CliError(format!("emit workflow: {e}")))?;
    let report = ImportReport {
        nodes: workflow["nodes"].as_array().map_or(0, Vec::len),
        yaml,
        warnings,
        repos,
        jobs,
    };
    for warning in &report.warnings {
        eprintln!("warning: {warning}");
    }
    if let Some(out) = out {
        std::fs::write(out, &report.yaml)
            .map_err(|e| CliError(format!("write {}: {e}", out.display())))?;
        println!(
            "wrote {} — {} repos, {} jobs, {} nodes, {} warnings",
            out.display(),
            report.repos,
            report.jobs,
            report.nodes,
            report.warnings.len()
        );
    } else {
        print!("{}", report.yaml);
        println!(
            "# {} repos, {} jobs, {} nodes, {} warnings",
            report.repos,
            report.jobs,
            report.nodes,
            report.warnings.len()
        );
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    /// The full mapping on one repo — single job.
    fn config(yaml: &str) -> BuphaConfig {
        let v: serde_norway::Value = serde_norway::from_str(yaml).unwrap();
        serde_norway::from_value(v).unwrap()
    }

    #[test]
    fn watch_inherits_job_repo_global() {
        let cfg = config(
            r#"
repos:
  - name: a/b
    dir: ~/x/b
    branch: refs/heads/dev
    queue: "all"
    runner: {kind: script, cmd: repo.sh, cancel_running: true}
    jobs:
      - name: docs
        runner: {kind: script, cmd: job.sh}
runner: {kind: dagger, token_env: TOK, cancel_running: true}
status: {context: ci/base}
"#,
        );
        let repo = &cfg.repos[0];
        // Named job: job runner wins, branch falls to repo, context
        // composes <base>/<job>.
        let w = watch(&cfg, repo, Some(&repo.jobs[0]));
        assert_eq!(w.key, "a/b:docs");
        assert_eq!(w.branch, "refs/heads/dev");
        assert_eq!(w.context, "ci/base/docs");
        assert_eq!(
            w.runner.unwrap().extra["cmd"],
            Value::String("job.sh".into())
        );
        assert!(w.queue_all);
        // Implicit: repo runner + repo branch + bare context.
        let bare = RepoEntry {
            name: "c/d".into(),
            ..RepoEntry::default()
        };
        let cfg2 = BuphaConfig {
            runner: Some(RunnerCfg {
                kind: Some("script".into()),
                token_env: Some("TOK".into()),
                cancel_running: Some(true),
                extra: Map::new(),
            }),
            ..BuphaConfig::default()
        };
        let w2 = watch(&cfg2, &bare, None);
        assert_eq!(w2.key, "c/d");
        assert_eq!(w2.branch, DEFAULT_BRANCH);
        assert_eq!(w2.context, DEFAULT_CONTEXT);
        assert!(!w2.queue_all);
        // Job context override wins over composition.
        let j = JobEntry {
            name: "x".into(),
            context: Some("custom/ctx".into()),
            ..JobEntry::default()
        };
        assert_eq!(job_context(cfg.status.as_ref(), Some(&j)), "custom/ctx");
    }

    #[test]
    fn secret_env_fallback_chain() {
        let trig = TriggerCfg {
            secret_env: Some("TRIG".into()),
            ..TriggerCfg::default()
        };
        let repo = |secret_env: Option<String>| RepoEntry {
            name: "a/b".into(),
            secret_env,
            ..RepoEntry::default()
        };
        assert_eq!(repo_secret_env(&repo(Some("R".into())), Some(&trig)), "R");
        assert_eq!(repo_secret_env(&repo(None), Some(&trig)), "TRIG");
        assert_eq!(repo_secret_env(&repo(None), None), DEFAULT_SECRET_ENV);
    }

    #[test]
    fn slugs_and_tilde() {
        assert_eq!(slug("9aia/myco:docs"), "9aia-myco-docs");
        assert_eq!(slug("/odd//"), "odd");
        assert!(expand_tilde("~/x").ends_with("/x"));
        assert_eq!(expand_tilde("/abs"), "/abs");
        assert_ne!(home(), "");
    }

    #[test]
    fn on_patterns_sturn_identities() {
        let cfg = BuphaConfig::default();
        let repo = RepoEntry {
            name: "a/b".into(),
            dir: Some("/srv/b".into()),
            sturn: true,
            ..RepoEntry::default()
        };
        let w = watch(&cfg, &repo, None);
        let mut wn = Vec::new();
        // remote: URL-shaped → three patterns; name → warning.
        let url_sturn = SturnTriggerCfg {
            remote: Some("git@h:a/b.git".into()),
            poll_ms: None,
        };
        let pats = on_patterns(&w, &mut wn, Some(&url_sturn));
        assert_eq!(pats.len(), 3);
        assert_eq!(pats[0].as_str().unwrap(), "push.a/b.refs/heads/main.**.*");
        assert_eq!(
            pats[1].as_str().unwrap(),
            "push./srv/b.refs/heads/main.**.*"
        );
        assert_eq!(
            pats[2].as_str().unwrap(),
            "push.git@h:a/b.git.refs/heads/main.**.*"
        );
        let named = SturnTriggerCfg {
            remote: Some("origin".into()),
            poll_ms: None,
        };
        let mut wn2 = Vec::new();
        assert_eq!(on_patterns(&w, &mut wn2, Some(&named)).len(), 2);
        assert_eq!(wn2.len(), 1);
        assert!(wn2[0].contains("origin"));
        // No sturn → the name pattern alone.
        let plain = RepoEntry {
            name: "a/b".into(),
            ..RepoEntry::default()
        };
        let w2 = watch(&cfg, &plain, None);
        let mut wn3 = Vec::new();
        assert_eq!(on_patterns(&w2, &mut wn3, None).len(), 1);
    }

    #[test]
    fn convert_full_shape() {
        let cfg = config(
            r#"
repos:
  - name: a/one
    secret_env: SEC_ONE
    dir: ~/GitHub/a/one
    runner: {kind: script, cmd: "bash ci.sh"}
  - name: a/two
    dir: ~/GitHub/a/two
    status_repo: x/status
    jobs:
      - {name: main, branch: refs/heads/main, runner: {kind: script, cmd: "ci.sh", cancel_running: true}}
      - {name: docs, branch: refs/heads/docs}
trigger: {kind: github, listen: "0.0.0.0:9", secret_env: FALLBACK, api_base: "https://ghe/api/v3"}
runner: {kind: dagger, function: ci, token_env: BUPHA_TOKEN, cancel_running: true, sccache_conf: ~/sccache}
status: {kind: github, context: ci/bupha, repo: legacy/repo}
worktree: {base: ~/wt}
tunnel: {provider: cloudflared}
cache: {conf: x}
"#,
        );
        let (wf, warnings, repos, jobs) = convert(Path::new("bupha.yaml"), &cfg);
        assert_eq!(repos, 2);
        assert_eq!(jobs, 3);
        let nodes = wf["nodes"].as_array().unwrap();
        // webhook + router + failmap + 3 pipelines × 3 nodes.
        assert_eq!(nodes.len(), 3 + 9);
        let webhook = &nodes[0];
        assert_eq!(webhook["id"], "ci.ingress.webhook");
        assert_eq!(webhook["params"]["listen"], "0.0.0.0:9");
        assert_eq!(webhook["params"]["api_base"], "https://ghe/api/v3");
        let repos_list = webhook["params"]["repos"].as_array().unwrap();
        assert_eq!(repos_list[0]["secret_env"], "SEC_ONE");
        assert_eq!(repos_list[1]["secret_env"], "FALLBACK");
        assert_eq!(wf["supersede"]["key"], "{repo}:{ref}");
        assert_eq!(wf["supersede"]["mode"], "latest-wins");
        assert_eq!(wf["supersede"]["cancel_running"], true);
        // Job node wiring.
        let git1 = nodes.iter().find(|n| n["id"] == "ci.a-one").unwrap();
        assert_eq!(git1["driver"], "git");
        assert_eq!(git1["on"][0], "push.a/one.refs/heads/main.**.*");
        assert_eq!(git1["params"]["url"], "${event.match.1}");
        assert_eq!(git1["params"]["sha"], "${event.match.2}");
        // bupha's worktree.base is its runtime dir — dropped for the
        // driver-git default, with a warning.
        assert_eq!(git1["params"]["base"], "~/.local/share/peregrin/worktrees");
        assert_eq!(git1["params"]["token_env"], "BUPHA_TOKEN");
        assert_eq!(git1["params"]["inner"]["params"]["cmd"], "bash ci.sh");
        assert_eq!(git1["params"]["inner"]["params"]["timeout"], 600);
        assert_eq!(git1["max_fires"], "unlimited");
        // Sinks: status_repo + context composition.
        let st = nodes
            .iter()
            .find(|n| n["id"] == "ci.a-two-main.status")
            .unwrap();
        assert_eq!(st["params"]["status_repo"], "x/status");
        assert_eq!(st["params"]["context"], "ci/bupha/main");
        assert_eq!(st["params"]["api_base"], "https://ghe/api/v3");
        assert_eq!(st["on"][2], "ci.a-two-main.failed.*");
        let pend = nodes
            .iter()
            .find(|n| n["id"] == "ci.a-two-docs.pending")
            .unwrap();
        assert_eq!(pend["params"]["state"], "pending");
        assert_eq!(pend["params"]["context"], "ci/bupha/docs");
        assert_eq!(pend["on"][0], "push.a/two.refs/heads/docs.**.*");
        // Warnings: tunnel, legacy status.repo, dropped cache section,
        // dagger kind + sccache on the docs job (falls to global runner),
        // queue/conflict notes are absent here (docs job: cancel=false,
        // others true → the mixed-cancel warning lands).
        assert!(warnings.iter().any(|w| w.contains("[tunnel] dropped")));
        assert!(
            warnings
                .iter()
                .any(|w| w.contains("status.repo 'legacy/repo'"))
        );
        assert!(warnings.iter().any(|w| w.contains("[cache] dropped")));
        assert!(
            warnings
                .iter()
                .any(|w| w.contains("kind 'dagger'") && w.contains("a/two:docs"))
        );
        assert!(warnings.iter().any(|w| w.contains("sccache_conf")));
        assert!(warnings.iter().any(|w| w.contains("mixed cancel_running")));
        assert!(warnings.iter().any(|w| !w.contains("no 'dir'")));
        // The dagger job keeps its on: entry with a stub cmd.
        let docs = nodes.iter().find(|n| n["id"] == "ci.a-two-docs").unwrap();
        assert!(
            docs["params"]["inner"]["params"]["cmd"]
                .as_str()
                .unwrap()
                .contains("exit 127")
        );
    }

    #[test]
    fn convert_edge_shapes() {
        // Empty repos, empty trigger kind + no sturn, status kind log.
        let cfg = config("repos: []\n");
        let (_wf, warnings, repos, jobs) = convert(Path::new("b.yaml"), &cfg);
        assert_eq!((repos, jobs), (0, 0));
        assert!(warnings.iter().any(|w| w.contains("no [[repos]]")));
        assert!(warnings.iter().any(|w| w.contains("no ingress")));

        // queue=all across the fleet → fifo; cancel conflict arm.
        let cfg2 = config(
            r#"
repos:
  - {name: a/b, dir: /x, queue: "all", runner: {kind: script, cmd: c, cancel_running: true}}
"#,
        );
        let (wf2, warnings2, ..) = convert(Path::new("b.yaml"), &cfg2);
        assert_eq!(wf2["supersede"]["mode"], "fifo");
        assert!(warnings2.iter().any(|w| w.contains("conflicts")));

        // script runner without cmd → stub + warning; missing dir note.
        let cfg3 = config(
            r"
repos:
  - {name: a/b, runner: {kind: script}}
status: {kind: log}
",
        );
        let (wf3, warnings3, ..) = convert(Path::new("b.yaml"), &cfg3);
        assert!(warnings3.iter().any(|w| w.contains("no cmd")));
        assert!(warnings3.iter().any(|w| w.contains("no 'dir'")));
        assert!(warnings3.iter().any(|w| w.contains("status.kind 'log'")));
        // status kind log drops the sinks but keeps the git node.
        let kinds: Vec<&str> = wf3["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|n| n["driver"].as_str())
            .collect();
        assert!(kinds.contains(&"git"));
        assert!(!kinds.contains(&"github-status"));

        // Unknown status kind → sinks emitted + warning.
        let cfg4 = config(
            "repos: [{name: a/b, dir: /x, runner: {kind: script, cmd: c}}]\nstatus: {kind: weird}\n",
        );
        let (wf4, warnings4, ..) = convert(Path::new("b.yaml"), &cfg4);
        assert!(warnings4.iter().any(|w| w.contains("status.kind 'weird'")));
        assert!(
            wf4["nodes"]
                .as_array()
                .unwrap()
                .iter()
                .any(|n| n["driver"] == "github-status")
        );

        // Unknown trigger kind → warning, no webhook node.
        let cfg5 = config(
            "repos: [{name: a/b, dir: /x, runner: {kind: script, cmd: c}}]\ntrigger: {kind: gitlab}\n",
        );
        let (wf5, warnings5, ..) = convert(Path::new("b.yaml"), &cfg5);
        assert!(warnings5.iter().any(|w| w.contains("'gitlab'")));
        assert!(
            !wf5["nodes"]
                .as_array()
                .unwrap()
                .iter()
                .any(|n| n["driver"] == "github-webhook")
        );

        // trigger.sturn configured, no opted repos → note; sturn repo
        // without dir → skip note.
        let cfg6 = config(
            "repos: [{name: a/b, sturn: true, runner: {kind: script, cmd: c}}]\ntrigger: {sturn: {poll_ms: 50, remote: origin}}\n",
        );
        let (_wf6, warnings6, ..) = convert(Path::new("b.yaml"), &cfg6);
        assert!(warnings6.iter().any(|w| w.contains("needs a 'dir'")));

        let cfg7 = config(
            "repos: [{name: a/b, dir: /x, runner: {kind: script, cmd: c}}]\ntrigger: {sturn: {poll_ms: 50}}\n",
        );
        let (_wf7, warnings7, ..) = convert(Path::new("b.yaml"), &cfg7);
        assert!(
            warnings7
                .iter()
                .any(|w| w.contains("no repo sets `sturn = true`"))
        );

        // unnamed repo + duplicate name + runs_on + retry + unknown key.
        let cfg8 = config(
            "repos:\n  - {name: '', dir: /x}\n  - {name: a/b, dir: /x, runs_on: [arm], retry: 2, bogus: 1, runner: {kind: script, cmd: c}}\n  - {name: a/b, dir: /y}\n",
        );
        let (_wf8, warnings8, ..) = convert(Path::new("b.yaml"), &cfg8);
        assert!(warnings8.iter().any(|w| w.contains("missing 'name'")));
        assert!(warnings8.iter().any(|w| w.contains("more than once")));
        assert!(warnings8.iter().any(|w| w.contains("runs_on dropped")));
        assert!(warnings8.iter().any(|w| w.contains("retry dropped")));
        assert!(warnings8.iter().any(|w| w.contains("'bogus' unmapped")));

        // sturn remote-name note via the route patterns + sturn node
        // params poll through.
        let cfg9 = config(
            "repos: [{name: a/b, dir: /x, sturn: true, runner: {kind: script, cmd: c}}]\ntrigger: {sturn: {poll_ms: 50, remote: \"https://h/a/b.git\"}}\n",
        );
        let (wf9, ..) = convert(Path::new("b.yaml"), &cfg9);
        let sturn_node = wf9["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|n| n["driver"] == "sturn-poll")
            .unwrap();
        assert_eq!(sturn_node["params"]["repo"], "/x");
        assert_eq!(sturn_node["params"]["poll_ms"], 50);
        assert_eq!(sturn_node["params"]["remote"], "https://h/a/b.git");

        // Empty trigger kind + sturn present → sturn-only note.
        let cfg10 = config(
            "repos: [{name: a/b, dir: /x, runner: {kind: script, cmd: c}}]\ntrigger: {kind: '', sturn: {}}\n",
        );
        let (_wf10, warnings10, ..) = convert(Path::new("b.yaml"), &cfg10);
        assert!(warnings10.iter().any(|w| w.contains("sturn-only")));
    }

    #[test]
    fn load_bupha_errors_and_toml() {
        let tmp = tempfile::TempDir::new().unwrap();
        assert!(load_bupha(&tmp.path().join("missing.yaml")).is_err());
        let bad = tmp.path().join("bad.yaml");
        std::fs::write(&bad, "{").unwrap();
        assert!(load_bupha(&bad).is_err());
        let scalar = tmp.path().join("s.yaml");
        std::fs::write(&scalar, "42").unwrap();
        assert!(load_bupha(&scalar).is_err());
        let bad_toml = tmp.path().join("bad.toml");
        std::fs::write(&bad_toml, "[").unwrap();
        assert!(load_bupha(&bad_toml).is_err());
        // TOML dispatch on extension + the [[repos]]/[[repo]] aliases
        // (serde rejects both spellings in one file — as does bupha).
        let toml_f = tmp.path().join("bupha.toml");
        std::fs::write(
            &toml_f,
            "[[repos]]\nname = \"a/b\"\ndir = \"/x\"\n[repos.runner]\nkind = \"script\"\ncmd = \"c\"\n[[repos]]\nname = \"c/d\"\ndir = \"/y\"\n[trigger]\nkind = \"github\"\n",
        )
        .unwrap();
        let cfg = load_bupha(&toml_f).unwrap();
        assert_eq!(cfg.repos.len(), 2);
        assert_eq!(cfg.repos[0].name, "a/b");
        assert_eq!(cfg.repos[1].name, "c/d");
        let toml_alias = tmp.path().join("alias.toml");
        std::fs::write(&toml_alias, "[[repo]]\nname = \"e/f\"\n").unwrap();
        assert_eq!(load_bupha(&toml_alias).unwrap().repos[0].name, "e/f");

        // import_bupha end-to-end: stdout path + --out path + summary.
        let report = import_bupha(&toml_f, None).unwrap();
        assert_eq!(report.repos, 2);
        assert!(report.yaml.contains("driver: github-webhook"));
        assert!(report.yaml.contains("ci.a-b"));
        let out = tmp.path().join("out.yaml");
        let report2 = import_bupha(&toml_f, Some(&out)).unwrap();
        assert_eq!(std::fs::read_to_string(&out).unwrap(), report2.yaml);
        assert!(import_bupha(&tmp.path().join("missing.yaml"), None).is_err());
    }
}
