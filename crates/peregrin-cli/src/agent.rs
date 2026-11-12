//! The agent ops — the retired `peregrin-mcp` tool surface re-expressed
//! as bounded engine verbs (P-028 / T-021). Every verb prints one JSON
//! document so `cli.<verb>` op callers and the generated `mcp` surface
//! get structured answers, never scraped prose.
//!
//! Tool → verb mapping (the MCP names were snake_case tools; the ops
//! are dotted clap leaves):
//!
//! | peregrin-mcp tool | op |
//! | --- | --- |
//! | `validate` | `cli.campaign.validate` (`--bundle`/`--workflow` selectors) |
//! | `bundle_install` | `cli.bundle.install` |
//! | `bundle_remove` | `cli.bundle.remove` |
//! | `bundle_list` | `cli.bundle.list` |
//! | `bundle_info` | `cli.bundle.info` |
//! | `runs` | `cli.logs.runs` |
//! | `events` | `cli.logs.events` (bounded `--limit`, default 50) |
//! | `explain` | `cli.logs.explain` |
//! | `campaign_run` | `cli.campaign.submit` + `cli.campaign.status` + `cli.campaign.events` — submit+poll, never blocking |
//!
//! `campaign submit` spawns a detached `run campaign exec` child (the
//! hidden plumbing verb — MCP's `campaign_run` body: load → config
//! merge → `RunOptions` → durable JSONL log) and returns `{run_id,
//! log, …}` immediately; `status`/`events` poll `<home>/runs/<id>.jsonl`.
//! Unbounded things stay surfaces: `campaign serve`, `campaign run`
//! (supervisor mode), `sessions attach`/`stream`, `events`, `serve`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};

use peregrin_core::Event;
use peregrin_store::EventStore;
use peregrin_store_json::JsonStore;
use serde_json::{Map, Value, json};
use tamias_core::StoreEntry;
use tamias_store_local::Store;

use crate::CliError;
use crate::ops::OpsEnv;

fn cli(e: impl std::fmt::Display) -> CliError {
    CliError(e.to_string())
}

/// Print the verb's JSON answer — pretty, one document.
pub fn emit(v: &Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(v).unwrap_or_else(|_| v.to_string())
    );
}

fn expand(path: &Path) -> PathBuf {
    PathBuf::from(peregrin_config::expand_tilde(&path.to_string_lossy()))
}

// --- bundle store verbs (the MCP `bundle_*` tools) -------------------------

/// The tamias install store + fetch cache → the shared install
/// pipeline's env. These are tamias's roots (`TAMIAS_HOME`/`TAMIAS_CACHE`
/// → XDG → `~/.local|cache`), so `bundle install` here lands in the
/// same store `tamias install` writes — one store, one pipeline.
fn install_env(env: &OpsEnv) -> tamias_install::Env {
    tamias_install::Env {
        store_root: env.store_root.clone(),
        cache_root: env.cache_root.clone(),
    }
}

fn entry_json(entry: &StoreEntry) -> Value {
    json!({
        "label": entry.label(),
        "name": entry.record.name,
        "version": entry.record.version,
        "source_ref": entry.record.source_ref,
        "resolved": entry.record.resolved,
        "digest": entry.record.digest,
        "dir": entry.dir.display().to_string(),
    })
}

/// `bundle install <ref>` — the shared tamias install pipeline
/// (resolve → fetch/cache → manifest → driver consent gate → store
/// commit). `allow_drivers` is the bundled-driver consent flag.
pub fn bundle_install(
    env: &OpsEnv,
    reference: &str,
    allow_drivers: bool,
) -> Result<Value, CliError> {
    let entry =
        tamias_install::install(&install_env(env), reference, allow_drivers).map_err(cli)?;
    Ok(entry_json(&entry))
}

/// `bundle remove <name>` — drop the store entry; the fetch cache is
/// untouched (same semantics as the MCP tool).
pub fn bundle_remove(env: &OpsEnv, name: &str) -> Result<Value, CliError> {
    let record = Store::new(&env.store_root).remove(name).map_err(cli)?;
    Ok(json!({
        "removed": format!("{}@{}", record.name, record.version.clone().unwrap_or_default()),
        "name": record.name,
        "resolved": record.resolved,
    }))
}

/// `bundle list` — every installed entry, `name@version` + provenance.
pub fn bundle_list(env: &OpsEnv) -> Result<Value, CliError> {
    let entries = Store::new(&env.store_root).entries().map_err(cli)?;
    Ok(json!({
        "bundles": entries.iter().map(entry_json).collect::<Vec<_>>(),
    }))
}

/// `bundle info <name>` — entry + `installed_at`, `size_bytes`, and
/// the manifest's `description`/`include` (or `manifest_error`).
pub fn bundle_info(env: &OpsEnv, name: &str) -> Result<Value, CliError> {
    let entry = Store::new(&env.store_root).find(name).map_err(cli)?;
    let mut info = entry_json(&entry);
    info["installed_at"] = json!(entry.record.installed_at);
    info["size_bytes"] = json!(entry.size().map_err(cli)?);
    match entry.manifest() {
        Ok(manifest) => {
            info["description"] = json!(manifest.description);
            info["include"] = json!(manifest.include);
        }
        Err(e) => info["manifest_error"] = json!(e.to_string()),
    }
    Ok(info)
}

// --- run-log observability (the MCP `runs`/`events`/`explain` tools) --------

/// The dir `logs runs` scans when `--dir` is omitted —
/// `<peregrin-home>/runs`, where `campaign submit` writes its logs.
pub fn runs_dir(env: &OpsEnv) -> PathBuf {
    env.home.join("runs")
}

/// Recursive `*.jsonl` walk, newest mtime first.
fn run_logs(dir: &Path) -> Result<Vec<PathBuf>, CliError> {
    if !dir.is_dir() {
        return Err(cli(format!("no such dir: {}", dir.display())));
    }
    let mut logs = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(next) = stack.pop() {
        for entry in
            std::fs::read_dir(&next).map_err(|e| cli(format!("read {}: {e}", next.display())))?
        {
            let path = entry
                .map_err(|e| cli(format!("read {}: {e}", next.display())))?
                .path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().and_then(|e| e.to_str()) == Some("jsonl") {
                logs.push(path);
            }
        }
    }
    let mtime = |p: &Path| {
        std::fs::metadata(p)
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
    };
    logs.sort_by_key(|p| std::cmp::Reverse(mtime(p)));
    Ok(logs)
}

fn sources_of(events: &[Event]) -> Value {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for e in events {
        *counts.entry(e.source.as_str()).or_default() += 1;
    }
    json!(counts)
}

/// `done` / `failed` / `cancelled` / `running` / `empty` — the MCP
/// verdict ladder off the log's terminal event.
fn verdict_of(terminal: Option<&Event>) -> &'static str {
    match terminal.map(|e| e.kind.as_str()) {
        None => "empty",
        Some("workflow.done") => "done",
        Some("workflow.failed") => "failed",
        Some("workflow.cancelled") => "cancelled",
        Some(_) => "running",
    }
}

/// Open a log and summarize — malformed files surface as
/// `{path, error}` rather than being silently skipped.
fn run_summary(path: &Path) -> Value {
    match JsonStore::open(path) {
        Ok(store) => {
            let all = store.all();
            let terminal = all.last();
            json!({
                "path": path.display().to_string(),
                "events": all.len(),
                "sources": sources_of(all),
                "terminal": terminal.map_or("", |e| e.kind.as_str()),
                "verdict": verdict_of(terminal),
            })
        }
        Err(e) => json!({"path": path.display().to_string(), "error": e.to_string()}),
    }
}

/// `logs runs [--dir] [--limit]` — recursive `*.jsonl` scan, newest
/// first: `{dir, runs[{path,events,sources,terminal,verdict}], total}`.
pub fn logs_runs(env: &OpsEnv, dir: Option<&Path>, limit: usize) -> Result<Value, CliError> {
    let dir = dir.map_or_else(|| runs_dir(env), expand);
    let logs = run_logs(&dir)?;
    let summaries: Vec<Value> = logs.iter().take(limit).map(|p| run_summary(p)).collect();
    Ok(json!({"dir": dir.display().to_string(), "runs": summaries, "total": logs.len()}))
}

/// `JsonStore::open` treats a missing file as an empty store, so the
/// existence check is what makes a typo'd path an error.
fn open_log(path: &Path) -> Result<JsonStore, CliError> {
    let path = expand(path);
    if !path.is_file() {
        return Err(cli(format!("no such log: {}", path.display())));
    }
    JsonStore::open(path).map_err(cli)
}

/// The bounded tail shared by `logs events` and `campaign events` —
/// `--limit` is always applied (defaulted upstream), so the op never
/// streams unboundedly.
fn tail_events(path: &Path, kind: Option<&str>, limit: usize) -> Result<Value, CliError> {
    let store = open_log(path)?;
    let matching: Vec<&Event> = store
        .all()
        .iter()
        .filter(|e| kind.is_none_or(|k| e.kind == k))
        .collect();
    let start = matching.len().saturating_sub(limit);
    Ok(json!({
        "total": matching.len(),
        "events": matching[start..]
            .iter()
            .map(|e| serde_json::to_value(*e).unwrap_or_default())
            .collect::<Vec<_>>(),
    }))
}

/// `logs events <path> [--type] [--limit]` — the last `limit`
/// events of one JSONL run log, optionally one event `type`.
pub fn logs_events(path: &Path, kind: Option<&str>, limit: usize) -> Result<Value, CliError> {
    tail_events(path, kind, limit)
}

/// `logs explain <path>` — verdict, terminal event, per-source/
/// per-kind counts, node failures, duration.
pub fn logs_explain(path: &Path) -> Result<Value, CliError> {
    let store = open_log(path)?;
    let all = store.all();
    let mut kinds: BTreeMap<&str, usize> = BTreeMap::new();
    for e in all {
        *kinds.entry(e.kind.as_str()).or_default() += 1;
    }
    let failures: Vec<Value> = all
        .iter()
        .filter(|e| e.kind == "node.failed")
        .map(|e| {
            json!({
                "node": e.payload.get("node"),
                "error": e.payload.get("error"),
            })
        })
        .collect();
    let (first, last) = (all.first(), all.last());
    let duration_ms = match (first, last) {
        (Some(f), Some(l)) => l.ts - f.ts,
        _ => 0,
    };
    Ok(json!({
        "path": store.path().display().to_string(),
        "events": all.len(),
        "duration_ms": duration_ms,
        "verdict": verdict_of(last),
        "terminal": last.map_or_else(|| json!(null), |e| {
            serde_json::to_value(e).unwrap_or_default()
        }),
        "sources": sources_of(all),
        "kinds": json!(kinds),
        "failures": failures,
    }))
}

// --- campaign submit + poll (the MCP `campaign_run` tool, P-028 option b) ---

/// `path` OR `bundle` (+ `workflow` = a file inside the bundle —
/// default `main.yaml`/`main.yml`). `path` wins when both are given.
/// `bundle` resolves against the tamias install store — the same
/// entries `tamias install`/`bundle list` see.
pub fn resolve_workflow(
    env: &OpsEnv,
    path: Option<&Path>,
    bundle: Option<&str>,
    entry: Option<&str>,
) -> Result<PathBuf, CliError> {
    if let Some(path) = path {
        return Ok(expand(path));
    }
    let bundle = bundle.ok_or_else(|| cli("missing `path` or `bundle`"))?;
    let store_entry = Store::new(&env.store_root).find(bundle).map_err(cli)?;
    match entry {
        Some(rel) => {
            let path = store_entry.dir.join(rel);
            if path.is_file() {
                Ok(path)
            } else {
                Err(cli(format!(
                    "no workflow '{rel}' in bundle {}",
                    store_entry.label()
                )))
            }
        }
        None => ["main.yaml", "main.yml"]
            .iter()
            .map(|name| store_entry.dir.join(name))
            .find(|path| path.is_file())
            .ok_or_else(|| {
                cli(format!(
                    "bundle {} has no main.yaml — pass `workflow`",
                    store_entry.label()
                ))
            }),
    }
}

/// `--config`/`--secrets` JSON args → typed maps. `config` merges
/// top-level over the campaign's `config:`; `secrets` values must be
/// strings (the MCP tool's contract).
pub fn parse_config_arg(raw: &str) -> Result<Map<String, Value>, CliError> {
    let value: Value = serde_json::from_str(raw).map_err(|e| cli(format!("--config: {e}")))?;
    value
        .as_object()
        .cloned()
        .ok_or_else(|| cli("`config` must be a JSON object"))
}

/// `--secrets` JSON arg → the run's explicit secrets record.
pub fn parse_secrets_arg(raw: &str) -> Result<BTreeMap<String, String>, CliError> {
    let value: Value = serde_json::from_str(raw).map_err(|e| cli(format!("--secrets: {e}")))?;
    let map = value
        .as_object()
        .ok_or_else(|| cli("`secrets` must be a JSON object"))?;
    let mut secrets = BTreeMap::new();
    for (key, value) in map {
        let value = value
            .as_str()
            .ok_or_else(|| cli(format!("secrets.{key} must be a string")))?;
        secrets.insert(key.clone(), value.to_string());
    }
    Ok(secrets)
}

static RUN_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Mint a run id — `{epoch_ms}-{pid}-{n}`; collision-safe within and
/// across processes (the runs dir is shared).
fn run_id() -> String {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis());
    format!(
        "{}-{}-{}",
        millis,
        std::process::id(),
        RUN_COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}

/// A run id is a filename component — reject anything that could
/// escape `<home>/runs/`.
fn run_log_path(env: &OpsEnv, run: &str, log: Option<&Path>) -> Result<PathBuf, CliError> {
    if let Some(log) = log {
        return Ok(expand(log));
    }
    if run.is_empty() || run.contains('/') || run.contains('\\') || run.contains("..") {
        return Err(cli(format!("bad run id '{run}'")));
    }
    Ok(runs_dir(env).join(format!("{run}.jsonl")))
}

/// `campaign submit` parameters — the workflow selector (`path` or
/// `bundle`+`workflow`), the run overrides, and the log override,
/// bundled so the verb stays under the arg-count lint.
#[derive(Clone, Copy)]
pub struct SubmitArgs<'a> {
    /// Workflow YAML path (wins over `bundle`).
    pub path: Option<&'a Path>,
    /// Installed-bundle name to resolve `workflow` inside.
    pub bundle: Option<&'a str>,
    /// File inside the bundle (default `main.yaml`/`main.yml`).
    pub entry: Option<&'a str>,
    /// Raw `--config` JSON — merged top-level into `campaign.config`.
    pub config: Option<&'a str>,
    /// Raw `--secrets` JSON — name→value for this run only.
    pub secrets: Option<&'a str>,
    /// Simulate without driver side effects.
    pub dry_run: bool,
    /// Log override (default `<home>/runs/<run-id>.jsonl`).
    pub log: Option<&'a Path>,
}

/// `campaign submit` — resolve the workflow, fail fast on a load
/// error, then spawn a detached `run campaign exec` child writing
/// `<home>/runs/<run-id>.jsonl` and return `{run_id, log, …}`
/// immediately. Poll with `campaign status`/`campaign events`.
///
/// The child is `current_exe run …` — under `cli.*` ops and the
/// `cli`/`mcp` surfaces that's `tamias-driver-peregrin`, whose `run`
/// verb re-enters [`crate::cli::run_from`]. Its stdio is null: the
/// JSONL log is the run's only record (same as MCP's `--log`).
pub fn campaign_submit(env: &OpsEnv, args: &SubmitArgs<'_>) -> Result<Value, CliError> {
    let SubmitArgs {
        path,
        bundle,
        entry,
        config,
        secrets,
        dry_run,
        log,
    } = *args;
    // Parse + resolve before spawning — arg and load errors are
    // synchronous answers, not a dead run id. The child re-parses the
    // raw JSON args itself.
    if let Some(raw) = config {
        parse_config_arg(raw)?;
    }
    if let Some(raw) = secrets {
        parse_secrets_arg(raw)?;
    }
    let workflow = resolve_workflow(env, path, bundle, entry)?;
    let campaign = peregrin_node::campaign::load(&workflow).map_err(cli)?;

    let id = run_id();
    let log_path = match log {
        Some(log) => expand(log),
        None => runs_dir(env).join(format!("{id}.jsonl")),
    };
    if let Some(parent) = log_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| cli(format!("create {}: {e}", parent.display())))?;
    }

    let exe = std::env::current_exe().map_err(|e| cli(format!("current_exe: {e}")))?;
    let mut cmd = std::process::Command::new(exe);
    cmd.arg("run")
        .arg("campaign")
        .arg("exec")
        .arg(&workflow)
        .arg("--history")
        .arg(&log_path);
    if let Some(raw) = config {
        cmd.arg("--config").arg(raw);
    }
    if let Some(raw) = secrets {
        cmd.arg("--secrets").arg(raw);
    }
    if dry_run {
        cmd.arg("--dry-run");
    }
    let child = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| cli(format!("spawn campaign exec: {e}")))?;

    Ok(json!({
        "run_id": id,
        "workflow": campaign.name,
        "log": log_path.display().to_string(),
        "pid": child.id(),
    }))
}

/// `campaign status <run>` — the run's current verdict off its JSONL
/// log: `{run, log, events, verdict, terminal, sources, duration_ms}`.
/// A not-yet-created log reports `verdict: "pending"` so poll loops
/// can start immediately after `submit`.
pub fn campaign_status(env: &OpsEnv, run: &str, log: Option<&Path>) -> Result<Value, CliError> {
    let path = run_log_path(env, run, log)?;
    if !path.is_file() {
        return Ok(json!({
            "run": run,
            "log": path.display().to_string(),
            "events": 0,
            "verdict": "pending",
        }));
    }
    let store = JsonStore::open(&path).map_err(cli)?;
    let all = store.all();
    let (first, last) = (all.first(), all.last());
    Ok(json!({
        "run": run,
        "log": path.display().to_string(),
        "events": all.len(),
        "duration_ms": match (first, last) {
            (Some(f), Some(l)) => l.ts - f.ts,
            _ => 0,
        },
        "verdict": verdict_of(last),
        "terminal": last.map_or_else(|| json!(null), |e| {
            serde_json::to_value(e).unwrap_or_default()
        }),
        "sources": sources_of(all),
    }))
}

/// `campaign events <run> [--type] [--limit]` — the bounded tail of
/// a submitted run's log (`<home>/runs/<run>.jsonl` unless `--log`).
pub fn campaign_events(
    env: &OpsEnv,
    run: &str,
    log: Option<&Path>,
    kind: Option<&str>,
    limit: usize,
) -> Result<Value, CliError> {
    let path = run_log_path(env, run, log)?;
    tail_events(&path, kind, limit)
}

// --- tests --------------------------------------------------------------------

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::pedantic)]

    use super::*;

    fn write_log(path: &Path, lines: &[&str]) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, lines.join("\n") + "\n").unwrap();
    }

    #[test]
    fn bundle_verbs_lifecycle() {
        let tmp = tempfile::tempdir().unwrap();
        let env = OpsEnv::under(tmp.path());
        assert_eq!(bundle_list(&env).unwrap()["bundles"], json!([]));

        // A payload dir — manifest + workflow; `type` defaults to
        // files so no driver consent is needed.
        let dir = tmp.path().join("demo");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("tamias.yaml"),
            "name: demo\nversion: \"0.1.0\"\ndescription: test bundle\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("main.yaml"),
            "name: bundled\nnodes:\n  - id: n\n    driver: noop\n    on: [start]\n",
        )
        .unwrap();

        let entry = bundle_install(&env, dir.to_str().unwrap(), false).unwrap();
        assert_eq!(entry["label"], "demo@0.1.0");

        let list = bundle_list(&env).unwrap();
        assert_eq!(list["bundles"].as_array().unwrap().len(), 1);

        let info = bundle_info(&env, "demo").unwrap();
        assert_eq!(info["name"], "demo");
        assert_eq!(info["description"], "test bundle");
        assert!(info["size_bytes"].as_u64().unwrap() > 0);

        let removed = bundle_remove(&env, "demo").unwrap();
        assert_eq!(removed["name"], "demo");
        assert_eq!(bundle_list(&env).unwrap()["bundles"], json!([]));

        assert!(bundle_info(&env, "ghost").is_err());
        assert!(bundle_remove(&env, "ghost").is_err());
        assert!(bundle_install(&env, "definitely-not-a-ref", false).is_err());
    }

    #[test]
    fn logs_verbs_over_fixtures() {
        let tmp = tempfile::tempdir().unwrap();
        let env = OpsEnv::under(tmp.path());
        let logs = tmp.path().join("logs");
        write_log(
            &logs.join("a.jsonl"),
            &[
                "{\"type\":\"start\",\"payload\":{},\"source\":\"__runner__\",\"ts\":1000}",
                "{\"type\":\"n.done\",\"payload\":{},\"source\":\"n\",\"ts\":1500}",
                "{\"type\":\"workflow.done\",\"payload\":{},\"source\":\"__runner__\",\"ts\":2000}",
            ],
        );
        write_log(
            &logs.join("nested").join("b.jsonl"),
            &[
                "{\"type\":\"start\",\"payload\":{},\"source\":\"__runner__\",\"ts\":100}",
                "{\"type\":\"node.failed\",\"payload\":{\"node\":\"x\",\"error\":\"boom\"},\"source\":\"x\",\"ts\":200}",
                "{\"type\":\"workflow.failed\",\"payload\":{\"node\":\"x\",\"error\":\"boom\"},\"source\":\"__runner__\",\"ts\":300}",
            ],
        );
        std::fs::write(logs.join("notes.txt"), "hello\n").unwrap();
        std::fs::write(logs.join("broken.jsonl"), "{not json\n").unwrap();

        // runs — recursive scan, verdicts, malformed surfaced.
        let report = logs_runs(&env, Some(&logs), 20).unwrap();
        assert_eq!(report["total"], 3);
        let runs = report["runs"].as_array().unwrap();
        assert!(runs.iter().any(|r| r["verdict"] == "done"));
        assert!(runs.iter().any(|r| r["verdict"] == "failed"));
        assert!(runs.iter().any(|r| r["error"].is_string()));
        assert!(logs_runs(&env, Some(Path::new("/nonexistent-dir")), 20).is_err());

        // Default dir is <home>/runs.
        let home_logs = runs_dir(&env);
        std::fs::create_dir_all(&home_logs).unwrap();
        write_log(&home_logs.join("home.jsonl"), &[]);
        let report = logs_runs(&env, None, 20).unwrap();
        assert_eq!(report["total"], 1);

        // events — bounded tail + type filter.
        let a = logs.join("a.jsonl");
        let report = logs_events(&a, None, 2).unwrap();
        assert_eq!(report["total"], 3);
        assert_eq!(report["events"].as_array().unwrap().len(), 2);
        assert_eq!(report["events"][0]["type"], "n.done");
        let report = logs_events(&a, Some("workflow.done"), 50).unwrap();
        assert_eq!(report["total"], 1);
        assert!(logs_events(Path::new("/nonexistent.jsonl"), None, 50).is_err());
        assert!(logs_events(&logs.join("broken.jsonl"), None, 50).is_err());

        // explain — verdict, failures, duration, kinds.
        let b = logs.join("nested").join("b.jsonl");
        let report = logs_explain(&b).unwrap();
        assert_eq!(report["verdict"], "failed");
        assert_eq!(report["events"], 3);
        assert_eq!(report["duration_ms"], 200);
        assert_eq!(report["failures"][0]["node"], "x");
        assert_eq!(report["failures"][0]["error"], "boom");
        assert_eq!(report["kinds"]["node.failed"], 1);
        let report = logs_explain(&a).unwrap();
        assert_eq!(report["verdict"], "done");
        assert_eq!(report["terminal"]["type"], "workflow.done");
        assert!(logs_explain(Path::new("/nonexistent.jsonl")).is_err());
    }

    #[test]
    fn resolve_workflow_path_bundle_and_entry() {
        let tmp = tempfile::tempdir().unwrap();
        let env = OpsEnv::under(tmp.path());
        let wf = tmp.path().join("wf.yaml");
        std::fs::write(&wf, "nodes: []\n").unwrap();
        assert_eq!(resolve_workflow(&env, Some(&wf), None, None).unwrap(), wf);

        // Bundle resolution — main.yaml default, `workflow` override,
        // missing-entry error.
        let dir = tmp.path().join("demo");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("tamias.yaml"), "name: demo\n").unwrap();
        std::fs::write(dir.join("main.yaml"), "nodes: []\n").unwrap();
        std::fs::write(dir.join("alt.yaml"), "nodes: []\n").unwrap();
        bundle_install(&env, dir.to_str().unwrap(), false).unwrap();

        let got = resolve_workflow(&env, None, Some("demo"), None).unwrap();
        assert!(got.ends_with("main.yaml"));
        let got = resolve_workflow(&env, None, Some("demo"), Some("alt.yaml")).unwrap();
        assert!(got.ends_with("alt.yaml"));
        assert!(resolve_workflow(&env, None, Some("demo"), Some("nope.yaml")).is_err());
        assert!(resolve_workflow(&env, None, Some("ghost"), None).is_err());
        assert!(resolve_workflow(&env, None, None, None).is_err());
    }

    #[test]
    fn status_pending_then_events_after_log() {
        let tmp = tempfile::tempdir().unwrap();
        let env = OpsEnv::under(tmp.path());
        let status = campaign_status(&env, "run-1", None).unwrap();
        assert_eq!(status["verdict"], "pending");
        assert_eq!(status["events"], 0);

        let log = runs_dir(&env).join("run-1.jsonl");
        write_log(
            &log,
            &[
                "{\"type\":\"start\",\"payload\":{},\"source\":\"__runner__\",\"ts\":1000}",
                "{\"type\":\"workflow.done\",\"payload\":{},\"source\":\"__runner__\",\"ts\":2000}",
            ],
        );
        let status = campaign_status(&env, "run-1", None).unwrap();
        assert_eq!(status["verdict"], "done");
        assert_eq!(status["events"], 2);
        assert_eq!(status["duration_ms"], 1000);

        let events = campaign_events(&env, "run-1", None, Some("workflow.done"), 50).unwrap();
        assert_eq!(events["total"], 1);
        assert_eq!(events["events"][0]["type"], "workflow.done");
        // Bounded: limit 1 of 2 → the tail event only.
        let events = campaign_events(&env, "run-1", None, None, 1).unwrap();
        assert_eq!(events["events"].as_array().unwrap().len(), 1);
        assert_eq!(events["events"][0]["type"], "workflow.done");

        // Path traversal + missing log arms.
        assert!(campaign_status(&env, "../escape", None).is_err());
        assert!(campaign_status(&env, "ghost", None).is_ok()); // pending
        assert!(campaign_events(&env, "ghost", None, None, 50).is_err());
    }

    #[test]
    fn arg_parsing_errors() {
        assert!(parse_config_arg("not json").is_err());
        assert!(parse_config_arg("[1]").is_err());
        assert!(parse_config_arg("{}").is_ok());
        assert!(parse_secrets_arg("{}").is_ok());
        assert!(parse_secrets_arg("{\"a\":1}").is_err());
        assert!(parse_secrets_arg("[]").is_err());
        assert_eq!(parse_secrets_arg("{\"tok\":\"!\"}").unwrap()["tok"], "!");
    }
}
