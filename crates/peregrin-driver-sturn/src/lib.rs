//! peregrin-driver-sturn — `driver: sturn-poll`, the sturn WAL-ref
//! poller ingress driver (ADR-0080: "`trigger-sturn` — WAL ref
//! poller, `sturn snap` → run for free"; the bupha `[trigger.sturn]`
//! watcher re-expressed as an ingress node).
//!
//! A sturn-poll node is *ingress*: `invoke` spawns a detached poll
//! thread and returns a `{id}.watching` readiness event immediately
//! (the `github-webhook` driver's `{id}.listening` convention — a
//! blocking invoke would stall the single-threaded run loop forever,
//! audit I-014). The thread diffs git ref heads every `poll_ms` and
//! pushes change events into the live run through the [`Injector`]
//! the driver was built with — the daemon constructs it with
//! `pending.injector.clone()` off the `RunSupervisor`'s `PendingRun`;
//! standalone tests use `peregrin_engine::ingress()` directly.
//!
//! ## Params (bupha `[trigger.sturn]`'s `remote`/`poll_ms`, extended)
//!
//! | Param | Default | Meaning |
//! | --- | --- | --- |
//! | `remote` | `origin` | Remote name or URL handed to `git ls-remote` — ls-remote covers detached, fetch-less watching, and a URL/path remote doesn't even need `repo` to be a repo. `""` = local mode: `git for-each-ref` on the clone itself (bupha's `remote = ""` single-box case). |
//! | `poll_ms` | `1000` | Poll interval in milliseconds. |
//! | `refs` | `refs/heads/*:refs/sturn/wal/*` | Ref patterns — a `:`-separated string or a list of strings (`:` can't appear in a git refname, so it's a safe separator). Patterns pass verbatim to `ls-remote`; in local mode `*`-runs deepen to `**` (`for-each-ref`'s `*` doesn't cross `/` where ls-remote's does — same watch, same refs). The default covers branch heads *and* WAL refs so one node serves ci-gate campaigns filtering `on: [push]` and sturn campaigns filtering `wal.update`. |
//! | `repo` | `.` | Local dir the git calls run in (`~` expands per the `peregrin-git` convention). |
//!
//! ## Events (`source` = the node id)
//!
//! The first *successful* poll seeds the baseline silently — a
//! restarted watcher doesn't replay history; only movements during
//! the watch emit. `old_sha`/`before` are `null` for a ref that
//! appears mid-watch. Ref deletions are not reported (a vanished ref
//! just leaves the diff surface).
//!
//! | Type | When | Payload |
//! | --- | --- | --- |
//! | `{id}.watching` | returned by `invoke` after the poller spawns (or with `dry_run: true` under `options.dry_run` — nothing spawned) | `{remote, repo, clone_url?, refs, poll_ms}` |
//! | `wal.update` | any `refs/sturn/*` head changed | `{remote, repo, ref, sha, old_sha, clone_url?}` + `ws` for `refs/sturn/{wal,ws}/<ws>` |
//! | `push` | a `refs/heads/*` head changed | the `github-webhook` shape `{repo, ref, sha, before, actor, clone_url}` + `old_sha`, `remote`, `branch` — `on: [push]` campaigns subscribe to polled and webhook pushes uniformly; `actor` is `""` (unattested — a polled ref carries no pusher identity) |
//! | `ref.update` | any other watched ref changed | `{remote, repo, ref, sha, old_sha, clone_url?}` |
//! | `driver.stalled` | a poll failed — emitted at streak positions 1, 2, 4, 8… (power-of-two rate limit, never run-killing) | `{error, remote, repo, failures}` |
//! | `driver.recovered` | first success after a failure streak | `{remote, repo, failures}` |
//! | `node.failed` | the poller thread couldn't spawn | `{error, node}` |
//!
//! `repo` is the resolved clone identity (`clone_url` when
//! resolvable, the `repo` dir otherwise) — the `{repo}:{ref}`
//! supersede-key half; `remote` stays the configured param verbatim.
//! `clone_url` is `remote` when it's already a URL/path,
//! `git remote get-url` for a remote name, the `repo` dir in local
//! mode — checkout drivers template it as `params.url`.
//!
//! Lifecycle: one poller per node id — a refire *replaces* the watch
//! (fresh params, fresh baseline). The poller exits on `stop`
//! (driver dropped/handle replaced — checked every ≤25 ms sleep
//! slice) or when the channel closes (`inject` fails after the run
//! ends).

use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use serde_json::{Map, Value};

use peregrin_core::{Event, EventSchema, Node};
use peregrin_driver::{Driver, DriverError, ExecutionContext};
use peregrin_engine::Injector;
use peregrin_git::{Git, SystemGit, expand_path};
use peregrin_template::{
    Context, Effects, EventScope, NodeScope, Options, ParamsScope, Scope, SystemEffects,
    resolve_value_opts,
};

/// `params.remote` default.
const DEFAULT_REMOTE: &str = "origin";
/// `params.refs` default — branch heads + sturn WAL heads, so one
/// node sees pushes *and* snaps.
const DEFAULT_REFS: &[&str] = &["refs/heads/*", "refs/sturn/wal/*"];
/// `params.poll_ms` default — bupha's `[trigger.sturn]` poll_ms.
const DEFAULT_POLL_MS: u64 = 1000;
/// Anything beneath this prefix is sturn state — `wal.update`.
const STURN_PREFIX: &str = "refs/sturn/";
/// Branch heads — `push`.
const HEADS_PREFIX: &str = "refs/heads/";
/// The longest a poller's stop check can lag — the interval sleep is
/// sliced so `Drop`/`replace` joins stay sub-tick.
const STOP_SLICE: Duration = Duration::from_millis(25);

/// One watch target — the resolved `params` shape the poller diffs.
/// Public so a daemon (or a test) can run [`Watch::run`] outside a
/// campaign fire; call [`Watch::resolve_clone_url`] first for
/// `clone_url`-bearing payloads.
#[derive(Clone, Debug)]
pub struct Watch {
    /// Local dir the git calls run in — `~`-expanded `params.repo`.
    pub repo: PathBuf,
    /// `params.remote` verbatim — `""` selects local `for-each-ref`
    /// mode.
    pub remote: String,
    /// Ref patterns handed to `ls-remote`/`for-each-ref`
    /// (deepened).
    pub patterns: Vec<String>,
    /// Poll interval.
    pub poll: Duration,
    /// The clone source emitted in payloads — `None` until
    /// [`Watch::resolve_clone_url`] runs.
    pub clone_url: Option<String>,
}

impl Watch {
    /// Parse a resolved `params` map — absent or mistyped fields fall
    /// back to the documented defaults (bupha's lenient `[trigger.sturn]`
    /// reading, not a hard validation surface).
    #[must_use]
    pub fn from_params(params: &Map<String, Value>) -> Self {
        // `remote` is deliberately *not* non-empty-filtered: `""` is
        // the meaningful local mode, so absent ⇒ origin, present ⇒
        // verbatim.
        let remote = params
            .get("remote")
            .and_then(Value::as_str)
            .unwrap_or(DEFAULT_REMOTE)
            .to_string();
        let repo = params
            .get("repo")
            .and_then(Value::as_str)
            .filter(|dir| !dir.is_empty())
            .map_or_else(|| PathBuf::from("."), |dir| expand_path(Path::new(dir)));
        let poll = params
            .get("poll_ms")
            .and_then(Value::as_u64)
            .filter(|ms| *ms > 0)
            .map_or(
                Duration::from_millis(DEFAULT_POLL_MS),
                Duration::from_millis,
            );
        let mut patterns: Vec<String> = match params.get("refs") {
            Some(Value::String(list)) => list
                .split(':')
                .filter(|part| !part.is_empty())
                .map(str::to_string)
                .collect(),
            Some(Value::Array(items)) => items
                .iter()
                .filter_map(Value::as_str)
                .filter(|part| !part.is_empty())
                .map(str::to_string)
                .collect(),
            _ => Vec::new(),
        };
        if patterns.is_empty() {
            patterns = DEFAULT_REFS.iter().map(|pat| (*pat).to_string()).collect();
        }
        Self {
            repo,
            remote,
            patterns,
            poll,
            clone_url: None,
        }
    }

    /// Fill `clone_url` — the checkout-driver (`params.url`) source:
    /// `remote` verbatim when it's already a URL/path, `git remote
    /// get-url` for a remote name (a miss just omits the field), the
    /// `repo` dir itself in local mode.
    pub fn resolve_clone_url(&mut self, git: &dyn Git) {
        self.clone_url = if self.remote.is_empty() {
            Some(self.repo.display().to_string())
        } else if looks_external(&self.remote) {
            Some(self.remote.clone())
        } else {
            git.run(
                &self.repo,
                &[
                    OsStr::new("remote"),
                    OsStr::new("get-url"),
                    OsStr::new(&self.remote),
                ],
            )
            .ok()
            .map(|url| url.trim().to_string())
            .filter(|url| !url.is_empty())
        };
    }

    /// `{remote, repo, clone_url?}` — the identification fields every
    /// emitted payload carries (`repo` = the resolved clone identity,
    /// the `{repo}:{ref}` supersede-key half).
    fn base_payload(&self) -> Map<String, Value> {
        let repo = self
            .clone_url
            .clone()
            .unwrap_or_else(|| self.repo.display().to_string());
        let mut map = Map::from_iter([
            ("remote".to_string(), Value::String(self.remote.clone())),
            ("repo".to_string(), Value::String(repo)),
        ]);
        if let Some(url) = &self.clone_url {
            map.insert("clone_url".to_string(), Value::String(url.clone()));
        }
        map
    }

    /// Current `{ref → sha}` for the watched patterns —
    /// `git ls-remote <remote> <patterns>` with `repo` as cwd
    /// (fetch-less; a URL/path remote doesn't need `repo` to be a
    /// repo), or `git for-each-ref <patterns>` on the clone in local
    /// mode (patterns pass through [`deepen_globs`] so `*` keeps
    /// `ls-remote`'s depth-agnostic meaning). `ls-remote`'s `HEAD`
    /// and peeled `^{}` lines are filtered; the `BTreeMap` keeps
    /// ticks deterministic.
    ///
    /// # Errors
    /// Whatever `git` reports — the poll loop degrades failures to
    /// `driver.stalled` events rather than dying.
    pub fn heads(&self, git: &dyn Git) -> Result<BTreeMap<String, String>, peregrin_git::Error> {
        if self.remote.is_empty() {
            let mut args: Vec<OsString> = vec![
                OsString::from("for-each-ref"),
                OsString::from("--format=%(refname)%09%(objectname)"),
            ];
            args.extend(
                self.patterns
                    .iter()
                    .map(|pat| OsString::from(deepen_globs(pat))),
            );
            let refs: Vec<&OsStr> = args.iter().map(OsString::as_os_str).collect();
            let out = git.run(&self.repo, &refs)?;
            Ok(out
                .lines()
                .filter_map(|line| line.split_once('\t'))
                .map(|(refname, sha)| (refname.to_string(), sha.to_string()))
                .collect())
        } else {
            let mut args: Vec<OsString> =
                vec![OsString::from("ls-remote"), OsString::from(&self.remote)];
            args.extend(self.patterns.iter().map(OsString::from));
            let refs: Vec<&OsStr> = args.iter().map(OsString::as_os_str).collect();
            let out = git.run(&self.repo, &refs)?;
            Ok(out
                .lines()
                .filter_map(|line| line.split_once('\t'))
                .filter(|(_, refname)| *refname != "HEAD" && !refname.ends_with("^{}"))
                .map(|(sha, refname)| (refname.to_string(), sha.to_string()))
                .collect())
        }
    }

    /// The poll loop — diff [`Watch::heads`] every `self.poll` and
    /// inject the change events under `source` (the node id). Exits
    /// when `stop` flips (checked each tick and inside the sliced
    /// sleep, so a `Drop`/replace join lags ≤ [`STOP_SLICE`]) or when
    /// the channel closes — `inject` failing after the run ended.
    ///
    /// The first successful poll seeds the baseline without emitting.
    /// Poll failures inject `driver.stalled` at streak positions
    /// 1, 2, 4, 8… (power-of-two rate limit) and `driver.recovered`
    /// on the first success after — git trouble never kills the loop.
    pub fn run(&self, git: &dyn Git, injector: &Injector, source: &str, stop: &AtomicBool) {
        let mut baseline: Option<BTreeMap<String, String>> = None;
        let mut failures = 0_u64;
        while !stop.load(Ordering::Relaxed) {
            match self.heads(git) {
                Ok(heads) => {
                    if failures > 0
                        && injector
                            .inject(recovered_event(self, failures, source))
                            .is_err()
                    {
                        return;
                    }
                    failures = 0;
                    if let Some(prev) = &baseline {
                        for (refname, sha) in &heads {
                            if prev.get(refname) != Some(sha)
                                && injector
                                    .inject(change_event(
                                        self,
                                        refname,
                                        sha,
                                        prev.get(refname),
                                        source,
                                    ))
                                    .is_err()
                            {
                                return;
                            }
                        }
                    }
                    baseline = Some(heads);
                }
                Err(error) => {
                    failures += 1;
                    if failures.is_power_of_two()
                        && injector
                            .inject(stalled_event(self, &error, failures, source))
                            .is_err()
                    {
                        return;
                    }
                }
            }
            let deadline = Instant::now() + self.poll;
            while !stop.load(Ordering::Relaxed) && Instant::now() < deadline {
                thread::sleep(STOP_SLICE.min(deadline.saturating_duration_since(Instant::now())));
            }
        }
    }
}

/// `for-each-ref` wildmatch is path-aware — its `*` does not cross
/// `/` (`refs/sturn/*` matches `refs/sturn/x` but not
/// `refs/sturn/wal/dev`) while `ls-remote`'s fnmatch `*` crosses
/// freely. Collapsing every run of `*` into `**` gives local mode
/// the same depth-agnostic semantics `ls-remote` applies in remote
/// mode. (`?`/bracket-class globs keep for-each-ref's path-aware
/// reading — a known divergence for patterns nobody writes.)
fn deepen_globs(pattern: &str) -> String {
    let mut out = String::with_capacity(pattern.len() + 1);
    let mut in_stars = false;
    for ch in pattern.chars() {
        if ch == '*' {
            if !in_stars {
                out.push_str("**");
            }
            in_stars = true;
        } else {
            out.push(ch);
            in_stars = false;
        }
    }
    out
}

/// `remote` is already a clone source (URL, ssh spec, or a path —
/// remote *names* never contain `:`/`@`/`/`/`\\`).
fn looks_external(remote: &str) -> bool {
    remote.contains("://")
        || remote.contains('@')
        || remote.contains('/')
        || remote.contains('\\')
        || remote == "."
        || remote == ".."
}

/// The event one ref movement produces: `wal.update` beneath
/// `refs/sturn/` (a `sturn snap` landing, a claim moving), `push`
/// beneath `refs/heads/` (the `github-webhook` push shape — `before`
/// mirrors `old_sha`, `actor` is unattested-`""`), `ref.update`
/// otherwise. `old` is `None` for a ref that appeared mid-watch —
/// `old_sha`/`before` serialize `null`.
fn change_event(
    watch: &Watch,
    refname: &str,
    sha: &str,
    old: Option<&String>,
    source: &str,
) -> Event {
    let kind = if refname.starts_with(STURN_PREFIX) {
        "wal.update"
    } else if refname.starts_with(HEADS_PREFIX) {
        "push"
    } else {
        "ref.update"
    };
    let mut payload = watch.base_payload();
    payload.insert("ref".to_string(), Value::String(refname.to_string()));
    payload.insert("sha".to_string(), Value::String(sha.to_string()));
    let old_value = old.map_or(Value::Null, |prev| Value::String(prev.clone()));
    payload.insert("old_sha".to_string(), old_value.clone());
    if kind == "push" {
        // The github-webhook push fields verbatim — `actor` stays ""
        // (unattested: a polled ref carries no pusher identity).
        payload.insert("before".to_string(), old_value);
        payload.insert("actor".to_string(), Value::String(String::new()));
    }
    if let Some(ws) = refname
        .strip_prefix("refs/sturn/wal/")
        .or_else(|| refname.strip_prefix("refs/sturn/ws/"))
    {
        payload.insert("ws".to_string(), Value::String(ws.to_string()));
    }
    if let Some(branch) = refname.strip_prefix(HEADS_PREFIX) {
        payload.insert("branch".to_string(), Value::String(branch.to_string()));
    }
    Event::new(kind, Value::Object(payload), source)
}

/// `driver.stalled` — a poll failed; carries the streak length so
/// observers can rate-limit further.
fn stalled_event(watch: &Watch, error: &peregrin_git::Error, failures: u64, source: &str) -> Event {
    let mut payload = watch.base_payload();
    payload.insert("error".to_string(), Value::String(error.to_string()));
    payload.insert("failures".to_string(), Value::from(failures));
    Event::new("driver.stalled", Value::Object(payload), source)
}

/// `driver.recovered` — first successful poll after a failure streak.
fn recovered_event(watch: &Watch, failures: u64, source: &str) -> Event {
    let mut payload = watch.base_payload();
    payload.insert("failures".to_string(), Value::from(failures));
    Event::new("driver.recovered", Value::Object(payload), source)
}

/// The spawn seam — production is [`thread::spawn`]; tests substitute
/// an observing or failing spawner.
pub type Spawn = dyn Fn(Box<dyn FnOnce() + Send>) -> io::Result<JoinHandle<()>> + Send + Sync;

/// A live poller — dropping it flips `stop` (the loop checks it each
/// tick and inside the sliced sleep) and joins the thread, so no
/// poll outlives its driver. The `github-webhook` `ListenerHandle`
/// convention.
struct PollerHandle {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Drop for PollerHandle {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// `driver: sturn-poll`. The driver is *per run*: the daemon clones
/// the run's `PendingRun.injector` and constructs it with
/// [`SturnDriver::new`]. Dropping the driver stops and joins every
/// live poller.
pub struct SturnDriver {
    git: Arc<dyn Git>,
    injector: Injector,
    effects: Arc<dyn Effects>,
    spawn: Arc<Spawn>,
    /// `node id → live poller` — a refire replaces the entry (the old
    /// handle's `Drop` stops and joins it).
    pollers: Mutex<BTreeMap<String, PollerHandle>>,
    output: Vec<EventSchema>,
}

impl SturnDriver {
    /// A driver over the system `git` binary and [`SystemEffects`],
    /// pushing into the run's `injector`.
    #[must_use]
    pub fn new(injector: Injector) -> Self {
        Self::with_parts(
            Arc::new(SystemGit::new()),
            Arc::new(SystemEffects),
            injector,
            Arc::new(|task| Ok(thread::spawn(task))),
        )
    }

    /// Every seam explicit — git fakes, ambient effects, the run's
    /// injector, and the thread spawn (observation/failure).
    #[must_use]
    pub fn with_parts(
        git: Arc<dyn Git>,
        effects: Arc<dyn Effects>,
        injector: Injector,
        spawn: Arc<Spawn>,
    ) -> Self {
        Self {
            git,
            injector,
            effects,
            spawn,
            pollers: Mutex::new(BTreeMap::new()),
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
                    secrets: Some(&ctx.secrets),
                },
            },
        )
        .map_err(|error| DriverError::new(error.to_string()))?;
        Ok(resolved.as_object().cloned().unwrap_or_default())
    }
}

impl Drop for SturnDriver {
    /// Stop and join every live poller — a run's ingress drivers must
    /// not leak threads past their run.
    fn drop(&mut self) {
        self.pollers
            .get_mut()
            .unwrap_or_else(PoisonError::into_inner)
            .clear();
    }
}

/// `input` contract — every field optional (all have defaults);
/// `refs` accepts a string or a list so its spec stays open.
fn input_schema() -> EventSchema {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "remote": {"type": "string"},
            "repo": {"type": "string"},
            "poll_ms": {"type": "integer"},
            "refs": {},
        },
    });
    EventSchema {
        kind: String::new(),
        fields: Map::new(),
        schema: schema.as_object().cloned().unwrap_or_default(),
        annotations: Map::new(),
    }
}

/// `output` contracts — the injected change types plus the
/// `driver.*`/`node.failed` observability types. The `{id}.watching`
/// readiness event is per-node so its contract rides the node's
/// `output:` (the `github-webhook` `*.listening` convention).
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
            "wal.update",
            &[
                ("remote", "string"),
                ("repo", "string"),
                ("ref", "string"),
                ("sha", "string"),
                ("old_sha", "string?"),
                ("clone_url", "string?"),
                ("ws", "string?"),
            ],
        ),
        schema(
            "push",
            &[
                ("remote", "string"),
                ("repo", "string"),
                ("ref", "string"),
                ("sha", "string"),
                ("old_sha", "string?"),
                ("clone_url", "string?"),
                ("before", "string?"),
                ("actor", "string"),
                ("branch", "string?"),
            ],
        ),
        schema(
            "ref.update",
            &[
                ("remote", "string"),
                ("repo", "string"),
                ("ref", "string"),
                ("sha", "string"),
                ("old_sha", "string?"),
                ("clone_url", "string?"),
            ],
        ),
        schema(
            "driver.stalled",
            &[
                ("error", "string"),
                ("remote", "string"),
                ("repo", "string"),
                ("failures", "integer"),
            ],
        ),
        schema(
            "driver.recovered",
            &[
                ("remote", "string"),
                ("repo", "string"),
                ("failures", "integer"),
            ],
        ),
        schema(
            "*.watching",
            &[
                ("remote", "string"),
                ("repo", "string"),
                ("clone_url", "string?"),
                ("refs", "array"),
                ("poll_ms", "integer"),
                ("dry_run", "boolean?"),
            ],
        ),
        schema("node.failed", &[("error", "string"), ("node", "string")]),
    ]
}

impl Driver for SturnDriver {
    fn name(&self) -> &'static str {
        "sturn-poll"
    }

    fn is_ingress(&self) -> bool {
        true
    }

    fn input(&self) -> Option<EventSchema> {
        Some(input_schema())
    }

    fn output(&self) -> &[EventSchema] {
        &self.output
    }

    /// Spawn (or replace) the node's poller and return the
    /// `{id}.watching` readiness event — change events never come
    /// back through `invoke`; they flow through the `Injector` for
    /// the rest of the run.
    fn invoke(
        &self,
        node: &Node,
        triggers: &[Event],
        ctx: &ExecutionContext,
    ) -> Result<Vec<Event>, DriverError> {
        let params = self.resolve_params(node, triggers, ctx)?;
        let mut watch = Watch::from_params(&params);
        watch.resolve_clone_url(&*self.git);
        let kind = format!("{}.watching", node.id);

        let mut ready = watch.base_payload();
        ready.insert(
            "refs".to_string(),
            Value::Array(
                watch
                    .patterns
                    .iter()
                    .map(|pat| Value::String(pat.clone()))
                    .collect(),
            ),
        );
        ready.insert(
            "poll_ms".to_string(),
            Value::from(u64::try_from(watch.poll.as_millis()).unwrap_or(u64::MAX)),
        );

        if ctx.dry_run {
            ready.insert("dry_run".to_string(), Value::Bool(true));
            return Ok(vec![Event::new(kind, Value::Object(ready), &node.id)]);
        }

        let stop = Arc::new(AtomicBool::new(false));
        let git = self.git.clone();
        let injector = self.injector.clone();
        let source = node.id.clone();
        let task = {
            let stop = stop.clone();
            move || watch.run(&*git, &injector, &source, &stop)
        };
        match (self.spawn)(Box::new(task)) {
            Ok(thread) => {
                // Replaces any previous poller for this node — the
                // old handle's Drop signals stop and joins it.
                self.pollers
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .insert(
                        node.id.clone(),
                        PollerHandle {
                            stop,
                            thread: Some(thread),
                        },
                    );
                Ok(vec![Event::new(kind, Value::Object(ready), &node.id)])
            }
            Err(error) => Ok(vec![Event::node_failed(
                &node.id,
                &format!("sturn-poll failed to spawn its poller: {error}"),
            )]),
        }
    }
}
