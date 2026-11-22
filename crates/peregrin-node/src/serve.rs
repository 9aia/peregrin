//! `peregrin campaign serve` — the multi-campaign daemon (PL-007
//! C1b): N campaign definitions under one process, each driven by
//! its own [`supervise`](crate::supervisor::supervise) loop (own `supersede:` policy, own queue
//! store, own history file), sharing ONE `github-webhook` ingress
//! listener.
//!
//! The shared listener is a [`SharedWebhook`] demux: every hosted
//! campaign's `github-webhook` node resolves to
//! [`SharedWebhookDriver`](peregrin_driver_github_webhook::SharedWebhookDriver) via
//! [`campaign::registry_with_ingress_shared`], which registers its
//! `params.repos` watches — repo `full_name` → secret + the
//! campaign's trigger channel — instead of binding the default
//! `127.0.0.1:29173` socket (N solo listeners would collide). The
//! daemon binds the socket once here — **unconditionally**: a fleet
//! with no webhook flow today may gain one via the flows API
//! ([`flows`], ADR-0093), and a later-registered flow must still
//! receive deliveries. Verified pushes demux **all-match**: every
//! route whose secret validated the signature gets the event, so
//! two campaigns watching the same repo both trigger. Other ingress
//! drivers (`sturn-poll`) are unaffected — they never bound a
//! socket anyway.
//!
//! Lifecycle: [`serve_campaigns`] returns a [`Served`] handle —
//! `request_shutdown` (the CLI wires it to SIGINT/SIGTERM) stops the
//! listener and flags every flow's own shutdown; each supervisor
//! then takes the trigger-disconnect drain path (active run
//! finishes, pending queue drains, listener run cancelled) and
//! [`Served::join`] collects the results via
//! [`FlowHost::join_all`]. Queue stores are per-campaign sqlite
//! files (`<queue_store_dir>/<name>.db`) and histories per-campaign
//! JSONL (`<history_dir>/<name>.jsonl`) so campaigns never share
//! state.
//!
//! The same socket also serves the management API: `POST
//! /api/events/inject` ([`inject`], the typed-event feed) and the
//! `/api/flows*` lifecycle routes ([`flows`]) — both bearer-gated
//! by `PEREGRIN_TOKEN`.

use std::collections::BTreeSet;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;

use peregrin_core::{Campaign, Event};
use peregrin_driver_github_webhook::SharedWebhook;
use peregrin_engine::Injector;

use crate::campaign::{self, CampaignError};
use crate::flows::{self, FlowHost, FlowSource};
use crate::inject;

/// One loaded campaign the daemon hosts.
pub struct HostedCampaign {
    /// The campaign's display/file key — `campaign.name` (or the
    /// yaml's file stem when unnamed), already deduplicated by the
    /// caller. Names the per-campaign history/queue-store files and
    /// the demux route owner.
    pub name: String,
    /// The loaded campaign — must be supervise-shaped (`supersede:`
    /// with ingress nodes for trigger-driven runs; finite campaigns
    /// idle, they just never see a trigger).
    pub campaign: Campaign,
    /// Where the yaml came from — the file it loaded from (boot +
    /// managed-dir flows) or posted text (in-memory flows). Reload
    /// re-reads it.
    pub source: FlowSource,
}

/// Per-run event sink — `(campaign_name, run_log)`; the serve-mode
/// equivalent of `campaign run`'s run-log output.
pub type RunCallback = Arc<dyn Fn(&str, &[Event]) + Send + Sync>;

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
    /// Where `POST /api/flows` persists posted yamls —
    /// `<dir>/<name>.yaml`, the same layout `peregrin provision
    /// deploy` writes, so a restart replays API-added flows. `None`
    /// keeps API-added flows in-memory only. The CLI fills this
    /// with the first directory path arg.
    pub managed_dir: Option<PathBuf>,
    /// Called with `(campaign name, run log)` for every finished
    /// run, in finish order — the CLI prints it. Runs on each
    /// campaign's supervisor thread.
    pub on_run: Option<RunCallback>,
    /// `RunOptions.secrets` for every campaign's spawned runs — the
    /// explicit record leg of secret resolution (ambient env →
    /// keyring → env file follow via the default stores).
    pub secrets: std::collections::BTreeMap<String, String>,
    /// This node's runner labels — the `runs_on` matcher's
    /// right-hand side (PL-007 C4). Campaigns whose `runs_on` isn't
    /// fully satisfied are skipped with a warning: a fleet node only
    /// hosts campaigns it declared itself eligible for. The CLI
    /// fills this from `PEREGRIN_RUNNER_LABELS`.
    pub labels: Vec<String>,
    /// The `--tracing`/`--metrics` observer (ADR-0040) shared by
    /// every hosted flow's listener + worker runs. `None` keeps the
    /// noop default.
    pub observer: Option<Arc<dyn peregrin_bus::Observer>>,
}

/// The daemon in flight — [`serve_campaigns`]'s return. `Drop`
/// requests shutdown (the signal flag + listener stop) but can't
/// join — call [`join`](Self::join) for the drained result.
pub struct Served {
    /// The shared listener's bound address — always bound in serve
    /// mode (a webhook-free fleet may gain flows via the API).
    pub listener: SocketAddr,
    /// Hosted campaign names at boot, in serve order. Flows added
    /// later appear in [`FlowHost::list`] / `GET /api/flows`, not
    /// here.
    pub campaigns: Vec<String>,
    /// The shared demux — exposed for tests/status (`repos()`,
    /// `route_count()`, `repos_for_owner()`).
    pub demux: Arc<SharedWebhook>,
    /// The flow registry — live spawn/delete/reload plus the drain
    /// join. The `/api/flows` router holds a clone.
    pub host: Arc<FlowHost>,
    /// The axum listener thread serving the merged router (webhook
    /// demux + `/api/events/inject` + `/api/flows`) — `join`
    /// collects it.
    listener_thread: Option<JoinHandle<()>>,
    /// Each boot campaign's external trigger injector — push events
    /// here for out-of-band feeds (tests, future daemon surfaces).
    /// The webhook routes hold their own clones, so dropping these
    /// does NOT end a webhook campaign's trigger stream — that's
    /// what [`request_shutdown`](Self::request_shutdown) is for.
    /// Flows spawned later via the API push their own injectors
    /// into `host.inject_targets`.
    pub injectors: Vec<Injector>,
    /// The session reconcile loop (P-054) — `None` when disabled or
    /// no campaign declares a `worktree_base`.
    reconcile: Option<JoinHandle<()>>,
    shutdown: Arc<AtomicBool>,
}

impl Served {
    /// Stop the shared listener and flag every flow's supervisor to
    /// drain — the SIGTERM path. The listener stops first so no new
    /// triggers can land between the flag and each loop's final
    /// channel drain. Idempotent; safe to call before `join`.
    pub fn request_shutdown(&self) {
        self.demux.stop();
        self.shutdown.store(true, Ordering::Relaxed);
        self.host.shutdown_all();
    }

    /// Whether shutdown was requested.
    #[must_use]
    pub fn is_shutdown_requested(&self) -> bool {
        self.shutdown.load(Ordering::Relaxed)
    }

    /// Drain and join every flow — first requests shutdown (stop
    /// accepting triggers), then joins the listener thread and every
    /// supervisor loop via [`FlowHost::join_all`]. Returns the first
    /// campaign error after all loops have finished (a panicking
    /// supervisor thread counts as an error).
    ///
    /// # Errors
    /// The first [`CampaignError`] any flow's loop produced.
    pub fn join(mut self) -> Result<(), CampaignError> {
        self.request_shutdown();
        // The listener thread exits on the same shutdown flag —
        // collect it before the supervisors so a wedged socket can't
        // outlive the drain.
        if let Some(thread) = self.listener_thread.take() {
            let _ = thread.join();
        }
        if let Some(thread) = self.reconcile.take() {
            let _ = thread.join();
        }
        self.host.join_all()
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

/// Boot the daemon: filter the fleet by `runs_on`, bind the ONE
/// shared listener unconditionally (webhook demux + `/api/events/
/// inject` + `/api/flows`), then spawn every campaign through
/// [`FlowHost::spawn`] — the same path `POST /api/flows` uses, so
/// boot flows and API flows are identical citizens.
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
                    source = %entry.source.describe(),
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
    // history/queue db, so reject them up front (`FlowHost::spawn`
    // re-checks for API-added flows).
    let mut seen = BTreeSet::new();
    for entry in &hosted {
        if !seen.insert(entry.name.clone()) {
            return Err(CampaignError::Serve(format!(
                "duplicate campaign name '{}'",
                entry.name
            )));
        }
    }

    // Durable dirs — created before the listener binds so a bad
    // path fails fast (the `campaign run --queue-store` convention).
    for dir in [
        config.queue_store_dir.as_ref(),
        config.history_dir.as_ref(),
        config.managed_dir.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        std::fs::create_dir_all(dir)
            .map_err(|e| CampaignError::Serve(format!("dir {}: {e}", dir.display())))?;
    }

    // One shared demux + listener — ALWAYS bound: the flows API can
    // add a webhook flow to a fleet that had none at boot, and its
    // deliveries must reach the demux. The same socket also serves
    // `POST /api/events/inject` (the typed-event feed external
    // producers push triggers through) and `/api/flows*`.
    let demux = Arc::new(SharedWebhook::new());
    let shutdown = Arc::new(AtomicBool::new(false));
    let inject_targets = inject::InjectTargets::default();
    let host = Arc::new(FlowHost::new(
        config.secrets.clone(),
        config.history_dir.clone(),
        config.queue_store_dir.clone(),
        config.labels.clone(),
        config.on_run.clone(),
        Arc::clone(&demux),
        Arc::clone(&inject_targets),
        config.managed_dir.clone(),
        config.observer.clone(),
        Arc::clone(&shutdown),
    ));
    let webhook = demux.router(&config.path).map_err(CampaignError::Serve)?;
    // Bearer gate: `PEREGRIN_TOKEN` when set (the credential
    // `peregrin serve` nodes and the bupha worker resolve); unset
    // leaves the route open — the tokenless-dev convention. The
    // socket is often tunnel-exposed, so keep the token configured
    // in production. The same token gates `/api/flows*`.
    let token = std::env::var("PEREGRIN_TOKEN")
        .ok()
        .filter(|t| !t.is_empty());
    let router = webhook
        .merge(inject::router(Arc::clone(&inject_targets), token.clone()))
        .merge(flows::router(Arc::clone(&host), token));
    let (listener, listener_thread) =
        inject::serve_listener(&config.listen, router, Arc::clone(&shutdown))
            .map_err(CampaignError::Serve)?;

    // Reconcile roots (P-054): every campaign's `worktree_base`
    // config — the session registry/ledger dirs the cleanup nodes
    // write under. Collected before `hosted` is consumed below.
    let reconcile_roots: Vec<crate::reconcile::ReconcileRoot> = hosted
        .iter()
        .filter_map(|entry| {
            entry
                .campaign
                .config
                .get("worktree_base")
                .and_then(serde_json::Value::as_str)
                .map(|base| crate::reconcile::ReconcileRoot {
                    name: entry.name.clone(),
                    worktree_base: PathBuf::from(base),
                })
        })
        .collect();

    let mut injectors = Vec::with_capacity(hosted.len());
    let mut names = Vec::with_capacity(hosted.len());
    for entry in hosted {
        let name = entry.name.clone();
        if !campaign::has_ingress_nodes(&entry.campaign) {
            tracing::warn!(
                campaign = %name,
                source = %entry.source.describe(),
                "campaign declares no ingress nodes — it will idle \
                 under `campaign serve` (no trigger source)"
            );
        }
        match host.spawn(entry) {
            Ok(injector) => {
                injectors.push(injector);
                names.push(name);
            }
            Err(e) => {
                // Earlier flows are live — drain them before
                // reporting the boot failure.
                host.shutdown_all();
                return Err(e);
            }
        }
    }

    // Session reconcile (P-054): expires retained sessions, retries
    // failed deletes, reaps orphans — runs beside the supervisors on
    // the same shutdown flag.
    let reconcile = crate::reconcile::spawn(reconcile_roots, Arc::clone(&shutdown));

    Ok(Served {
        listener,
        campaigns: names,
        demux,
        host,
        listener_thread: Some(listener_thread),
        injectors,
        reconcile,
        shutdown,
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
