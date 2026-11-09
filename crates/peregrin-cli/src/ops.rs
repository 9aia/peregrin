//! The P6 ops verbs — provision/teardown plus the observability
//! surface (ADR-0079's "provision/teardown verbs — bupha parity"):
//!
//! `create --template <bundle>` resolves an installed bundle
//! (`tamias_store_local::Store`), materializes a node config (`include:`
//! the bundle's `main.yaml` + operator `config:` params + an inline
//! `github-webhook` ingress node per `--repo`), and writes a per-user
//! systemd unit via `hypha-systemd` — *without* enabling it
//! (`peregrin service enable --unit <name>` is the operator's step).
//! `deploy webhook` / `destroy webhook` register/remove each watched
//! repo's push hook via the GitHub REST API, mirroring `bupha deploy
//! webhook` (upsert by exact hook-URL match; token + per-repo secrets
//! resolve by NAME through the env → keyring → env-file chain —
//! values never sit in config). `destroy service` / `destroy
//! worktrees` disable + remove the unit and run
//! `peregrin_git::Worktrees::cleanup` with an orphan report.
//! `runs` / `explain` / `log` read the durable JSONL
//! event log (`peregrin-store-json`'s format — the same file
//! `campaign run --history` appends to) plus the node's own health
//! checks.
//!
//! Verbs print report lines like `distribution`/`import` do and
//! return them (or a report struct) so tests assert without scraping
//! stdout.

use std::collections::HashSet;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::{Map, Value, json};

use hypha_dotenv::DotenvSecrets;
use hypha_keyring::KeyringSecrets;
use hypha_secret::Secrets;
use peregrin_config::ConfigSource;
use peregrin_core::Campaign;
use peregrin_git::{Listing, Worktrees};
use tamias_store_local::Store;

use crate::CliError;

// --- environment ---------------------------------------------------------------

/// Node-config file names tried, in order — YAML first, TOML last
/// (the `peregrin-config-file` search convention).
const CONFIG_NAMES: [&str; 3] = ["peregrin.yaml", "peregrin.yml", "peregrin.toml"];

/// The webhook node's default hook path (`params.path`).
const DEFAULT_HOOK_PATH: &str = "/hook";
/// The default forge REST endpoint (`params.api_base`).
const DEFAULT_API_BASE: &str = "https://api.github.com";
/// The default forge-token secret NAME (`config.token_env`).
const DEFAULT_TOKEN_ENV: &str = "GITHUB_TOKEN";
/// The default worktree base (`config.worktree_base`).
const DEFAULT_WORKTREE_BASE: &str = "~/.local/share/peregrin/worktrees";

/// Store/config/unit/log roots — [`OpsEnv::default`] derives them
/// from `PEREGRIN_HOME`/XDG/`HOME`; tests inject tempdirs via
/// [`OpsEnv::under`].
#[derive(Clone, Debug)]
pub struct OpsEnv {
    /// `<peregrin-home>/store` — the installed-bundle store.
    pub store_root: PathBuf,
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
            store_root: peregrin_node::peregrin_home().join("store"),
            config_dir: config_dir(),
            unit_dir: hypha_systemd::default_unit_dir(),
            home: peregrin_node::peregrin_home(),
        }
    }
}

/// `XDG_CONFIG_HOME/peregrin` → `~/.config/peregrin` (the
/// `peregrin-config-file` search dir).
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

/// The secret chain — environment → OS keyring → the config-dir env
/// file. Verbs read secret *names* from config; values only ever
/// resolve through here (the per-impl binding rule).
fn secret(env: &OpsEnv, name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .filter(|v| !v.is_empty())
        .or_else(|| KeyringSecrets::default().get(name))
        .or_else(|| {
            DotenvSecrets {
                path: env.env_file(),
            }
            .get(name)
        })
}

/// The unresolved-secret error — names the file the value could land in.
fn missing_secret(env: &OpsEnv, name: &str) -> CliError {
    CliError(format!(
        "secret '{name}' unresolved — set it in the environment, the OS \
         keyring (service 'peregrin') or {}",
        env.env_file().display()
    ))
}

/// Seed `env_file` with the secret names (empty values — never
/// invented). Written `0o600`; an existing file is never clobbered —
/// returns `Some(path)` only when it actually wrote.
fn write_env_skeleton(env: &OpsEnv, names: &[String]) -> Result<Option<PathBuf>, CliError> {
    let path = env.env_file();
    if path.exists() {
        return Ok(None);
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)
            .map_err(|e| CliError(format!("create {}: {e}", dir.display())))?;
    }
    let mut body = "# peregrin secrets — NAME=value per line. These are the names the\n\
         # materialized config references; values resolve env > keyring > this file.\n"
        .to_string();
    for name in names {
        body.push_str(name);
        body.push_str("=\n");
    }
    let mut opts = std::fs::OpenOptions::new();
    opts.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    opts.open(&path)
        .and_then(|mut f| f.write_all(body.as_bytes()))
        .map_err(|e| CliError(format!("write {}: {e}", path.display())))?;
    Ok(Some(path))
}

// --- node config -------------------------------------------------------------------

/// Resolve the node-config path: `--config`, else `./peregrin.*`,
/// else `~/.config/peregrin/peregrin.*` (the `peregrin-config-file`
/// search).
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
    for name in CONFIG_NAMES {
        let local = PathBuf::from(name);
        if local.exists() {
            return Ok(local);
        }
        let candidate = env.config_dir.join(name);
        if candidate.exists() {
            return Ok(candidate);
        }
    }
    Err(CliError(format!(
        "no peregrin.{{yaml,yml,toml}} found (search: ./, {}) — pass --config",
        env.config_dir.display()
    )))
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

/// A `params.<key>` literal — templated `${…}` values can't resolve
/// without a run's scope, so ops verbs reject them.
fn literal<'a>(params: &'a Map<String, Value>, key: &str) -> Option<&'a str> {
    params
        .get(key)
        .and_then(Value::as_str)
        .filter(|v| !v.is_empty() && !v.contains("${"))
}

/// One watched repo from a `github-webhook` node's `params.repos[]`.
#[derive(Clone, Debug)]
pub struct RepoWatch {
    /// `owner/name` — the repo `full_name`.
    pub name: String,
    /// The NAME of the repo's webhook secret.
    pub secret_env: String,
    /// The node's hook path (`params.path`, default `/hook`).
    pub hook_path: String,
    /// The node's forge API base (`params.api_base`).
    pub api_base: String,
    /// The ingress node id this entry came from.
    pub node: String,
}

/// Collect `params.repos` across the campaign's `github-webhook`
/// nodes, deduped by repo name (first wins — like bupha's one hook
/// per repo).
fn watched_repos(campaign: &Campaign) -> Result<Vec<RepoWatch>, CliError> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for node in &campaign.nodes {
        if node.driver != "github-webhook" {
            continue;
        }
        let params = node.params.as_object().cloned().unwrap_or_default();
        let hook_path = literal(&params, "path").unwrap_or(DEFAULT_HOOK_PATH);
        let api_base = literal(&params, "api_base").unwrap_or(DEFAULT_API_BASE);
        let Some(repos) = params
            .get("repos")
            .and_then(Value::as_array)
            .filter(|repos| !repos.is_empty())
        else {
            return Err(CliError(format!(
                "github-webhook node '{}': missing 'params.repos' — a non-empty \
                 list of {{name, secret_env}}",
                node.id
            )));
        };
        for entry in repos {
            let err =
                |reason: &str| CliError(format!("github-webhook node '{}': {reason}", node.id));
            let entry_obj = entry.as_object().cloned().unwrap_or_default();
            let Some(name) = literal(&entry_obj, "name") else {
                return Err(err(
                    "'params.repos[]' entries need a literal non-empty 'name'",
                ));
            };
            let Some(secret_env) = literal(&entry_obj, "secret_env") else {
                return Err(err(&format!(
                    "repo '{name}' missing 'secret_env' — the secret's literal NAME"
                )));
            };
            if seen.insert(name.to_string()) {
                out.push(RepoWatch {
                    name: name.to_string(),
                    secret_env: secret_env.to_string(),
                    hook_path: hook_path.to_string(),
                    api_base: api_base.to_string(),
                    node: node.id.clone(),
                });
            }
        }
    }
    Ok(out)
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

/// The public URL the hooks point at — `--url`, else
/// `config.public_url`.
fn public_url(campaign: &Campaign, url: Option<&str>) -> Result<String, CliError> {
    url.map(str::to_string)
        .or_else(|| config_str(campaign, "public_url"))
        .map(|u| u.trim_end_matches('/').to_string())
        .filter(|u| !u.is_empty())
        .ok_or_else(|| {
            CliError(
                "no public URL — pass --url or set `config.public_url` in the \
                 node config (the hook lands at <url><hook-path>)"
                    .into(),
            )
        })
}

/// The forge-token secret NAME — `--token-env`, else
/// `config.token_env`, else `GITHUB_TOKEN`.
fn token_name(campaign: &Campaign, token_env: Option<&str>) -> String {
    token_env
        .map(str::to_string)
        .or_else(|| config_str(campaign, "token_env"))
        .unwrap_or_else(|| DEFAULT_TOKEN_ENV.to_string())
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

/// Register or update the repo's push webhook pointing at `hook_url`
/// (`POST`/`PATCH /repos/{repo}/hooks`) — the `bupha deploy webhook`
/// upsert: only a hook already pointing at this exact URL is taken
/// over. Returns the human message.
pub fn register_hook(
    token: &str,
    repo: &str,
    hook_url: &str,
    secret: &str,
    api_base: &str,
) -> Result<String, CliError> {
    let agent = gh_agent();
    let existing = find_hook(&agent, api_base, repo, hook_url, token)?;
    let config = json!({"url": hook_url, "content_type": "json", "secret": secret});
    if let Some(id) = existing {
        agent
            .patch(format!("{api_base}/repos/{repo}/hooks/{id}"))
            .header("Authorization", &format!("Bearer {token}"))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "peregrin")
            .send_json(json!({"config": config, "events": ["push"], "active": true}))
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
                "events": ["push"], "active": true}))
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

// --- create --------------------------------------------------------------------------

/// What `create` wrote — returned for tests, printed by the verb.
#[derive(Debug)]
pub struct CreateReport {
    /// The instance name (`--name` or the bundle name).
    pub name: String,
    /// The materialized node config.
    pub config_path: PathBuf,
    /// The written unit file.
    pub unit_path: PathBuf,
    /// The unit name (`<name>.service`).
    pub unit_name: String,
    /// The emitted yaml body.
    pub yaml: String,
    /// Secret NAMES the deployment needs resolved.
    pub secrets: Vec<String>,
}

/// `owner/name[:SECRET_ENV]` → the default webhook-secret NAME —
/// `PEREGRIN_WH_<OWNER>_<REPO>` (uppercased, non-alnum → `_`).
fn default_secret_env(repo: &str) -> String {
    let slug: String = repo
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect();
    format!("PEREGRIN_WH_{slug}")
}

/// `--set KEY=VALUE` — the value parses as a YAML scalar (numbers and
/// bools type correctly); an unparseable value stays a string.
fn parse_set(kv: &str) -> Result<(String, Value), CliError> {
    let Some((key, raw)) = kv.split_once('=') else {
        return Err(CliError(format!("--set '{kv}' — expected KEY=VALUE")));
    };
    if key.trim().is_empty() {
        return Err(CliError(format!("--set '{kv}' — the key is empty")));
    }
    let parsed: serde_norway::Value =
        serde_norway::from_str(raw).unwrap_or(serde_norway::Value::String(raw.into()));
    let value: Value =
        serde_norway::from_value(parsed).map_err(|e| CliError(format!("--set '{kv}': {e}")))?;
    Ok((key.to_string(), value))
}

/// `--repo owner/name[:SECRET_ENV]` → the `params.repos[]` entry.
fn parse_repo(spec: &str) -> Result<Value, CliError> {
    let (repo, secret_env) = match spec.split_once(':') {
        Some((name, env)) if !env.is_empty() => (name, env.to_string()),
        Some((name, _)) => (name, default_secret_env(name)),
        None => (spec, default_secret_env(spec)),
    };
    if repo.is_empty() || !repo.contains('/') {
        return Err(CliError(format!(
            "--repo '{spec}' — expected owner/name[:SECRET_ENV]"
        )));
    }
    Ok(json!({"name": repo, "secret_env": secret_env}))
}

/// `create --template <bundle>` — resolve the installed bundle,
/// materialize `<name>.yaml` (include the bundle's `main.yaml` +
/// merged operator params + the inline `github-webhook` ingress node
/// from `--repo`) and write `<name>.service` via `hypha-systemd`.
/// The unit is written, never enabled — the operator runs `peregrin
/// service enable --unit <name>` themselves.
pub fn create(
    env: &OpsEnv,
    template: &str,
    name: Option<&str>,
    sets: &[String],
    repos: &[String],
    config_out: Option<&Path>,
    unit: Option<&str>,
) -> Result<CreateReport, CliError> {
    let entry = Store::new(&env.store_root)
        .find(template)
        .map_err(|e| CliError(format!("{e} — install it first: `peregrin install <ref>`")))?;
    let workflow = ["main.yaml", "main.yml"]
        .iter()
        .map(|n| entry.dir.join(n))
        .find(|p| p.is_file())
        .ok_or_else(|| {
            CliError(format!(
                "bundle '{template}' ({}) has no main.yaml payload entry",
                entry.label()
            ))
        })?;
    let name = name.map_or_else(|| entry.record.name.clone(), str::to_string);

    // Operator params — `--set` entries merge into `config:` (the
    // bundle's own config block is the defaults, local keys win).
    let mut config = Map::new();
    for kv in sets {
        let (key, value) = parse_set(kv)?;
        config.insert(key, value);
    }
    if !config.contains_key("name") {
        config.insert("name".to_string(), json!(name));
    }

    // `--repo` entries → the inline ingress node (node-level config —
    // never shipped inside bundles). `secrets` collects every NAME the
    // deployment references before `watches` moves into the node.
    let watches: Vec<Value> = repos
        .iter()
        .map(|spec| parse_repo(spec))
        .collect::<Result<_, _>>()?;
    let mut secrets: Vec<String> = watches
        .iter()
        .filter_map(|w| {
            w.get("secret_env")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .collect();
    // The forge token completes the name list before the doc writes —
    // `capabilities.secrets` below grants exactly these names (a
    // bundle never self-grants; the operator's materialization does).
    if let Some(token) = config.get("token_env").and_then(Value::as_str) {
        secrets.push(token.to_string());
    } else if !secrets.is_empty() {
        secrets.push(DEFAULT_TOKEN_ENV.to_string());
    }
    secrets.sort();
    secrets.dedup();

    let mut nodes = Vec::new();
    if !watches.is_empty() {
        nodes.push(json!({
            "id": format!("{name}.ingress"),
            "driver": "github-webhook",
            "on": ["start"],
            "emits": [format!("{name}.ingress.listening"), "push"],
            "params": {"repos": Value::Array(watches)},
        }));
    }

    let mut doc = Map::new();
    doc.insert("name".into(), json!(name));
    doc.insert(
        "annotations".into(),
        json!({
            "generated_by": format!("peregrin create --template {template}"),
            "bundle": entry.label(),
        }),
    );
    doc.insert("include".into(), json!([workflow.to_string_lossy()]));
    doc.insert("config".into(), Value::Object(config.clone()));
    if !secrets.is_empty() {
        doc.insert(
            "capabilities".into(),
            json!({ "secrets": Value::Array(
                secrets.iter().map(|s| Value::String(s.clone())).collect(),
            ) }),
        );
    }
    if !nodes.is_empty() {
        doc.insert("nodes".into(), Value::Array(nodes));
    }
    let yaml = serde_yaml_ng::to_string(&Value::Object(doc))
        .map_err(|e| CliError(format!("emit node config: {e}")))?;

    let config_path = config_out.map_or_else(
        || env.config_dir.join(format!("{name}.yaml")),
        Path::to_path_buf,
    );
    if let Some(dir) = config_path.parent() {
        std::fs::create_dir_all(dir)
            .map_err(|e| CliError(format!("create {}: {e}", dir.display())))?;
    }
    std::fs::write(&config_path, &yaml)
        .map_err(|e| CliError(format!("write {}: {e}", config_path.display())))?;

    // The unit — written but NOT enabled.
    let unit_name = unit.map_or_else(|| name.clone(), str::to_string);
    let bin = std::env::current_exe()
        .and_then(|p| p.canonicalize())
        .map_err(|e| CliError(format!("cannot resolve the peregrin binary path: {e}")))?;
    let history = env.history_path();
    let exec = vec![
        bin.to_string_lossy().into_owned(),
        "campaign".into(),
        "run".into(),
        config_path.to_string_lossy().into_owned(),
        "--history".into(),
        history.to_string_lossy().into_owned(),
    ];
    let unit_path = hypha_systemd::write_unit(
        &env.unit_dir,
        &unit_name,
        &exec,
        &env.env_file(),
        &format!("Peregrin campaign {name}"),
    )
    .map_err(|e| CliError(format!("write unit: {e}")))?;

    // The env skeleton lists the same names `capabilities.secrets`
    // grants — collected above (webhook per-repo + forge token).
    if let Some(env_path) = write_env_skeleton(env, &secrets)? {
        println!("env file: {} (fill the secrets)", env_path.display());
    }

    println!("config: {}", config_path.display());
    println!("unit:   {}", unit_path.display());
    for secret_name in &secrets {
        println!("secret: {secret_name} (name — resolve via env, keyring or the env file)");
    }
    println!(
        "next:   peregrin service enable --unit {unit_name}   # start now + at login\n\
         \x20       peregrin deploy webhook --config {}       # register the repo hooks",
        config_path.display()
    );
    Ok(CreateReport {
        name,
        config_path,
        unit_path,
        unit_name,
        yaml,
        secrets,
    })
}

// --- deploy / destroy ------------------------------------------------------------------

/// `deploy webhook` — register/update the push webhook on each repo a
/// `github-webhook` node watches. One hook per repo, keyed on the
/// exact `<public_url><path>` URL (the bupha upsert).
pub fn deploy_webhook(
    env: &OpsEnv,
    config: Option<&Path>,
    url: Option<&str>,
    token_env: Option<&str>,
) -> Result<Vec<String>, CliError> {
    let path = find_config(env, config)?;
    let campaign = load_config(&path)?;
    let repos = watched_repos(&campaign)?;
    if repos.is_empty() {
        return Err(CliError(format!(
            "{}: no `driver: github-webhook` node with params.repos — nothing \
             registers hooks",
            path.display()
        )));
    }
    let url = public_url(&campaign, url)?;
    let token_env = token_name(&campaign, token_env);
    let token = secret(env, &token_env).ok_or_else(|| missing_secret(env, &token_env))?;
    let mut lines = Vec::with_capacity(repos.len());
    for watch in &repos {
        let hook_url = format!("{}{}", url, watch.hook_path);
        let hook_secret =
            secret(env, &watch.secret_env).ok_or_else(|| missing_secret(env, &watch.secret_env))?;
        let msg = register_hook(
            &token,
            &watch.name,
            &hook_url,
            &hook_secret,
            &watch.api_base,
        )?;
        println!("{msg}");
        lines.push(msg);
    }
    Ok(lines)
}

/// `destroy webhook` — delete the hooks `deploy webhook` registered
/// (exact URL match). `--repo` selects one watched repo, `--all`
/// every one — clap enforces one or the other.
pub fn destroy_webhook(
    env: &OpsEnv,
    config: Option<&Path>,
    url: Option<&str>,
    token_env: Option<&str>,
    repo: Option<&str>,
    all: bool,
) -> Result<Vec<String>, CliError> {
    let path = find_config(env, config)?;
    let campaign = load_config(&path)?;
    let repos = watched_repos(&campaign)?;
    let targets: Vec<&RepoWatch> = if let Some(name) = repo {
        let matching: Vec<_> = repos.iter().filter(|r| r.name == name).collect();
        if matching.is_empty() {
            return Err(CliError(format!(
                "{name}: not a watched repo in {}",
                path.display()
            )));
        }
        matching
    } else {
        if !all {
            return Err(CliError(
                "destroy webhook needs --repo <owner/name> or --all".into(),
            ));
        }
        repos.iter().collect()
    };
    let url = public_url(&campaign, url)?;
    let token_env = token_name(&campaign, token_env);
    let token = secret(env, &token_env).ok_or_else(|| missing_secret(env, &token_env))?;
    let mut lines = Vec::with_capacity(targets.len());
    for watch in targets {
        let hook_url = format!("{}{}", url, watch.hook_path);
        let msg = remove_hook(&token, &watch.name, &hook_url, &watch.api_base)?;
        println!("{msg}");
        lines.push(msg);
    }
    Ok(lines)
}

/// `destroy service <name>` — disable + remove the unit
/// (`hypha-systemd::remove` is tolerant of a missing unit/bus).
pub fn destroy_service(env: &OpsEnv, name: &str) -> Result<PathBuf, CliError> {
    let path = hypha_systemd::remove(&env.unit_dir, name).map_err(|e| CliError(e.to_string()))?;
    println!("removed {}", path.display());
    Ok(path)
}

/// `destroy worktrees` — `Worktrees::cleanup` over the base dir
/// (`--base`, else `config.worktree_base`, else the driver default)
/// plus the orphan report `Worktrees::list` reconciles.
pub fn destroy_worktrees(
    env: &OpsEnv,
    config: Option<&Path>,
    base: Option<&Path>,
) -> Result<Vec<String>, CliError> {
    let base = match base {
        Some(path) => peregrin_git::expand_path(path),
        None => worktree_base(env, config)?,
    };
    let mgr = Worktrees::new(&base);
    let listings = mgr.list().map_err(cli)?;
    let orphans: Vec<&Path> = listings
        .iter()
        .filter_map(|listing| match listing {
            Listing::Orphan(path) => Some(path.as_path()),
            Listing::Tracked(_) => None,
        })
        .collect();
    let destroyed = mgr.cleanup().map_err(cli)?;
    let mut lines = Vec::new();
    for path in &destroyed {
        lines.push(format!("removed {}", path.display()));
    }
    for orphan in &orphans {
        lines.push(format!(
            "orphan {} (not manifest-tracked — left in place)",
            orphan.display()
        ));
    }
    lines.push(format!(
        "{} worktree(s) removed, {} orphan(s) reported",
        destroyed.len(),
        orphans.len()
    ));
    for line in &lines {
        println!("{line}");
    }
    Ok(lines)
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
    /// done | failed | cancelled | running.
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
/// `*.done`/`workflow.done` → `done`, else `running`.
fn run_status(events: &[Value]) -> String {
    let mut status = "running";
    for event in events {
        status = match type_segment(event_kind(event)) {
            "cancelled" => "cancelled",
            "failed" => "failed",
            "done" => "done",
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
