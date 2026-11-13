//! `peregrin campaign serve` — the multi-campaign daemon (PL-007
//! C1b): N campaign definitions under one process, each driven by
//! its own [`supervise`] loop (own `supersede:` policy, own queue
//! store, own history file), sharing ONE `github-webhook` ingress
//! listener.
//!
//! The shared listener is a [`SharedWebhook`] demux: every hosted
//! campaign's `github-webhook` node resolves to
//! [`SharedWebhookDriver`] via
//! [`campaign::registry_with_ingress_shared`], which registers its
//! `params.repos` watches — repo `full_name` → secret + the
//! campaign's trigger channel — instead of binding the default
//! `127.0.0.1:29173` socket (N solo listeners would collide). The
//! daemon binds the socket once here; verified pushes demux
//! **all-match**: every route whose secret validated the signature
//! gets the event, so two campaigns watching the same repo both
//! trigger. Other ingress drivers (`sturn-poll`) are unaffected —
//! they never bound a socket anyway.
//!
//! Lifecycle: [`serve_campaigns`] returns a [`Served`] handle —
//! `request_shutdown` (the CLI wires it to SIGINT/SIGTERM) stops the
//! listener and sets the flag every supervisor polls; each loop then
//! takes the trigger-disconnect drain path (active run finishes,
//! pending queue drains, listener run cancelled) and `join` collects
//! the results. Queue stores are per-campaign sqlite files
//! (`<queue_store_dir>/<name>.db`) and histories per-campaign JSONL
//! (`<history_dir>/<name>.jsonl`) so campaigns never share state.

use std::collections::{BTreeMap, BTreeSet};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;

use peregrin_core::{Campaign, Event};
use peregrin_driver_github_webhook::SharedWebhook;
use peregrin_engine::Injector;
use peregrin_queue::QueueStore;

use crate::campaign::{self, CampaignError};
use crate::supervisor::{self, SupervisedOptions};

/// One loaded campaign the daemon hosts.
pub struct HostedCampaign {
    /// The campaign's display/file key — `campaign.name` (or the
    /// yaml's file stem when unnamed), already deduplicated by the
    /// caller. Names the per-campaign history/queue-store files.
    pub name: String,
    /// The loaded campaign — must be supervise-shaped (`supersede:`
    /// with ingress nodes for trigger-driven runs; finite campaigns
    /// idle, they just never see a trigger).
    pub campaign: Campaign,
    /// The yaml it loaded from — error messages and future reloads.
    pub source: PathBuf,
}

/// Per-run event sink — `(campaign_name, run_log)`; the serve-mode
/// equivalent of `campaign run`'s run-log output.
pub type RunCallback = Arc<dyn Fn(&str, &[Event]) + Send + Sync>;

/// A campaign supervisor's join handle — drained run logs or the
/// first run failure.
type SupervisorJoin = JoinHandle<Result<Vec<Event>, CampaignError>>;

/// `serve_campaigns` wiring — everything the CLI flags map to.
pub struct ServeConfig {
    /// Shared listener bind address (`--listen`, default
    /// `127.0.0.1:29173`). `…:0` picks an ephemeral port —
    /// [`Served::listener`] reports the resolved address.
    pub listen: String,
    /// Shared hook path (`--path`, default `/hook`) — a literal
    /// axum route.
    pub path: String,
    /// Per-campaign durable history dir — each campaign appends to
    /// `<dir>/<name>.jsonl` (the `campaign run --history` format
    /// `peregrin runs`/`log`/`explain` read). `None` keeps run logs
    /// ephemeral.
    pub history_dir: Option<PathBuf>,
    /// Per-campaign durable queue-store dir — each campaign opens
    /// `<dir>/<name>.db` (sqlite [`QueueStore`]) so pending triggers
    /// and the in-flight marker replay per campaign on restart.
    /// `None` keeps queues in-memory.
    pub queue_store_dir: Option<PathBuf>,
    /// Called with `(campaign name, run log)` for every finished
    /// run, in finish order — the CLI prints it. Runs on each
    /// campaign's supervisor thread.
    pub on_run: Option<RunCallback>,
    /// `RunOptions.secrets` for every campaign's spawned runs — the
    /// explicit record leg of secret resolution (ambient env →
    /// keyring → env file follow via the default stores).
    pub secrets: BTreeMap<String, String>,
    /// This node's runner labels — the `runs_on` matcher's
    /// right-hand side (PL-007 C4). Campaigns whose `runs_on` isn't
    /// fully satisfied are skipped with a warning: a fleet node only
    /// hosts campaigns it declared itself eligible for. The CLI
    /// fills this from `PEREGRIN_RUNNER_LABELS`.
    pub labels: Vec<String>,
}

/// The daemon in flight — [`serve_campaigns`]'s return. `Drop`
/// requests shutdown (the signal flag + listener stop) but can't
/// join — call [`join`](Self::join) for the drained result.
pub struct Served {
    /// The shared listener's bound address — `None` when no hosted
    /// campaign declares `driver: github-webhook` (nothing to demux
    /// to, so no socket is bound).
    pub listener: Option<SocketAddr>,
    /// Hosted campaign names, in serve order.
    pub campaigns: Vec<String>,
    /// The shared demux — exposed for tests/status (`repos()`,
    /// `route_count()`).
    pub demux: Option<Arc<SharedWebhook>>,
    /// Each campaign's external trigger injector — push events here
    /// for out-of-band feeds (tests, future daemon surfaces). The
    /// webhook routes hold their own clones, so dropping these does
    /// NOT end a webhook campaign's trigger stream — that's what
    /// [`request_shutdown`](Self::request_shutdown) is for.
    pub injectors: Vec<Injector>,
    shutdown: Arc<AtomicBool>,
    joins: Vec<(String, SupervisorJoin)>,
}

impl Served {
    /// Stop the shared listener and flag every supervisor to drain —
    /// the SIGTERM path. The listener stops first so no new triggers
    /// can land between the flag and each loop's final channel drain.
    /// Idempotent; safe to call before `join`.
    pub fn request_shutdown(&self) {
        if let Some(demux) = &self.demux {
            demux.stop();
        }
        self.shutdown.store(true, Ordering::Relaxed);
    }

    /// Whether shutdown was requested.
    #[must_use]
    pub fn is_shutdown_requested(&self) -> bool {
        self.shutdown.load(Ordering::Relaxed)
    }

    /// Drain and join every campaign — first requests shutdown (stop
    /// accepting triggers), then joins each supervisor loop in serve
    /// order. Returns the first campaign error after all loops have
    /// finished (a panicking supervisor thread counts as an error).
    ///
    /// # Errors
    /// The first [`CampaignError`] any campaign's loop produced.
    pub fn join(mut self) -> Result<(), CampaignError> {
        self.request_shutdown();
        let mut first_error = None;
        for (name, handle) in std::mem::take(&mut self.joins) {
            match handle.join() {
                Ok(Ok(_)) => {}
                Ok(Err(e)) => {
                    tracing::error!("campaign '{name}' supervisor failed: {e}");
                    if first_error.is_none() {
                        first_error = Some(e);
                    }
                }
                Err(_) => {
                    let e = CampaignError::Run(peregrin_driver::DriverError::new(format!(
                        "campaign '{name}' supervisor thread panicked"
                    )));
                    if first_error.is_none() {
                        first_error = Some(e);
                    }
                }
            }
        }
        match first_error {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }
}

impl Drop for Served {
    fn drop(&mut self) {
        // Best-effort: flag + unbind so nothing keeps running after
        // the handle is gone. Join handles detach — the process (or
        // the test) owns their finish from here.
        self.request_shutdown();
    }
}

/// A filename-safe campaign slug — `[A-Za-z0-9._-]` kept, everything
/// else `_`; empty becomes `campaign`. Used for `<name>.jsonl` /
/// `<name>.db` under the history/queue-store dirs.
fn file_slug(name: &str) -> String {
    let slug: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') {
                c
            } else {
                '_'
            }
        })
        .collect();
    if slug.is_empty() {
        "campaign".to_string()
    } else {
        slug
    }
}

/// Does the campaign declare a `github-webhook` node — i.e. does it
/// need a route in the shared demux (and therefore the listener)?
fn wants_shared_webhook(campaign: &Campaign) -> bool {
    campaign.nodes.iter().any(|n| n.driver == "github-webhook")
}

/// Boot the daemon: open every per-campaign queue store (fail fast,
/// before any listener binds), bind the ONE shared webhook listener
/// when at least one campaign watches repos, then `supervise` each
/// campaign with the shared demux + shutdown flag wired in.
///
/// # Errors
/// [`CampaignError::Serve`] on duplicate campaign names, an empty
/// host list, or the shared listener failing to bind/validate;
/// [`CampaignError::QueueStore`] when a per-campaign store can't be
/// opened; [`CampaignError::Load`]-adjacent failures surface earlier
/// in the caller (loading is the CLI's job).
pub fn serve_campaigns(
    hosted: Vec<HostedCampaign>,
    config: &ServeConfig,
) -> Result<Served, CampaignError> {
    if hosted.is_empty() {
        return Err(CampaignError::Serve("no campaigns to serve".to_string()));
    }
    // Fleet gate (PL-007 C4): a campaign's `runs_on` labels must all
    // be advertised by this node — drop ineligible campaigns up
    // front so their queues/webhook routes never half-exist here.
    let hosted: Vec<HostedCampaign> = hosted
        .into_iter()
        .filter(|entry| {
            if entry.campaign.runs_on_matches(&config.labels) {
                true
            } else {
                tracing::warn!(
                    campaign = %entry.name,
                    source = %entry.source.display(),
                    runs_on = ?entry.campaign.runs_on,
                    labels = ?config.labels,
                    "campaign requires runner labels this node doesn't \
                     advertise — skipping under `campaign serve`"
                );
                false
            }
        })
        .collect();
    if hosted.is_empty() {
        return Err(CampaignError::Serve(
            "no campaigns match this node's runner labels".to_string(),
        ));
    }
    // Names key the per-campaign files — duplicates would share a
    // history/queue db, so reject them up front.
    let mut seen = BTreeSet::new();
    for entry in &hosted {
        if !seen.insert(entry.name.clone()) {
            return Err(CampaignError::Serve(format!(
                "duplicate campaign name '{}'",
                entry.name
            )));
        }
    }

    // Per-campaign durable stores — opened before the listener binds
    // so a bad path fails fast (the `campaign run --queue-store`
    // convention).
    let mut queue_stores: Vec<Option<Arc<dyn QueueStore>>> = Vec::with_capacity(hosted.len());
    if let Some(dir) = &config.queue_store_dir {
        std::fs::create_dir_all(dir)
            .map_err(|e| CampaignError::Serve(format!("queue store dir {}: {e}", dir.display())))?;
    }
    for entry in &hosted {
        let store = match &config.queue_store_dir {
            Some(dir) => {
                let path = dir.join(format!("{}.db", file_slug(&entry.name)));
                Some(
                    Arc::new(peregrin_queue_sqlite::SqliteQueueStore::open(&path)?)
                        as Arc<dyn QueueStore>,
                )
            }
            None => None,
        };
        queue_stores.push(store);
    }
    if let Some(dir) = &config.history_dir {
        std::fs::create_dir_all(dir)
            .map_err(|e| CampaignError::Serve(format!("history dir {}: {e}", dir.display())))?;
    }

    // One shared demux + listener — only when a campaign actually
    // declares `github-webhook` nodes; a fleet of storn-poll/schedule
    // campaigns needs no socket.
    let needs_listener = hosted.iter().any(|e| wants_shared_webhook(&e.campaign));
    let (demux, listener) = if needs_listener {
        let demux = Arc::new(SharedWebhook::new());
        let addr = demux
            .serve(&config.listen, &config.path)
            .map_err(CampaignError::Serve)?;
        (Some(demux), Some(addr))
    } else {
        (None, None)
    };

    let shutdown = Arc::new(AtomicBool::new(false));
    let mut injectors = Vec::with_capacity(hosted.len());
    let mut joins = Vec::with_capacity(hosted.len());
    let mut names = Vec::with_capacity(hosted.len());
    for (entry, queue_store) in hosted.into_iter().zip(queue_stores) {
        let name = entry.name.clone();
        if !campaign::has_ingress_nodes(&entry.campaign) {
            tracing::warn!(
                campaign = %name,
                source = %entry.source.display(),
                "campaign declares no ingress nodes — it will idle \
                 under `campaign serve` (no trigger source)"
            );
        }
        let on_run: Option<supervisor::OnRunDone> = config.on_run.as_ref().map(|callback| {
            let name = name.clone();
            let callback = Arc::clone(callback);
            let wrapped: supervisor::OnRunDone =
                Box::new(move |log: &[Event]| callback(&name, log));
            wrapped
        });
        let options = SupervisedOptions {
            secrets: config.secrets.clone(),
            history: config
                .history_dir
                .as_ref()
                .map(|dir| dir.join(format!("{}.jsonl", file_slug(&name)))),
            queue_store,
            shared_webhook: demux.as_ref().map(Arc::clone),
            shutdown: Some(Arc::clone(&shutdown)),
            on_run,
            campaign_name: Some(name.clone()),
            ..SupervisedOptions::default()
        };
        let run = supervisor::supervise(&entry.campaign, options);
        injectors.push(run.injector);
        joins.push((name.clone(), run.join));
        names.push(name);
    }

    Ok(Served {
        listener,
        campaigns: names,
        demux,
        injectors,
        shutdown,
        joins,
    })
}

/// `campaign.name` or the yaml's file stem — the campaign's display
/// key. The CLI resolves this at load time; kept here so tests and
/// future loaders share the fallback rule.
#[must_use]
pub fn campaign_name(campaign: &Campaign, source: &Path) -> String {
    if !campaign.name.is_empty() {
        return campaign.name.clone();
    }
    source
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("campaign")
        .to_string()
}
