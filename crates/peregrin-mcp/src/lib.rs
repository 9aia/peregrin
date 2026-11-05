//! peregrin-mcp — the agent control surface for the substrate
//! (ADR-0080's "agent surface" bullet). One hand-rolled MCP stdio
//! server — newline-delimited JSON-RPC, the same transport shape
//! sturn-mcp uses; no protocol crate.
//!
//! Every tool is a thin wrapper over the library composition the
//! `peregrin` CLI drives — `peregrin_node::campaign` for load/run,
//! `peregrin-bundle` + `peregrin-source` for the ADR-0079 store verbs
//! (the same fns `peregrin-cli::distribution` calls, recomposed here
//! so nothing `println!`s onto the JSON-RPC stream), and
//! `peregrin-store-json` for run-log observability.
//!
//! Tools are advisory — there is no enforcement layer. `claim`-style
//! coordination belongs to sturn (`sturn claim`), not to this server.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::io::{BufRead, Write as IoWrite};
use std::path::{Path, PathBuf};

use peregrin_bundle::{
    FetchCache, Manifest, Store, StoreEntry, StoreRecord, dir_digest, overlay, payload_root,
};
use peregrin_core::{Campaign, Event};
use peregrin_engine::RunOptions;
use peregrin_source::{Resolved, Source, SourceRef, parse_ref};
use peregrin_source_fs::FsSource;
use peregrin_source_git::GitSource;
use peregrin_source_http::HttpSource;
use peregrin_store::EventStore;
use peregrin_store_json::JsonStore;
use serde_json::{Value, json};

/// One tool in the agent vocabulary.
pub struct Tool {
    /// MCP tool name.
    pub name: &'static str,
    /// What the agent reads before calling it.
    pub about: &'static str,
}

/// The whole surface — the substrate verbs as tools.
pub const TOOLS: &[Tool] = &[
    Tool {
        name: "campaign_run",
        about: "Load a workflow file (or installed bundle's main.yaml) and run it on this machine's campaign plane",
    },
    Tool {
        name: "validate",
        about: "Load + validate a workflow file — parse, expansion, normalization errors plus wiring warnings",
    },
    Tool {
        name: "bundle_install",
        about: "Install a bundle ref (./path, git:<url>[//subdir][#rev], http(s) .tgz) into the store",
    },
    Tool {
        name: "bundle_remove",
        about: "Drop an installed bundle's store entry (the fetch cache is untouched)",
    },
    Tool {
        name: "bundle_list",
        about: "List installed bundles — name@version, resolved sha, source ref",
    },
    Tool {
        name: "bundle_info",
        about: "A bundle's manifest, provenance and size",
    },
    Tool {
        name: "runs",
        about: "Scan a dir for JSONL run logs and summarize each",
    },
    Tool {
        name: "events",
        about: "Tail events from a JSONL run log, optionally filtered by type",
    },
    Tool {
        name: "explain",
        about: "Summarize one run log — verdict, per-source counts, failures, duration",
    },
];

/// Runtime context the tools close over — the store/cache roots the
/// distribution verbs share, plus the default dir `runs` scans.
/// [`Ctx::default`] derives them from `PEREGRIN_HOME`/`PEREGRIN_CACHE`/
/// XDG like the CLI's `DistEnv`; tests inject tempdirs via
/// [`Ctx::under`].
#[derive(Clone, Debug)]
pub struct Ctx {
    /// `<peregrin-home>/store`.
    pub store_root: PathBuf,
    /// `<cache>/fetch`.
    pub cache_root: PathBuf,
    /// Default dir `runs` scans when `dir` is omitted —
    /// `<peregrin-home>/runs`.
    pub runs_dir: PathBuf,
}

impl Ctx {
    /// Store/cache/runs under one sandbox root (tests).
    #[must_use]
    pub fn under(root: &Path) -> Self {
        Self {
            store_root: root.join("store"),
            cache_root: root.join("fetch"),
            runs_dir: root.join("runs"),
        }
    }
}

impl Default for Ctx {
    fn default() -> Self {
        Self {
            store_root: peregrin_node::peregrin_home().join("store"),
            cache_root: peregrin_cache_root().join("fetch"),
            runs_dir: peregrin_node::peregrin_home().join("runs"),
        }
    }
}

/// `PEREGRIN_CACHE` → `XDG_CACHE_HOME/peregrin` → `~/.cache/peregrin`
/// — the same chain `peregrin-cli::distribution` uses.
fn peregrin_cache_root() -> PathBuf {
    std::env::var_os("PEREGRIN_CACHE")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("XDG_CACHE_HOME").map(|d| PathBuf::from(d).join("peregrin")))
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache/peregrin")))
        .unwrap_or_else(|| PathBuf::from(".cache/peregrin"))
}

fn tool_text(v: impl std::fmt::Display) -> Value {
    json!({"content": [{"type": "text", "text": v.to_string()}]})
}

fn tool_err(v: impl std::fmt::Display) -> Value {
    json!({"content": [{"type": "text", "text": v.to_string()}], "isError": true})
}

/// A JSON payload as the tool's text content — every tool answers
/// structured JSON so the caller never scrapes prose.
fn tool_json(v: &Value) -> Value {
    tool_text(serde_json::to_string_pretty(v).unwrap_or_else(|_| v.to_string()))
}

fn arg<'a>(a: &'a Value, k: &str) -> Result<&'a str, String> {
    a.get(k)
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("missing arg `{k}`"))
}

fn fail(e: impl std::fmt::Display) -> String {
    e.to_string()
}

/// Dispatch one `tools/call` — returns the result value.
fn call(ctx: &Ctx, name: &str, a: &Value) -> Value {
    match name {
        "campaign_run" => campaign_run(ctx, a),
        "validate" => validate(ctx, a),
        "bundle_install" => bundle_install(ctx, a),
        "bundle_remove" => bundle_remove(ctx, a),
        "bundle_list" => bundle_list(ctx, a),
        "bundle_info" => bundle_info(ctx, a),
        "runs" => runs(ctx, a),
        "events" => events(ctx, a),
        "explain" => explain(ctx, a),
        _ => tool_err(format!("unknown tool `{name}`")),
    }
}

// --- campaign tools -----------------------------------------------------------

/// `path` OR `bundle` (+ optional `workflow` inside the bundle —
/// default `main.yaml`/`main.yml`). `path` wins when both are given.
fn resolve_workflow(ctx: &Ctx, a: &Value) -> Result<PathBuf, String> {
    if let Ok(path) = arg(a, "path") {
        return Ok(peregrin_source::expand_path(Path::new(path)));
    }
    let bundle = arg(a, "bundle")?;
    let entry = Store::new(&ctx.store_root).find(bundle).map_err(fail)?;
    match a.get("workflow").and_then(|v| v.as_str()) {
        Some(rel) => {
            let path = entry.dir.join(rel);
            if path.is_file() {
                Ok(path)
            } else {
                Err(format!("no workflow '{rel}' in bundle {}", entry.label()))
            }
        }
        None => ["main.yaml", "main.yml"]
            .iter()
            .map(|name| entry.dir.join(name))
            .find(|path| path.is_file())
            .ok_or_else(|| {
                format!(
                    "bundle {} has no main.yaml — pass `workflow`",
                    entry.label()
                )
            }),
    }
}

/// `secrets` arg → `RunOptions::secrets`; values must be strings.
fn secrets_arg(a: &Value) -> Result<BTreeMap<String, String>, String> {
    let mut secrets = BTreeMap::new();
    if let Some(obj) = a.get("secrets") {
        let map = obj
            .as_object()
            .ok_or_else(|| "`secrets` must be an object".to_string())?;
        for (key, value) in map {
            let value = value
                .as_str()
                .ok_or_else(|| format!("secrets.{key} must be a string"))?;
            secrets.insert(key.clone(), value.to_string());
        }
    }
    Ok(secrets)
}

fn campaign_run(ctx: &Ctx, a: &Value) -> Value {
    let r = (|| -> Result<Value, String> {
        let path = resolve_workflow(ctx, a)?;
        let mut campaign = peregrin_node::campaign::load(&path).map_err(fail)?;
        if let Some(config) = a.get("config") {
            let overrides = config
                .as_object()
                .ok_or_else(|| "`config` must be an object".to_string())?;
            // Top-level merge — an override key replaces the
            // campaign's wholesale (no deep walk).
            for (key, value) in overrides {
                campaign.config.insert(key.clone(), value.clone());
            }
        }
        let options = RunOptions {
            dry_run: a.get("dry_run").and_then(Value::as_bool).unwrap_or(false),
            secrets: secrets_arg(a)?,
            ..RunOptions::default()
        };
        let log = match a.get("log").and_then(|v| v.as_str()) {
            Some(log_path) => {
                let mut runner = peregrin_node::campaign::runner_with_log(
                    &peregrin_source::expand_path(Path::new(log_path)),
                )
                .map_err(fail)?;
                runner.run(&campaign, &options).map_err(fail)?
            }
            None => peregrin_node::campaign::runner()
                .run(&campaign, &options)
                .map_err(fail)?,
        };
        let limit = a.get("limit").and_then(Value::as_u64).unwrap_or(25) as usize;
        let start = log.len().saturating_sub(limit);
        let tail: Vec<Value> = log[start..]
            .iter()
            .map(|e| serde_json::to_value(e).unwrap_or_default())
            .collect();
        let terminal = log.last().map_or_else(
            || json!(null),
            |e| serde_json::to_value(e).unwrap_or_default(),
        );
        let kind = log.last().map_or("", |e| e.kind.as_str());
        Ok(json!({
            "ok": kind == "workflow.done",
            "workflow": campaign.name,
            "events": log.len(),
            "terminal": terminal,
            "tail": tail,
        }))
    })();
    r.map_or_else(tool_err, |v| tool_json(&v))
}

/// The `validate` wiring check — a warning per `on` pattern nothing
/// emits (the same rule `peregrin campaign validate` reports).
fn wiring_warnings(campaign: &Campaign) -> Vec<String> {
    let mut emit_types: BTreeSet<&str> = BTreeSet::new();
    for node in &campaign.nodes {
        for emit in &node.emits {
            emit_types.insert(emit.as_str());
        }
    }
    for schema in &campaign.events {
        emit_types.insert(schema.kind.as_str());
    }
    for kind in [
        "start",
        "node.failed",
        "node.exhausted",
        "workflow.done",
        "workflow.failed",
        "workflow.cancel",
        "workflow.cancelled",
        "workflow.cycle_detected",
        "event.invalid",
        "dlq.event",
    ] {
        emit_types.insert(kind);
    }
    let mut warnings = Vec::new();
    for node in &campaign.nodes {
        for pattern in &node.on {
            let satisfied = pattern == "*"
                || emit_types
                    .iter()
                    .any(|emit| peregrin_core::subscribes_to(pattern, emit));
            if !satisfied {
                warnings.push(format!(
                    "node '{}' subscribes to '{pattern}' but no node emits it",
                    node.id
                ));
            }
        }
    }
    warnings
}

fn validate(ctx: &Ctx, a: &Value) -> Value {
    if a.get("path").is_none() && a.get("bundle").is_none() {
        return tool_err("missing `path` or `bundle`");
    }
    match resolve_workflow(ctx, a)
        .and_then(|path| peregrin_node::campaign::load(&path).map_err(fail))
    {
        Ok(campaign) => tool_json(&json!({
            "ok": true,
            "workflow": campaign.name,
            "nodes": campaign.nodes.len(),
            "events": campaign.events.len(),
            "includes": campaign.imports,
            "stop_on": campaign.stop_on,
            "warnings": wiring_warnings(&campaign),
        })),
        // An invalid workflow is a valid answer — structured, not isError.
        Err(e) => tool_json(&json!({"ok": false, "error": e})),
    }
}

// --- distribution — the ADR-0079 composition from peregrin-cli ----------------
// Recomposed here (not imported) because the CLI's verbs `println!`
// their reports, which would corrupt the stdio JSON-RPC stream.

/// Dispatch the parsed ref to its impl.
fn resolve(reference: &SourceRef) -> Result<Resolved, String> {
    match reference {
        SourceRef::Fs(_) => FsSource::new().resolve(reference),
        SourceRef::Git(_) => GitSource::new().resolve(reference),
        SourceRef::Http(_) => HttpSource::new().resolve(reference),
    }
    .map_err(fail)
}

fn fetch(resolved: &Resolved, dest: &Path) -> Result<peregrin_source::Fetched, String> {
    match resolved {
        Resolved::Fs { .. } => FsSource::new().fetch(resolved, dest),
        Resolved::Git { .. } => GitSource::new().fetch(resolved, dest),
        Resolved::Http { .. } => HttpSource::new().fetch(resolved, dest),
    }
    .map_err(fail)
}

/// resolve → cache-hit or fetch+commit → `(payload_dir, resolved_id)`.
fn fetch_or_cached(ctx: &Ctx, resolved: &Resolved) -> Result<(PathBuf, String), String> {
    let cache = FetchCache::new(&ctx.cache_root);
    if let Some((key, resolved_id)) = resolved.prefetch_identity()
        && cache.has(&key)
    {
        return Ok((cache.entry(&key), resolved_id));
    }
    let scratch = cache.scratch().map_err(fail)?;
    match fetch(resolved, &scratch) {
        Err(e) => {
            cache.discard(&scratch);
            Err(e)
        }
        Ok(fetched) => {
            let dir = cache.commit(&scratch, &fetched.content_key).map_err(fail)?;
            Ok((dir, fetched.resolved))
        }
    }
}

/// The recursive include-aware install — `installing` guards cycles
/// across ref-valued `include:` entries.
fn install_ref(
    ctx: &Ctx,
    reference: &str,
    installing: &mut HashSet<String>,
) -> Result<StoreEntry, String> {
    if !installing.insert(reference.to_string()) {
        return Err(format!("include cycle at '{reference}'"));
    }
    let result = install_ref_inner(ctx, reference, installing);
    installing.remove(reference);
    result
}

fn install_ref_inner(
    ctx: &Ctx,
    reference: &str,
    installing: &mut HashSet<String>,
) -> Result<StoreEntry, String> {
    let parsed = parse_ref(reference).map_err(fail)?;
    let resolved = resolve(&parsed)?;
    let (payload_dir, resolved_id) = fetch_or_cached(ctx, &resolved)?;
    let payload = payload_root(&payload_dir).map_err(fail)?;
    let manifest = Manifest::load(&payload).map_err(fail)?;
    let payload = if manifest.include.is_empty() {
        payload
    } else {
        compose(ctx, &manifest, &payload, installing)?
    };
    Store::new(&ctx.store_root)
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
        .map_err(fail)
}

/// Overlay `include:` bases under the local payload — store names
/// resolve to their entry dirs, refs install recursively.
fn compose(
    ctx: &Ctx,
    manifest: &Manifest,
    payload: &Path,
    installing: &mut HashSet<String>,
) -> Result<PathBuf, String> {
    let store = Store::new(&ctx.store_root);
    let cache = FetchCache::new(&ctx.cache_root);
    let mut bases = Vec::with_capacity(manifest.include.len());
    for include in &manifest.include {
        let dir = if parse_ref(include).is_ok() {
            install_ref(ctx, include, installing)?.dir
        } else {
            store
                .find(include)
                .map_err(|e| format!("include '{include}': {e}"))?
                .dir
        };
        bases.push(dir);
    }
    let scratch = cache.scratch().map_err(fail)?;
    let refs: Vec<&Path> = bases.iter().map(PathBuf::as_path).collect();
    if let Err(e) = overlay(&refs, payload, &scratch) {
        cache.discard(&scratch);
        return Err(fail(e));
    }
    let digest = dir_digest(&scratch).map_err(fail)?;
    let dir = cache
        .commit(&scratch, &format!("sha256-{digest}"))
        .map_err(fail)?;
    payload_root(&dir).map_err(fail)
}

fn entry_json(entry: &StoreEntry) -> Value {
    json!({
        "label": entry.label(),
        "name": entry.record.name,
        "version": entry.record.version,
        "source_ref": entry.record.source_ref,
        "resolved_sha": entry.record.resolved_sha,
        "digest": entry.record.digest,
        "dir": entry.dir.display().to_string(),
    })
}

fn bundle_install(ctx: &Ctx, a: &Value) -> Value {
    match arg(a, "ref").and_then(|r| install_ref(ctx, r, &mut HashSet::new())) {
        Ok(entry) => tool_json(&entry_json(&entry)),
        Err(e) => tool_err(e),
    }
}

fn bundle_remove(ctx: &Ctx, a: &Value) -> Value {
    match arg(a, "name").and_then(|name| Store::new(&ctx.store_root).remove(name).map_err(fail)) {
        Ok(record) => tool_json(&json!({
            "removed": format!("{}@{}", record.name, record.version.clone().unwrap_or_default()),
            "name": record.name,
            "resolved_sha": record.resolved_sha,
        })),
        Err(e) => tool_err(e),
    }
}

fn bundle_list(ctx: &Ctx, a: &Value) -> Value {
    let _ = a;
    match Store::new(&ctx.store_root).entries().map_err(fail) {
        Ok(entries) => tool_json(&json!({
            "bundles": entries.iter().map(entry_json).collect::<Vec<_>>(),
        })),
        Err(e) => tool_err(e),
    }
}

fn bundle_info(ctx: &Ctx, a: &Value) -> Value {
    let r = (|| -> Result<Value, String> {
        let entry = Store::new(&ctx.store_root)
            .find(arg(a, "name")?)
            .map_err(fail)?;
        let mut info = entry_json(&entry);
        info["installed_at"] = json!(entry.record.installed_at);
        info["size_bytes"] = json!(entry.size().map_err(fail)?);
        match entry.manifest() {
            Ok(manifest) => {
                info["description"] = json!(manifest.description);
                info["include"] = json!(manifest.include);
            }
            Err(e) => info["manifest_error"] = json!(e.to_string()),
        }
        Ok(info)
    })();
    r.map_or_else(tool_err, |v| tool_json(&v))
}

// --- observability — the JSONL run logs ----------------------------------------

/// Recursive `*.jsonl` walk, newest mtime first.
fn run_logs(dir: &Path) -> Result<Vec<PathBuf>, String> {
    if !dir.is_dir() {
        return Err(format!("no such dir: {}", dir.display()));
    }
    let mut logs = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(next) = stack.pop() {
        for entry in
            std::fs::read_dir(&next).map_err(|e| format!("read {}: {e}", next.display()))?
        {
            let path = entry
                .map_err(|e| format!("read {}: {e}", next.display()))?
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

/// Open a log and summarize — malformed files surface as `{path, error}`
/// rather than being silently skipped.
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

fn sources_of(events: &[Event]) -> Value {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for e in events {
        *counts.entry(e.source.as_str()).or_default() += 1;
    }
    json!(counts)
}

fn verdict_of(terminal: Option<&Event>) -> &'static str {
    match terminal.map(|e| e.kind.as_str()) {
        None => "empty",
        Some("workflow.done") => "done",
        Some("workflow.failed") => "failed",
        Some("workflow.cancelled") => "cancelled",
        Some(_) => "running",
    }
}

fn runs(ctx: &Ctx, a: &Value) -> Value {
    let r = (|| -> Result<Value, String> {
        let dir = match a.get("dir").and_then(|v| v.as_str()) {
            Some(d) => peregrin_source::expand_path(Path::new(d)),
            None => ctx.runs_dir.clone(),
        };
        let limit = a.get("limit").and_then(Value::as_u64).unwrap_or(20) as usize;
        let logs = run_logs(&dir)?;
        let summaries: Vec<Value> = logs.iter().take(limit).map(|p| run_summary(p)).collect();
        Ok(json!({"dir": dir.display().to_string(), "runs": summaries, "total": logs.len()}))
    })();
    r.map_or_else(tool_err, |v| tool_json(&v))
}

/// `events`/`explain` share this open — `JsonStore::open` treats a
/// missing file as an empty store, so the existence check is what
/// makes a typo'd path an error.
fn open_log(a: &Value) -> Result<JsonStore, String> {
    let path = peregrin_source::expand_path(Path::new(arg(a, "path")?));
    if !path.is_file() {
        return Err(format!("no such log: {}", path.display()));
    }
    JsonStore::open(path).map_err(fail)
}

fn events(ctx: &Ctx, a: &Value) -> Value {
    let _ = ctx;
    let r = (|| -> Result<Value, String> {
        let store = open_log(a)?;
        let kind = a.get("type").and_then(|v| v.as_str());
        let limit = a.get("limit").and_then(Value::as_u64).unwrap_or(50) as usize;
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
    })();
    r.map_or_else(tool_err, |v| tool_json(&v))
}

fn explain(ctx: &Ctx, a: &Value) -> Value {
    let _ = ctx;
    let r = (|| -> Result<Value, String> {
        let store = open_log(a)?;
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
    })();
    r.map_or_else(tool_err, |v| tool_json(&v))
}

// --- MCP wire -----------------------------------------------------------------

fn tool_schema(name: &str, about: &str, props: &Value, required: &[&str]) -> Value {
    json!({
        "name": name,
        "description": about,
        "inputSchema": {
            "type": "object",
            "properties": props,
            "required": required,
        }
    })
}

fn tool_list() -> Value {
    let s = |k: &str| (k.to_string(), json!({"type": "string"}));
    json!({"tools": [
        tool_schema("campaign_run", "Run a workflow file or an installed bundle's workflow on the campaign plane. Returns terminal event + tail of the log.", &json!({
            "path": s("workflow").1,
            "bundle": s("bundle").1,
            "workflow": s("workflow").1,
            "config": {"type": "object"},
            "secrets": {"type": "object"},
            "dry_run": {"type": "boolean"},
            "log": s("log").1,
            "limit": {"type": "integer"},
        }), &[]),
        tool_schema("validate", "Load + validate a workflow file (parse, include/snippet expansion, normalization, wiring warnings). {ok:false, error} on failure.", &json!({
            "path": s("workflow").1,
            "bundle": s("bundle").1,
            "workflow": s("workflow").1,
        }), &[]),
        tool_schema("bundle_install", "Install a bundle ref — ./path, /abs/path, git:<url>[//subdir][#rev], or an http(s) .tgz/.tar.gz URL — into the store.", &json!({
            "ref": s("ref").1,
        }), &["ref"]),
        tool_schema("bundle_remove", "Drop an installed bundle's store entry (name or name@version). The fetch cache is untouched.", &json!({
            "name": s("name").1,
        }), &["name"]),
        tool_schema("bundle_list", "List installed bundles — name@version, resolved sha, source ref.", &json!({}), &[]),
        tool_schema("bundle_info", "A bundle's manifest, provenance and size.", &json!({
            "name": s("name").1,
        }), &["name"]),
        tool_schema("runs", "Scan a dir (recursive) for *.jsonl run logs, newest first — path, event count, verdict per run.", &json!({
            "dir": s("dir").1,
            "limit": {"type": "integer"},
        }), &[]),
        tool_schema("events", "Tail events from a JSONL run log; `type` filters to one event type.", &json!({
            "path": s("path").1,
            "type": s("type").1,
            "limit": {"type": "integer"},
        }), &["path"]),
        tool_schema("explain", "Summarize one run log — verdict, terminal event, per-source/per-kind counts, node failures, duration.", &json!({
            "path": s("path").1,
        }), &["path"]),
    ]})
}

/// Serve newline-delimited JSON-RPC (MCP stdio transport) until
/// stdin closes. Compiled out under `cfg(coverage)` — stdin can't be
/// piped in-process; [`serve_io`] carries the same loop and is what
/// tests drive.
#[cfg(not(coverage))]
pub fn serve(ctx: &Ctx) -> std::io::Result<()> {
    let stdin = std::io::stdin().lock();
    let mut out = std::io::stdout().lock();
    serve_io(ctx, stdin, &mut out)
}

/// The stdio loop over any reader/writer — requests and
/// notifications interleave; only requests get responses.
pub fn serve_io(ctx: &Ctx, reader: impl BufRead, writer: &mut impl IoWrite) -> std::io::Result<()> {
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let req: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        // Notifications have no id — never respond to them.
        let Some(id) = req.get("id").cloned() else {
            continue;
        };
        let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let result = match method {
            "initialize" => json!({
                "protocolVersion": req.get("params")
                    .and_then(|p| p.get("protocolVersion"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("2024-11-05"),
                "capabilities": {"tools": {"listChanged": false}},
                "serverInfo": {"name": "peregrin-mcp", "version": env!("CARGO_PKG_VERSION")},
            }),
            "ping" => json!({}),
            "tools/list" => tool_list(),
            "tools/call" => {
                let p = req.get("params").cloned().unwrap_or(json!({}));
                let name = p.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let args = p.get("arguments").cloned().unwrap_or(json!({}));
                call(ctx, name, &args)
            }
            other => {
                let resp = json!({
                    "jsonrpc": "2.0", "id": id,
                    "error": {"code": -32601, "message": format!("method not found: {other}")},
                });
                writer.write_all(resp.to_string().as_bytes())?;
                writer.write_all(b"\n")?;
                writer.flush()?;
                continue;
            }
        };
        let resp = json!({"jsonrpc": "2.0", "id": id, "result": result});
        writer.write_all(resp.to_string().as_bytes())?;
        writer.write_all(b"\n")?;
        writer.flush()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    //! Tool-level coverage: every `tools/call` verb against tempdir
    //! fixture workflows/bundles/logs, plus the tool schema list and
    //! the `serve_io` loop. `serve` itself (real stdin) is excluded
    //! under `cfg(coverage)` like the node mains.

    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::pedantic)]

    use super::*;
    use std::io::Cursor;

    fn fixture_workflow(dir: &Path) -> PathBuf {
        let yaml = format!(
            "name: mcp-fixture\n\
             capabilities:\n  secrets: true\n\
             nodes:\n\
             \x20 - id: write\n\
             \x20   driver: shell\n\
             \x20   on: [start]\n\
             \x20   emits: [write.done, write.failed]\n\
             \x20   params:\n\
             \x20     cmd: \"printf ${{config.msg}}${{secret.tok}} > {dir}/out.txt\"\n\
             \x20     emit:\n\
             \x20       \"0\": write.done\n\
             \x20       default: write.failed\n",
            dir = dir.display()
        );
        let path = dir.join("fixture.yaml");
        std::fs::write(&path, yaml).unwrap();
        path
    }

    /// A bundle payload dir — manifest + a runnable `main.yaml`.
    fn bundle_dir(root: &Path, name: &str, extra: &[(&str, &str)]) -> PathBuf {
        let dir = root.join(name);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("peregrin-bundle.yaml"),
            format!("name: {name}\nversion: \"0.1.0\"\ndescription: test bundle\n"),
        )
        .unwrap();
        std::fs::write(
            dir.join("main.yaml"),
            "name: bundled\nnodes:\n  - id: n\n    driver: noop\n    on: [start]\n",
        )
        .unwrap();
        for (rel, body) in extra {
            let path = dir.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, body).unwrap();
        }
        dir
    }

    fn write_log(path: &Path, lines: &[&str]) {
        std::fs::write(path, lines.join("\n") + "\n").unwrap();
    }

    fn text(v: &Value) -> &str {
        v["content"][0]["text"].as_str().unwrap()
    }

    fn parsed(v: &Value) -> Value {
        serde_json::from_str(text(v)).unwrap()
    }

    fn is_err(v: &Value) -> bool {
        v.get("isError").and_then(|b| b.as_bool()).unwrap_or(false)
    }

    #[test]
    fn tool_vocabulary_and_schemas() {
        assert_eq!(TOOLS.len(), 9);
        let list = tool_list();
        let tools = list["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 9);
        assert!(tools.iter().all(|t| t["inputSchema"]["type"] == "object"));
    }

    #[test]
    fn campaign_run_executes_and_reports() {
        let tmp = tempfile::tempdir().unwrap();
        let ctx = Ctx::under(tmp.path());
        let path = fixture_workflow(tmp.path());
        let v = call(
            &ctx,
            "campaign_run",
            &json!({
                "path": path.to_str().unwrap(),
                "config": {"msg": "hi"},
                "secrets": {"tok": "!"},
                "log": tmp.path().join("run.jsonl").to_str().unwrap(),
            }),
        );
        assert!(!is_err(&v), "{}", text(&v));
        let report = parsed(&v);
        assert_eq!(report["ok"], true);
        assert_eq!(report["workflow"], "mcp-fixture");
        assert_eq!(report["terminal"]["type"], "workflow.done");
        // The config+secrets merge reached the driver params.
        assert_eq!(
            std::fs::read_to_string(tmp.path().join("out.txt")).unwrap(),
            "hi!"
        );
        // `log` persisted a JSONL log the observability tools can read.
        assert!(tmp.path().join("run.jsonl").is_file());
    }

    #[test]
    fn campaign_run_dry_run_and_error_arms() {
        let tmp = tempfile::tempdir().unwrap();
        let ctx = Ctx::under(tmp.path());
        let path = fixture_workflow(tmp.path());
        let v = call(
            &ctx,
            "campaign_run",
            &json!({"path": path.to_str().unwrap(), "dry_run": true}),
        );
        assert_eq!(parsed(&v)["ok"], true);
        assert!(!tmp.path().join("out.txt").exists());

        // Missing both selectors, missing file, unknown bundle,
        // non-object config, non-string secret.
        assert!(is_err(&call(&ctx, "campaign_run", &json!({}))));
        assert!(is_err(&call(
            &ctx,
            "campaign_run",
            &json!({"path": "/nonexistent.yaml"})
        )));
        assert!(is_err(&call(
            &ctx,
            "campaign_run",
            &json!({"bundle": "not-installed"})
        )));
        assert!(is_err(&call(
            &ctx,
            "campaign_run",
            &json!({"path": path.to_str().unwrap(), "config": "x"})
        )));
        assert!(is_err(&call(
            &ctx,
            "campaign_run",
            &json!({"path": path.to_str().unwrap(), "secrets": {"a": 1}})
        )));
        assert!(is_err(&call(
            &ctx,
            "campaign_run",
            &json!({"path": path.to_str().unwrap(), "secrets": "x"})
        )));
    }

    #[test]
    fn campaign_run_from_bundle() {
        let tmp = tempfile::tempdir().unwrap();
        let ctx = Ctx::under(tmp.path());
        let bundle = bundle_dir(tmp.path(), "demo", &[]);
        let v = call(
            &ctx,
            "bundle_install",
            &json!({"ref": bundle.to_str().unwrap()}),
        );
        assert!(!is_err(&v), "{}", text(&v));

        let v = call(&ctx, "campaign_run", &json!({"bundle": "demo"}));
        let report = parsed(&v);
        assert_eq!(report["workflow"], "bundled");
        assert_eq!(report["terminal"]["type"], "workflow.done");

        // Missing workflow inside the bundle → clean error.
        let v = call(
            &ctx,
            "campaign_run",
            &json!({"bundle": "demo", "workflow": "other.yaml"}),
        );
        assert!(is_err(&v));
    }

    #[test]
    fn validate_ok_warnings_and_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let ctx = Ctx::under(tmp.path());
        let path = fixture_workflow(tmp.path());
        let v = call(&ctx, "validate", &json!({"path": path.to_str().unwrap()}));
        let report = parsed(&v);
        assert_eq!(report["ok"], true);
        assert_eq!(report["nodes"], 1);
        assert_eq!(report["warnings"], json!([]));

        // Orphan subscription → warning, still ok.
        let warn = tmp.path().join("warn.yaml");
        std::fs::write(
            &warn,
            "nodes:\n  - id: orphan\n    driver: shell\n    on: [never.emitted]\n",
        )
        .unwrap();
        let report = parsed(&call(
            &ctx,
            "validate",
            &json!({"path": warn.to_str().unwrap()}),
        ));
        assert_eq!(report["ok"], true);
        assert!(
            report["warnings"].as_array().unwrap()[0]
                .as_str()
                .unwrap()
                .contains("never.emitted"),
            "report: {report}"
        );

        // Load failure → {ok:false, error} — a valid answer, not isError.
        let report = parsed(&call(
            &ctx,
            "validate",
            &json!({"path": "/nonexistent.yaml"}),
        ));
        assert_eq!(report["ok"], false);
        assert!(report["error"].as_str().unwrap().contains("nonexistent"));

        // Missing arg → isError.
        assert!(is_err(&call(&ctx, "validate", &json!({}))));
    }

    #[test]
    fn bundle_verbs_lifecycle() {
        let tmp = tempfile::tempdir().unwrap();
        let ctx = Ctx::under(tmp.path());

        // Empty store lists cleanly.
        let report = parsed(&call(&ctx, "bundle_list", &json!({})));
        assert_eq!(report["bundles"], json!([]));

        let bundle = bundle_dir(tmp.path(), "demo", &[]);
        let v = call(
            &ctx,
            "bundle_install",
            &json!({"ref": bundle.to_str().unwrap()}),
        );
        assert!(!is_err(&v), "{}", text(&v));
        assert_eq!(parsed(&v)["label"], "demo@0.1.0");

        let report = parsed(&call(&ctx, "bundle_list", &json!({})));
        assert_eq!(report["bundles"].as_array().unwrap().len(), 1);

        let report = parsed(&call(&ctx, "bundle_info", &json!({"name": "demo"})));
        assert_eq!(report["name"], "demo");
        assert_eq!(report["description"], "test bundle");
        assert!(report["size_bytes"].as_u64().unwrap() > 0);

        let report = parsed(&call(&ctx, "bundle_remove", &json!({"name": "demo"})));
        assert_eq!(report["name"], "demo");
        let report = parsed(&call(&ctx, "bundle_list", &json!({})));
        assert_eq!(report["bundles"], json!([]));

        // Error arms — unknown names, bad ref, missing args.
        assert!(is_err(&call(
            &ctx,
            "bundle_info",
            &json!({"name": "ghost"})
        )));
        assert!(is_err(&call(
            &ctx,
            "bundle_remove",
            &json!({"name": "ghost"})
        )));
        assert!(is_err(&call(
            &ctx,
            "bundle_install",
            &json!({"ref": "definitely-not-a-ref"})
        )));
        assert!(is_err(&call(
            &ctx,
            "bundle_install",
            &json!({"ref": "/nonexistent/dir"})
        )));
        assert!(is_err(&call(&ctx, "bundle_install", &json!({}))));
        assert!(is_err(&call(&ctx, "bundle_info", &json!({}))));
    }

    #[test]
    fn bundle_install_composes_includes_and_rejects_cycles() {
        let tmp = tempfile::tempdir().unwrap();
        let ctx = Ctx::under(tmp.path());
        let base = bundle_dir(tmp.path(), "base", &[]);
        // A payload whose manifest `include:` is a ref (absolute path)
        // — exercises the recursive install_ref + overlay path.
        let top = tmp.path().join("top");
        std::fs::create_dir_all(&top).unwrap();
        std::fs::write(
            top.join("peregrin-bundle.yaml"),
            format!("name: top\ninclude:\n  - {}\n", base.display()),
        )
        .unwrap();
        let v = call(
            &ctx,
            "bundle_install",
            &json!({"ref": top.to_str().unwrap()}),
        );
        assert!(!is_err(&v), "{}", text(&v));
        assert!(ctx.store_root.join("base@0.1.0").is_dir());
        // Unversioned → the label carries the resolved sha's short form.
        assert!(Store::new(&ctx.store_root).find("top").is_ok());

        // Include naming an installed store entry.
        let mid = tmp.path().join("mid");
        std::fs::create_dir_all(&mid).unwrap();
        std::fs::write(
            mid.join("peregrin-bundle.yaml"),
            "name: mid\ninclude: [base]\n",
        )
        .unwrap();
        let v = call(
            &ctx,
            "bundle_install",
            &json!({"ref": mid.to_str().unwrap()}),
        );
        assert!(!is_err(&v), "{}", text(&v));

        // Self-include → cycle guard.
        let loopy = tmp.path().join("loopy");
        std::fs::create_dir_all(&loopy).unwrap();
        std::fs::write(
            loopy.join("peregrin-bundle.yaml"),
            format!("name: loopy\ninclude:\n  - {}\n", loopy.display()),
        )
        .unwrap();
        let v = call(
            &ctx,
            "bundle_install",
            &json!({"ref": loopy.to_str().unwrap()}),
        );
        assert!(is_err(&v));
        assert!(text(&v).contains("cycle"));

        // Missing store include → named error.
        let missing = tmp.path().join("missing");
        std::fs::create_dir_all(&missing).unwrap();
        std::fs::write(
            missing.join("peregrin-bundle.yaml"),
            "name: missing\ninclude: [ghost]\n",
        )
        .unwrap();
        let v = call(
            &ctx,
            "bundle_install",
            &json!({"ref": missing.to_str().unwrap()}),
        );
        assert!(is_err(&v));
        assert!(text(&v).contains("ghost"));
    }

    #[test]
    fn runs_events_explain_over_log_fixtures() {
        let tmp = tempfile::tempdir().unwrap();
        let ctx = Ctx::under(tmp.path());
        let logs = tmp.path().join("logs");
        std::fs::create_dir_all(logs.join("nested")).unwrap();
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
        // Not a log — ignored by `runs`.
        std::fs::write(logs.join("notes.txt"), "hello\n").unwrap();
        // Malformed .jsonl — surfaced with an error field.
        std::fs::write(logs.join("broken.jsonl"), "{not json\n").unwrap();

        let report = parsed(&call(&ctx, "runs", &json!({"dir": logs.to_str().unwrap()})));
        assert_eq!(report["total"], 3);
        let runs = report["runs"].as_array().unwrap();
        assert_eq!(runs.len(), 3);
        assert!(runs.iter().any(|r| r["verdict"] == "done"));
        assert!(runs.iter().any(|r| r["verdict"] == "failed"));
        assert!(runs.iter().any(|r| r["error"].is_string()));

        // Default dir (ctx.runs_dir) + missing dir arm.
        std::fs::create_dir_all(&ctx.runs_dir).unwrap();
        write_log(&ctx.runs_dir.join("home.jsonl"), &[]);
        let report = parsed(&call(&ctx, "runs", &json!({})));
        assert_eq!(report["total"], 1);
        assert!(is_err(&call(
            &ctx,
            "runs",
            &json!({"dir": "/nonexistent-dir"})
        )));

        // events — tail + type filter.
        let report = parsed(&call(
            &ctx,
            "events",
            &json!({"path": logs.join("a.jsonl").to_str().unwrap(), "limit": 2}),
        ));
        assert_eq!(report["total"], 3);
        assert_eq!(report["events"].as_array().unwrap().len(), 2);
        assert_eq!(report["events"][0]["type"], "n.done");
        let report = parsed(&call(
            &ctx,
            "events",
            &json!({
                "path": logs.join("a.jsonl").to_str().unwrap(),
                "type": "workflow.done",
            }),
        ));
        assert_eq!(report["total"], 1);
        assert!(is_err(&call(
            &ctx,
            "events",
            &json!({"path": "/nonexistent.jsonl"})
        )));
        assert!(is_err(&call(
            &ctx,
            "events",
            &json!({"path": logs.join("broken.jsonl").to_str().unwrap()})
        )));
        assert!(is_err(&call(&ctx, "events", &json!({}))));

        // explain — verdict, failures, duration.
        let report = parsed(&call(
            &ctx,
            "explain",
            &json!({"path": logs.join("nested").join("b.jsonl").to_str().unwrap()}),
        ));
        assert_eq!(report["verdict"], "failed");
        assert_eq!(report["events"], 3);
        assert_eq!(report["duration_ms"], 200);
        assert_eq!(report["failures"][0]["node"], "x");
        assert_eq!(report["failures"][0]["error"], "boom");
        assert_eq!(report["kinds"]["node.failed"], 1);

        let report = parsed(&call(
            &ctx,
            "explain",
            &json!({"path": logs.join("a.jsonl").to_str().unwrap()}),
        ));
        assert_eq!(report["verdict"], "done");
        assert_eq!(report["terminal"]["type"], "workflow.done");
        assert!(is_err(&call(
            &ctx,
            "explain",
            &json!({"path": "/nonexistent.jsonl"})
        )));
        assert!(is_err(&call(&ctx, "explain", &json!({}))));
    }

    #[test]
    fn unknown_tool_and_malformed_calls() {
        let tmp = tempfile::tempdir().unwrap();
        let ctx = Ctx::under(tmp.path());
        assert!(is_err(&call(&ctx, "nope", &json!({}))));
    }

    #[test]
    fn serve_io_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let ctx = Ctx::under(tmp.path());
        let path = fixture_workflow(tmp.path());
        let input = format!(
            "{{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{{\"protocolVersion\":\"2025-03-26\"}}}}\n\
             {{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}}\n\
             not-json-garbage\n\
             \n\
             {{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"ping\"}}\n\
             {{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"tools/list\"}}\n\
             {{\"jsonrpc\":\"2.0\",\"id\":4,\"method\":\"tools/call\",\"params\":{{\"name\":\"validate\",\"arguments\":{{\"path\":\"{}\"}}}}}}\n\
             {{\"jsonrpc\":\"2.0\",\"id\":5,\"method\":\"bogus/method\"}}\n",
            path.display()
        );
        let mut out = Vec::new();
        serve_io(&ctx, Cursor::new(input.into_bytes()), &mut out).unwrap();
        let text = String::from_utf8(out).unwrap();
        let lines: Vec<Value> = text
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        assert_eq!(lines.len(), 5);
        assert_eq!(lines[0]["result"]["serverInfo"]["name"], "peregrin-mcp");
        assert_eq!(lines[0]["result"]["protocolVersion"], "2025-03-26");
        assert_eq!(lines[1]["id"], 2);
        assert_eq!(lines[2]["result"]["tools"].as_array().unwrap().len(), 9);
        let v = &lines[3]["result"];
        assert!(
            serde_json::from_str::<Value>(v["content"][0]["text"].as_str().unwrap()).unwrap()["ok"]
                .as_bool()
                .unwrap()
        );
        assert_eq!(lines[4]["error"]["code"], -32601);
    }
}
