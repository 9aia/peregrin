//! peregrin-driver-git — `driver: git`, the detached worktree-at-sha
//! driver (ADR-0080 / PL-003 P2). Materializes a detached worktree at
//! an exact sha over the `peregrin-git` lifecycle, runs an *inner*
//! driver inside it, then cleans the worktree up (or keeps it for
//! debug under `params.keep`).
//!
//! `params.url`/`params.repo` is the clone source — a CI campaign
//! templates it off the push event
//! (`url: ${event.payload.origin.clone_url}`). A local path that is
//! already a git repo is used directly; anything else is
//! `git clone --bare`'d into a cache (`<base>/.repos/<key>`) and
//! fetched on reuse. `params.sha` (also event-templated) is
//! `rev-parse --verify`'d to a full commit sha before anything is
//! created; the worktree dest defaults to
//! `<idempotency-key>-<sha12>` under `params.base` (default
//! `~/.local/share/peregrin/worktrees`; `params.dest` overrides).
//!
//! `params.inner` selects the inner driver: a bare driver name
//! (`inner: shell`) or a node fragment
//! (`inner: {driver: shell, params: {...}, emits: [...]}`). Names
//! resolve through the [`DriverLookup`] the driver was built with —
//! the runner's registry at the composition level. The inner node
//! inherits the outer node's id and `on` patterns (wildcard captures
//! still resolve); its params get `cwd` = the worktree (a relative
//! inner `cwd` joins under it) and `env` bindings
//! `PERE_{WORKTREE,SHA,REPO,CLONE_URL}` unless the inner params
//! already declare them.
//!
//! `params.extra_worktrees` materializes *additional* checkouts of
//! the same repo at other refs — `[{ref, dir}]` where `ref` is a
//! branch/tag/ref name (`refs/heads/main`, `main`, a tag; revision
//! syntax like `@{…}`/`~`/`^` is rejected) resolved via
//! `rev-parse --verify` with one best-effort `fetch origin` retry,
//! and `dir` is a relative path under the **primary worktree's
//! parent directory** (`"."` — the default — places the extra at the
//! parent itself). Extras are created *before* the primary: a nested
//! layout puts the primary inside a dir an extra may occupy (the
//! docs-deploy shape — primary `dest: "<run>/docs"` = the docs-branch
//! checkout, extra `refs/heads/main` at `"."` = the `<run>` main
//! checkout its `cd ..` gen scripts expect). Entries are created
//! shallowest-path-first so a container extra lands before anything
//! nested under it, and cleanup destroys the primary before the
//! extras in reverse order — an extra can *contain* the primary's
//! dir, and `worktree remove` is recursive.
//!
//! The inner driver's last event collapses to an `inner_rc`
//! (`payload.rc`, else `1` for a `.failed`-kinded event, else `0`)
//! that `params.emit`'s rc→type map can remap; otherwise `0` emits
//! `emits[0]`/`<node>.done` and nonzero emits `node.failed`, both
//! carrying `{worktree, sha, inner_rc}`. Clone/checkout failures —
//! including `peregrin-git`'s `DestOccupied` — are `node.failed`
//! events; missing params and unknown inner drivers are
//! `DriverError`s. Inner `ctx.emit`/`inject` side channels flow
//! through the shared context unchanged.
//!
//! Credential binding (PL-003 — per-impl, resolution outside the
//! driver): `params.ssh_env`/`params.token_env` name *variables*;
//! each name resolves against the run's secrets record first, then
//! the ambient environment, and a miss is audited like any other
//! secret miss. Resolved values reach git as
//! `-c core.sshCommand=<v>` / `-c http.extraHeader=Authorization:
//! Basic <b64(x:v)>` on clone/fetch — argv-scoped, never persisted
//! into the cached clone's config. Basic covers PATs and `gho_`
//! OAuth tokens; `bearer` is rejected by smart-HTTP for OAuth.

use std::ffi::{OsStr, OsString};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use serde_json::{Map, Value};

use peregrin_core::{Event, EventSchema, Node, js_truthy};
use peregrin_driver::{Driver, DriverError, ExecutionContext};
// `DriverLookup` moved to the port crate when `council` became the
// second consumer — re-exported so `peregrin_driver_git::DriverLookup`
// keeps resolving.
pub use peregrin_driver::DriverLookup;
use peregrin_git::{Git, SystemGit, Worktrees, expand_path};
use peregrin_template::{
    Context, Effects, EventScope, NodeScope, Options, ParamsScope, Scope, SystemEffects,
    resolve_value_opts,
};

/// Default `params.base` — worktrees and the clone cache live under
/// it; `~` expands per the `peregrin-git` convention.
const DEFAULT_BASE: &str = "~/.local/share/peregrin/worktrees";

/// Refspec for cache-refresh fetches. `git clone --bare` writes no
/// `remote.origin.fetch` (observed on git 2.56), so a bare
/// `fetch origin` only updates FETCH_HEAD — refs never move and
/// `rev-parse` can't see new commits. Passing the refspec on argv
/// works for caches cloned before any config repair.
const FETCH_REFSPEC: &str = "+refs/heads/*:refs/heads/*";

/// A stable, filesystem-safe cache key for a clone source —
/// `<slug>-<fnv8>`, where the slug keeps the source readable in
/// `list()` output and the hash keeps distinct sources distinct.
#[must_use]
pub fn cache_key(source: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in source.as_bytes() {
        hash = (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3);
    }
    let slug: String = source
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .take(24)
        .collect();
    format!("{slug}-{:08x}", hash & 0xffff_ffff)
}

/// `driver: git`. `git` backs both the [`Worktrees`] lifecycle and
/// the driver's own rev-parse/clone/fetch calls; `lookup` resolves
/// inner driver names; `effects` is the ambient-env fallback for
/// credential names.
pub struct GitDriver<G: Git + Clone = SystemGit> {
    git: G,
    lookup: Arc<dyn DriverLookup>,
    effects: Arc<dyn Effects>,
    output: Vec<EventSchema>,
}

impl GitDriver<SystemGit> {
    /// A driver over the system `git` binary and [`SystemEffects`].
    #[must_use]
    pub fn new(lookup: impl DriverLookup + 'static) -> Self {
        Self::with_parts(SystemGit::new(), Arc::new(lookup), Arc::new(SystemEffects))
    }
}

impl<G: Git + Clone> GitDriver<G> {
    /// A driver over a custom [`Git`] (fakes, hermetic toolchains).
    #[must_use]
    pub fn with_git(git: G, lookup: impl DriverLookup + 'static) -> Self {
        Self::with_parts(git, Arc::new(lookup), Arc::new(SystemEffects))
    }

    /// A driver with every seam explicit (tests).
    #[must_use]
    pub fn with_parts(git: G, lookup: Arc<dyn DriverLookup>, effects: Arc<dyn Effects>) -> Self {
        Self {
            git,
            lookup,
            effects,
            output: output_schemas(),
        }
    }

    /// Re-resolve `node.params` the way the sibling drivers do inside
    /// `invoke` (the runner resolves too — the driver-level pass keeps
    /// direct invocations working). `ctx` lends its strict flag,
    /// capability map, and shared missing-secret ledger.
    fn resolve_params(
        &self,
        node: &Node,
        triggers: &[Event],
        ctx: &ExecutionContext,
    ) -> Result<Map<String, Value>, DriverError> {
        let tctx = Context {
            strict: ctx.strict,
            missing_secrets: ctx.missing_secrets.clone(),
            capabilities: (*ctx.capabilities).clone(),
            effects: &*self.effects,
        };
        let state_view = |key: &str| ctx.state.borrow().get(key);
        let secret_view = |name: &str| ctx.resolve_secret(name);
        let resolved = resolve_value_opts(
            &node.params,
            &tctx,
            &Options {
                node: Some(node),
                functions: &[],
                scope: Scope {
                    event: triggers.first().map(|event| EventScope {
                        event,
                        patterns: &node.on,
                    }),
                    config: Some(&ctx.config),
                    params: ParamsScope::Resolved(&ctx.params),
                    state: Some(&state_view),
                    node: Some(NodeScope {
                        id: &node.id,
                        fires: ctx.fires,
                    }),
                    secrets: Some(&secret_view),
                },
            },
        )
        .map_err(|error| DriverError::new(error.to_string()))?;
        Ok(resolved.as_object().cloned().unwrap_or_default())
    }

    /// `git <creds as -c k=v> <args>` — credential config rides argv
    /// for exactly this call so clone/fetch authenticate without the
    /// `Git` trait needing an env channel.
    fn run_with_creds(
        &self,
        dir: &Path,
        creds: &[String],
        args: &[&OsStr],
    ) -> Result<String, peregrin_git::Error> {
        let mut argv: Vec<OsString> = Vec::with_capacity(args.len() + creds.len() * 2);
        for cred in creds {
            argv.push(OsString::from("-c"));
            argv.push(OsString::from(cred));
        }
        argv.extend(args.iter().map(OsString::from));
        let refs: Vec<&OsStr> = argv.iter().map(OsString::as_os_str).collect();
        self.git.run(dir, &refs)
    }

    /// Local repo path, or a cached bare clone of the remote source.
    /// An existing dir that git doesn't recognize is a hard error —
    /// silently cloning *over* a path the campaign typo'd would hide
    /// the mistake. Cache hits fetch best-effort (the rev-parse gate
    /// below decides whether the sha is present).
    fn materialize_repo(
        &self,
        source: &str,
        worktrees: &Worktrees<G>,
        creds: &[String],
    ) -> Result<PathBuf, String> {
        let path = expand_path(Path::new(source));
        if path.is_dir() {
            self.git
                .run(&path, &[OsStr::new("rev-parse"), OsStr::new("--git-dir")])
                .map_err(|error| {
                    format!("'{source}' is a directory but not a git repository: {error}")
                })?;
            return Ok(path);
        }
        let repos = worktrees.base().join(".repos");
        std::fs::create_dir_all(&repos)
            .map_err(|error| format!("clone cache {}: {error}", repos.display()))?;
        let cache = repos.join(cache_key(source));
        if cache.is_dir() {
            let _ = self.run_with_creds(
                &cache,
                creds,
                &[
                    OsStr::new("fetch"),
                    OsStr::new("--prune"),
                    OsStr::new("origin"),
                    OsStr::new(FETCH_REFSPEC),
                ],
            );
            return Ok(cache);
        }
        if cache.exists() {
            return Err(format!(
                "clone cache {} is occupied by a non-repo path",
                cache.display()
            ));
        }
        self.run_with_creds(
            &repos,
            creds,
            &[
                OsStr::new("clone"),
                OsStr::new("--bare"),
                OsStr::new(source),
                cache.as_os_str(),
            ],
        )
        .map_err(|error| format!("clone '{source}' failed: {error}"))?;
        Ok(cache)
    }

    /// Resolve an `extra_worktrees[].ref` to a full commit sha in
    /// `repo`. `rev-parse --verify` first; on a miss, one best-effort
    /// `fetch origin` (credential args ride argv like clone/fetch) and
    /// a retry — covers refs the cached clone hasn't seen yet. A repo
    /// without `origin` just fails the retry and reports the
    /// rev-parse error.
    fn resolve_ref(
        &self,
        repo: &Path,
        reference: &str,
        creds: &[String],
    ) -> Result<String, String> {
        let verify = format!("{reference}^{{commit}}");
        let rev = [
            OsStr::new("rev-parse"),
            OsStr::new("--verify"),
            OsStr::new(&verify),
        ];
        match self.git.run(repo, &rev) {
            Ok(out) => Ok(out.trim().to_string()),
            Err(first) => {
                let _ = self.run_with_creds(
                    repo,
                    creds,
                    &[
                        OsStr::new("fetch"),
                        OsStr::new("--prune"),
                        OsStr::new("origin"),
                        OsStr::new(FETCH_REFSPEC),
                    ],
                );
                self.git
                    .run(repo, &rev)
                    .map(|out| out.trim().to_string())
                    .map_err(|_| {
                        format!(
                            "extra worktree ref '{reference}' not resolvable in {}: {first}",
                            repo.display()
                        )
                    })
            }
        }
    }
}

/// `params.inner` — a bare driver name or a node fragment
/// `{driver, handler?, params?, emits?}`.
struct InnerSpec {
    driver: String,
    handler: String,
    params: Map<String, Value>,
    emits: Vec<String>,
}

/// Parse `params.inner`; `None` is the missing-param error.
fn parse_inner(raw: Option<&Value>) -> Result<InnerSpec, String> {
    match raw {
        Some(Value::String(name)) if !name.is_empty() => Ok(InnerSpec {
            driver: name.clone(),
            handler: String::new(),
            params: Map::new(),
            emits: Vec::new(),
        }),
        Some(Value::Object(map)) => {
            let driver = map
                .get("driver")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if driver.is_empty() {
                return Err("object 'params.inner' missing 'driver'".to_string());
            }
            Ok(InnerSpec {
                driver: driver.to_string(),
                handler: map
                    .get("handler")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                params: map
                    .get("params")
                    .and_then(Value::as_object)
                    .cloned()
                    .unwrap_or_default(),
                emits: map
                    .get("emits")
                    .and_then(Value::as_array)
                    .map(|items| {
                        items
                            .iter()
                            .filter_map(|item| item.as_str().map(str::to_string))
                            .collect()
                    })
                    .unwrap_or_default(),
            })
        }
        _ => {
            Err("missing or invalid 'params.inner' (driver name or {driver, params?})".to_string())
        }
    }
}

/// A non-empty string param, else `None`.
fn non_empty_str<'a>(params: &'a Map<String, Value>, key: &str) -> Option<&'a str> {
    params
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
}

/// `ssh_env`/`token_env` accept one name or a list of names.
fn env_names(raw: Option<&Value>) -> Vec<&str> {
    match raw {
        Some(Value::String(name)) => vec![name.as_str()],
        Some(Value::Array(items)) => items.iter().filter_map(Value::as_str).collect(),
        _ => Vec::new(),
    }
}

/// One `params.extra_worktrees` entry — an additional checkout of the
/// same repo at `reference`, placed at `dir` relative to the primary
/// worktree's parent directory (`"."` = the parent itself).
struct ExtraWorktree {
    reference: String,
    dir: String,
}

/// A ref name safe to hand `rev-parse`: branch/tag/`refs/…` names and
/// full shas only — no leading `-` (flag injection into the argv),
/// no `..`, and none of the revision syntax (`@{`, `~`, `^`, `:`,
/// globs) that could resolve outside the intended ref.
fn valid_ref(reference: &str) -> bool {
    !reference.is_empty()
        && !reference.starts_with('-')
        && !reference.starts_with('.')
        && !reference.contains("..")
        && reference
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '/' | '-'))
}

/// A relative dir safe to join under the run root: no absolute path,
/// no `..` escape, no prefixes — `.`/`./x`-style `CurDir` components
/// are fine (they collapse away at the join).
fn valid_rel_dir(dir: &str) -> bool {
    !dir.is_empty()
        && Path::new(dir)
            .components()
            .all(|c| matches!(c, Component::Normal(_) | Component::CurDir))
}

/// Parse `params.extra_worktrees` — `[{ref, dir?}]`; `dir` defaults
/// to `"."` (the primary worktree's parent). Malformed shapes are
/// param errors, like `params.inner`.
fn parse_extra_worktrees(raw: Option<&Value>) -> Result<Vec<ExtraWorktree>, String> {
    let Some(value) = raw else {
        return Ok(Vec::new());
    };
    let items = value
        .as_array()
        .ok_or("params.extra_worktrees must be an array")?;
    let mut extras = Vec::with_capacity(items.len());
    for item in items {
        let map = item
            .as_object()
            .ok_or("params.extra_worktrees entries must be objects")?;
        let reference = map
            .get("ref")
            .and_then(Value::as_str)
            .filter(|r| valid_ref(r))
            .ok_or("params.extra_worktrees entry missing a valid 'ref'")?;
        let dir = map.get("dir").and_then(Value::as_str).unwrap_or(".");
        if !valid_rel_dir(dir) {
            return Err(format!(
                "params.extra_worktrees entry 'dir' {dir:?} must be a relative path without '..'"
            ));
        }
        extras.push(ExtraWorktree {
            reference: reference.to_string(),
            dir: dir.to_string(),
        });
    }
    Ok(extras)
}

/// Resolve credential names to `-c` config args. Each name resolves
/// through `ctx.resolve_secret` — the run's secrets record, the
/// ambient environment, then the fallback stores — with the
/// injected `effects.env` as a last test channel. The value's
/// *origin* stays at the composition level; the driver only binds
/// the declared name. A miss is audited through `ctx.get_secret`
/// (strict-mode `missing_secrets`), then skipped.
fn cred_args(
    params: &Map<String, Value>,
    ctx: &ExecutionContext,
    effects: &dyn Effects,
) -> Vec<String> {
    let resolve = |name: &str| -> Option<String> {
        let value = ctx
            .resolve_secret(name)
            .or_else(|| effects.env(name))
            .filter(|value| !value.is_empty());
        if value.is_none() {
            // Audited miss — records under strict, "" otherwise.
            let _ = ctx.get_secret(name);
        }
        value
    };
    let mut args = Vec::new();
    for name in env_names(params.get("ssh_env")) {
        if let Some(value) = resolve(name) {
            args.push(format!("core.sshCommand={value}"));
        }
    }
    for name in env_names(params.get("token_env")) {
        if let Some(value) = resolve(name) {
            args.push(format!(
                "http.extraHeader=Authorization: Basic {}",
                basic_b64(&format!("x:{value}"))
            ));
        }
    }
    args
}

/// `Authorization: Basic` userinfo — `x:<token>` basic-auth works for
/// PATs and `gho_` OAuth tokens alike; `bearer` is rejected by
/// smart-HTTP for OAuth tokens.
fn basic_b64(input: &str) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.as_bytes().chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        out.push(A[(b[0] >> 2) as usize] as char);
        out.push(A[((b[0] << 4 | b[1] >> 4) & 63) as usize] as char);
        out.push(if chunk.len() > 1 {
            A[((b[1] << 2 | b[2] >> 6) & 63) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            A[(b[2] & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}

/// Filesystem-safe worktree dest name: `<idempotency-key>-<sha12>`
/// (falling back to the node id, then `run`), so a retry attempt or a
/// different sha never collides with a live record.
fn default_dest(node: &Node, ctx: &ExecutionContext, sha: &str) -> String {
    let raw = if ctx.idempotency_key.is_empty() {
        node.id.as_str()
    } else {
        ctx.idempotency_key.as_str()
    };
    let slug: String = raw
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') {
                c
            } else {
                '-'
            }
        })
        .take(48)
        .collect();
    let slug = slug.trim_matches('-');
    let short: String = sha.chars().take(12).collect();
    if slug.is_empty() {
        format!("run-{short}")
    } else {
        format!("{slug}-{short}")
    }
}

/// Collapse the inner driver's events to an rc: the last event's
/// `payload.rc`, else `1` when it is a `.failed`/`node.failed`-kinded
/// event, else `0`. No events means the inner driver had nothing to
/// report — success.
fn inner_rc(events: &[Event]) -> i64 {
    let Some(last) = events.last() else {
        return 0;
    };
    if let Some(rc) = last.payload.get("rc").and_then(Value::as_i64) {
        return rc;
    }
    i64::from(last.kind == "node.failed" || last.kind.ends_with(".failed"))
}

/// `emit[rc]` then `emit.default` — the `params.emit` convention.
fn emit_mapped(emit: &Map<String, Value>, rc: i64) -> Option<String> {
    emit.get(&rc.to_string())
        .or_else(|| emit.get("default"))
        .and_then(Value::as_str)
        .map(str::to_string)
}

/// `owner/name` out of a clone source — https, ssh (`git@host:`),
/// scp-style, or a local path (basename's parent/name). Trailing
/// `.git` and `/` are stripped; an unparseable source yields `""`
/// (the caller's `params.repo` override remains the accurate path).
fn repo_from_url(source: &str) -> String {
    let trimmed = source.trim_end_matches('/').trim_end_matches(".git");
    let after_scheme = trimmed.rsplit_once("://").map_or(trimmed, |(_, rest)| rest);
    let path = match after_scheme.rsplit_once(':') {
        Some((host, rest)) if !rest.contains('/') => format!("{host}/{rest}"),
        Some((_, rest)) => rest.to_string(),
        None => after_scheme.to_string(),
    };
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    if segments.len() >= 2 {
        format!(
            "{}/{}",
            segments[segments.len() - 2],
            segments[segments.len() - 1]
        )
    } else {
        segments.last().map_or(String::new(), |s| (*s).to_string())
    }
}

/// The emitted `node.failed` for materialization/inner failures —
/// the event-path failure (driver errors are reserved for params
/// that couldn't produce a run at all).
fn failed_event(node: &Node, error: String, extra: &Map<String, Value>) -> Event {
    let mut payload = extra.clone();
    payload.insert("error".to_string(), Value::String(error));
    payload.insert("node".to_string(), Value::String(node.id.clone()));
    Event::new("node.failed", Value::Object(payload), &node.id)
}

/// `input` contract — `sha` + `inner` required; `url`/`repo` are
/// either-or (the driver errors when both are absent). `inner` and
/// the credential-name params accept several shapes, so their specs
/// stay open (`{}` — the local validator only checks string-typed
/// `type` constraints).
fn input_schema() -> EventSchema {
    let schema = serde_json::json!({
        "type": "object",
        "required": ["sha", "inner"],
        "properties": {
            "url": {"type": "string"},
            "repo": {"type": "string"},
            "sha": {"type": "string"},
            "inner": {},
            "base": {"type": "string"},
            "dest": {"type": "string"},
            "extra_worktrees": {},
            "keep": {},
            "emit": {"type": "object"},
            "ssh_env": {},
            "token_env": {},
        },
    });
    EventSchema {
        kind: String::new(),
        fields: Map::new(),
        schema: schema.as_object().cloned().unwrap_or_default(),
        annotations: Map::new(),
    }
}

/// `output` contracts — the success type is `emits[0]`/`<node>.done`
/// (node-scoped), matched via `*.done`; `node.failed` covers every
/// failure path.
fn output_schemas() -> Vec<EventSchema> {
    let schema = |kind: &str, fields: &[(&str, &str)]| EventSchema {
        kind: kind.to_string(),
        fields: fields
            .iter()
            .map(|(k, t)| ((*k).to_string(), Value::String((*t).to_string())))
            .collect(),
        schema: Map::new(),
        annotations: Map::new(),
    };
    vec![
        schema(
            "*.done",
            &[
                ("worktree", "string"),
                ("sha", "string"),
                ("inner_rc", "integer"),
            ],
        ),
        schema(
            "node.failed",
            &[
                ("error", "string"),
                ("node", "string"),
                ("worktree", "string?"),
                ("sha", "string?"),
                ("inner_rc", "integer?"),
            ],
        ),
    ]
}

impl<G: Git + Clone> Driver for GitDriver<G> {
    fn name(&self) -> &'static str {
        "git"
    }

    fn input(&self) -> Option<EventSchema> {
        Some(input_schema())
    }

    fn output(&self) -> &[EventSchema] {
        &self.output
    }

    fn invoke(
        &self,
        node: &Node,
        triggers: &[Event],
        ctx: &ExecutionContext,
    ) -> Result<Vec<Event>, DriverError> {
        let params = self.resolve_params(node, triggers, ctx)?;
        let source = non_empty_str(&params, "url")
            .or_else(|| non_empty_str(&params, "repo"))
            .map(str::to_string)
            .ok_or_else(|| {
                DriverError::new(format!(
                    "git node '{}' missing 'params.url' or 'params.repo'",
                    node.id
                ))
            })?;
        let sha = non_empty_str(&params, "sha")
            .map(str::to_string)
            .ok_or_else(|| {
                DriverError::new(format!("git node '{}' missing 'params.sha'", node.id))
            })?;
        let inner = parse_inner(params.get("inner"))
            .map_err(|error| DriverError::new(format!("git node '{}': {error}", node.id)))?;
        let extras = parse_extra_worktrees(params.get("extra_worktrees"))
            .map_err(|error| DriverError::new(format!("git node '{}': {error}", node.id)))?;
        let inner_driver = self.lookup.driver(&inner.driver).ok_or_else(|| {
            DriverError::new(format!(
                "git node '{}' has no driver registered for inner '{}'",
                node.id, inner.driver
            ))
        })?;

        let done_type = node
            .emits
            .first()
            .cloned()
            .unwrap_or_else(|| format!("{}.done", node.id));

        if ctx.dry_run {
            return Ok(vec![Event::new(
                done_type,
                serde_json::json!({ "dry_run": true, "url": source, "sha": sha }),
                &node.id,
            )]);
        }

        let base = non_empty_str(&params, "base").unwrap_or(DEFAULT_BASE);
        let worktrees = Worktrees::with_git(base, self.git.clone());
        let creds = cred_args(&params, ctx, &*self.effects);

        // --- materialize: repo → resolved sha → detached worktree ----
        // `repo` echoes the logical owner/name (`params.repo`, else
        // derived from the clone source) into every emitted payload so
        // downstream routing can carry it — e.g. ci-gate's failmap
        // re-types `node.failed` as `.failed.<sha>.<repo>`.
        let repo_label =
            non_empty_str(&params, "repo").map_or_else(|| repo_from_url(&source), str::to_string);
        let facts = |worktree: &Path| {
            Map::from_iter([
                (
                    "worktree".to_string(),
                    Value::String(worktree.display().to_string()),
                ),
                ("sha".to_string(), Value::String(sha.clone())),
                ("repo".to_string(), Value::String(repo_label.clone())),
            ])
        };
        let fail = |error: String| -> DriverError { DriverError::new(error) };
        let repo = match self.materialize_repo(&source, &worktrees, &creds) {
            Ok(repo) => repo,
            Err(error) => {
                return Ok(vec![failed_event(node, error, &facts(Path::new("")))]);
            }
        };
        let resolved = match self.git.run(
            &repo,
            &[
                OsStr::new("rev-parse"),
                OsStr::new("--verify"),
                OsStr::new(&format!("{sha}^{{commit}}")),
            ],
        ) {
            Ok(out) => out.trim().to_string(),
            Err(error) => {
                return Ok(vec![failed_event(
                    node,
                    format!("sha '{sha}' not resolvable in {source}: {error}"),
                    &facts(Path::new("")),
                )]);
            }
        };
        let dest = non_empty_str(&params, "dest")
            .map_or_else(|| default_dest(node, ctx, &resolved), str::to_string);

        // --- extra worktrees: same repo, other refs ------------------
        // `dir` resolves under the primary's parent dir — the "run
        // root" for a nested `dest` like `<run>/docs`. Ref resolution
        // happens before anything is created; creation is
        // shallowest-path-first so a container extra (the run root
        // itself) lands before anything nested under it, and ALWAYS
        // before the primary — a primary created first would leave
        // the run root non-empty and `worktree add` would refuse it.
        let mut prepared: Vec<(PathBuf, String)> = Vec::with_capacity(extras.len());
        if !extras.is_empty() {
            // Mirror `Worktrees::dest_path`: `~`-expand, relative joins
            // under the (already absolutized) base.
            let expanded = expand_path(Path::new(&dest));
            let dest_abs = if expanded.is_absolute() {
                expanded
            } else {
                worktrees.base().join(expanded)
            };
            let run_root = dest_abs
                .parent()
                .map_or_else(|| worktrees.base().to_path_buf(), Path::to_path_buf);
            for extra in &extras {
                match self.resolve_ref(&repo, &extra.reference, &creds) {
                    Ok(extra_sha) => {
                        let mut path = run_root.clone();
                        for component in Path::new(&extra.dir).components() {
                            if let Component::Normal(part) = component {
                                path.push(part);
                            }
                        }
                        prepared.push((path, extra_sha));
                    }
                    Err(error) => {
                        return Ok(vec![failed_event(node, error, &facts(Path::new("")))]);
                    }
                }
            }
            // Parents before children regardless of yaml order.
            prepared.sort_by_key(|(path, _)| path.components().count());
        }
        let mut created_extras: Vec<peregrin_git::Worktree> = Vec::with_capacity(prepared.len());
        for (path, extra_sha) in &prepared {
            match worktrees.create(&repo, extra_sha, path) {
                Ok(extra) if extra.sha == *extra_sha => created_extras.push(extra),
                Ok(extra) => {
                    // Adopted a foreign checkout at the wrong sha —
                    // same contract as the primary below: leave it
                    // (and its record) for `Worktrees::cleanup`, but
                    // destroy the extras this run did create.
                    for made in created_extras.iter().rev() {
                        let _ = worktrees.destroy(&made.path);
                    }
                    return Ok(vec![failed_event(
                        node,
                        format!(
                            "worktree {} is detached at {}, expected {extra_sha}",
                            extra.path.display(),
                            extra.sha
                        ),
                        &facts(&extra.path),
                    )]);
                }
                Err(error) => {
                    for made in created_extras.iter().rev() {
                        let _ = worktrees.destroy(&made.path);
                    }
                    return Ok(vec![failed_event(
                        node,
                        format!("extra worktree create failed: {error}"),
                        &facts(Path::new("")),
                    )]);
                }
            }
        }

        let worktree = match worktrees.create(&repo, &resolved, &dest) {
            Ok(worktree) => worktree,
            Err(error) => {
                // The primary never landed — the extras are ours and
                // safe to take back down (nothing foreign can be
                // nested inside them yet).
                for made in created_extras.iter().rev() {
                    let _ = worktrees.destroy(&made.path);
                }
                return Ok(vec![failed_event(
                    node,
                    format!("worktree create failed: {error}"),
                    &facts(Path::new("")),
                )]);
            }
        };
        if worktree.sha != resolved {
            // `create` adopted a foreign checkout at the dest — leave
            // it (and its record) for `Worktrees::cleanup` rather than
            // deleting something peregrin-git didn't create. The
            // extras stay recorded too: one may BE the run root
            // containing this foreign checkout, so taking it down
            // synchronously could delete the adopted dir with it.
            return Ok(vec![failed_event(
                node,
                format!(
                    "worktree {} is detached at {}, expected {resolved}",
                    worktree.path.display(),
                    worktree.sha
                ),
                &facts(&worktree.path),
            )]);
        }

        // --- inner node: same identity, inner driver + worktree cwd --
        let mut inner_params = inner.params.clone();
        let cwd = inner_params
            .get("cwd")
            .and_then(Value::as_str)
            .filter(|dir| !dir.is_empty());
        let cwd = match cwd {
            Some(dir) if !Path::new(dir).is_absolute() => worktree.path.join(dir),
            _ => worktree.path.clone(),
        };
        inner_params.insert("cwd".to_string(), Value::String(cwd.display().to_string()));
        // `env` bindings injected under whatever the inner params
        // declare — a non-object `env` is replaced outright.
        let mut env = inner_params
            .get("env")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        for (key, value) in [
            ("PERE_WORKTREE", worktree.path.display().to_string()),
            ("PERE_SHA", resolved.clone()),
            ("PERE_REPO", worktree.repo.display().to_string()),
            ("PERE_CLONE_URL", source.clone()),
        ] {
            env.entry(key.to_string()).or_insert(Value::String(value));
        }
        inner_params.insert("env".to_string(), Value::Object(env));
        let mut inner_node = node.clone();
        inner_node.driver.clone_from(&inner.driver);
        inner_node.handler.clone_from(&inner.handler);
        inner_node.params = Value::Object(inner_params);
        if !inner.emits.is_empty() {
            inner_node.emits.clone_from(&inner.emits);
        }

        let inner_result = inner_driver.invoke(&inner_node, triggers, ctx);

        // --- cleanup: destroy unless kept-for-debug -----------------
        // Primary first, then extras in reverse creation order
        // (deepest first): an extra can be the run root CONTAINING
        // the primary's dir, and `worktree remove` is recursive —
        // removing the container first would take the primary's
        // checkout with it. Every entry is attempted; the first
        // error is reported.
        let keep = params.get("keep").is_some_and(js_truthy);
        let mut cleanup_error = None;
        let mut teardown = |path: &Path| {
            let result = if keep {
                worktrees.keep(path).map(|_| ())
            } else {
                worktrees.destroy(path)
            };
            if let Err(error) = result
                && cleanup_error.is_none()
            {
                cleanup_error = Some(error.to_string());
            }
        };
        teardown(&worktree.path);
        for extra in created_extras.iter().rev() {
            teardown(&extra.path);
        }
        let facts = facts(&worktree.path);
        if let Some(error) = cleanup_error {
            return Ok(vec![failed_event(
                node,
                format!("worktree cleanup failed: {error}"),
                &facts,
            )]);
        }
        let events = inner_result.map_err(|error| {
            fail(format!(
                "git node '{}' inner driver '{}' failed: {error}",
                node.id, inner.driver
            ))
        })?;

        // --- emit: params.emit rc-map, else rc → done/failed --------
        let rc = inner_rc(&events);
        let mut facts = facts;
        facts.insert(
            "inner_rc".to_string(),
            Value::Number(serde_json::Number::from(rc)),
        );
        let emit_type = params
            .get("emit")
            .and_then(Value::as_object)
            .and_then(|emit| emit_mapped(emit, rc));
        if let Some(mapped) = emit_type {
            return Ok(vec![Event::new(mapped, Value::Object(facts), &node.id)]);
        }
        if rc == 0 {
            return Ok(vec![Event::new(done_type, Value::Object(facts), &node.id)]);
        }
        Ok(vec![failed_event(
            node,
            format!("inner driver '{}' exited rc {rc}", inner.driver),
            &facts,
        )])
    }
}
