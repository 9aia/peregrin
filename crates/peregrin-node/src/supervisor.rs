//! The node-side composition of the two coupled P1 pieces — the
//! `peregrin-queue` supersede policy and the engine's ingress
//! injection channel. [`RunSupervisor`] owns the [`Queue`], hands out
//! one [`PendingRun`] at a time (each carrying a freshly-created
//! ingress pair), and turns `cancel_running` supersede signals into a
//! `workflow.cancel` event injected into the active run's channel —
//! the cancellation surface the single-threaded runner actually
//! honours (the run ends at the next loop iteration).
//!
//! The daemon's event loop drives it like:
//!
//! ```ignore
//! let mut supervisor = RunSupervisor::new(policy);
//! // on every trigger event:
//! let outcome = supervisor.submit(event);
//! if let Some(pending) = outcome.start {
//!     let runner = campaign::runner().with_ingress(pending.ingress);
//!     thread::spawn(move || runner.run(&campaign, &RunOptions::default()));
//! }
//! // when the active run returns:
//! if let Some(next) = supervisor.finish() { /* start it the same way */ }
//! ```
//!
//! [`supervise`] is the production driving loop — persistent
//! ingress listeners → `submit` → per-trigger spawned runs →
//! sequential drain, exactly as sketched above.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use peregrin_core::{Campaign, Event};
use peregrin_driver::{DriverError, SecretSource};
use peregrin_engine::{Ingress, Injector, RunOptions, Runner, WaitOutcome, ingress};
use peregrin_queue::{Decision, Job, Queue, QueueStore, QueueStoreError, Supersede};
use peregrin_store::MemoryDlq;

use crate::campaign::{self, CampaignError};

/// A job ready to start — a freshly-created ingress channel pair with
/// the trigger event already queued on it, so the run sees its
/// trigger on the first loop iteration (published after `start`).
/// Hand `ingress` to `Runner::with_ingress`; clone `injector` for any
/// external producers that should emit mid-run.
#[non_exhaustive]
pub struct PendingRun {
    /// The scheduled job — trigger event + resolved supersede key.
    pub job: Job,
    /// Producer half for this run's channel (the supervisor keeps a
    /// clone for cancels).
    pub injector: Injector,
    /// Receiver half — `Runner::with_ingress`.
    pub ingress: Ingress,
}

/// What one [`RunSupervisor::submit`] produced.
#[non_exhaustive]
pub struct SubmitOutcome {
    /// The queue's raw decision — resolved key, dedupe disposition.
    pub decision: Decision,
    /// When the supervisor was idle, the run to start now.
    pub start: Option<PendingRun>,
    /// A `workflow.cancel` was pushed into the active run's ingress
    /// channel (`cancel_running` + a run in flight). `false` when the
    /// signal didn't apply or the channel was already closed (a run
    /// that ended without `finish` being called yet still resolves
    /// itself out).
    pub cancelled_active: bool,
}

/// Sequential run supervisor: one active run, pending jobs deduped by
/// supersede key, active-run cancels delivered over the run's own
/// ingress channel.
pub struct RunSupervisor {
    queue: Queue,
    /// The active run's injector — `Some` from `start` until
    /// `finish`. Clones of it are what external producers hold.
    injector: Option<Injector>,
    /// The durable queue store — `None` keeps the queue purely
    /// in-memory (the pre-C1a behavior).
    store: Option<Arc<dyn QueueStore>>,
}

impl RunSupervisor {
    /// A supervisor over the campaign's `supersede:` policy.
    #[must_use]
    pub fn new(policy: Supersede) -> Self {
        Self {
            queue: Queue::new(policy),
            injector: None,
            store: None,
        }
    }

    /// A supervisor over `policy` *with* a durable [`QueueStore`] —
    /// loads the persisted snapshot and replays it through
    /// [`Queue::restore`] before this supervisor accepts anything:
    /// pending submissions come back in drain order and an
    /// interrupted run's marker re-queues at the front
    /// (at-least-once). Entries the restore validation rejects are
    /// logged and dropped.
    ///
    /// # Errors
    /// [`QueueStoreError`] when the store can't be read — a corrupt
    /// *entry* never fails the load (restore drops it), but an
    /// unreadable store means the durable state is unknowable, so
    /// booting anyway would silently lose queued triggers.
    pub fn with_store(
        policy: Supersede,
        store: Arc<dyn QueueStore>,
    ) -> Result<Self, QueueStoreError> {
        let snapshot = store.load()?;
        let restored = Queue::restore(policy, snapshot);
        if !restored.dropped.is_empty() {
            tracing::warn!(
                "queue store: dropped {} persisted job(s) that failed re-validation",
                restored.dropped.len()
            );
        }
        Ok(Self {
            queue: restored.queue,
            injector: None,
            store: Some(store),
        })
    }

    /// The underlying queue — pending/active inspection for status
    /// surfaces. Mutation stays on this type (`submit`/`finish`) so
    /// the injector bookkeeping can't drift from the queue state.
    #[must_use]
    pub fn queue(&self) -> &Queue {
        &self.queue
    }

    /// Feed a trigger event. When a run slot is free the queued job
    /// comes straight back as a [`PendingRun`] to start; when a run
    /// is active the event queues per the policy (and cancels it
    /// under `cancel_running`).
    pub fn submit(&mut self, event: Event) -> SubmitOutcome {
        let decision = self.queue.submit(event);
        // Drain the queue's one-shot signal whenever one is pending —
        // `decision.cancel_active` just reports *this* submit raised
        // it; a signal from an earlier submit must not linger.
        let cancelled_active = self.cancel_active();
        let start = self.start_next();
        if self.store.is_some() {
            // Persist *after* the drain: `start_next` may have popped a
            // pending job (possibly of this same key) into `running`,
            // and the per-key writes must reflect post-mutation state.
            self.persist_key(&decision.key);
            if let Some(pending) = &start
                && pending.job.key != decision.key
            {
                self.persist_key(&pending.job.key);
            }
            self.persist_running();
        }
        SubmitOutcome {
            decision,
            start,
            cancelled_active,
        }
    }

    /// The active run ended — drop its injector and hand out the
    /// next pending run, if any.
    pub fn finish(&mut self) -> Option<PendingRun> {
        self.queue.finish();
        self.injector = None;
        let start = self.start_next();
        if self.store.is_some() {
            if let Some(pending) = &start {
                self.persist_key(&pending.job.key);
            }
            self.persist_running();
        }
        start
    }

    /// No active run and nothing pending.
    #[must_use]
    pub fn is_idle(&self) -> bool {
        self.injector.is_none() && self.queue.is_idle()
    }

    /// Push `workflow.cancel` into the active run's channel and drain
    /// the queue's one-shot signal. `false` when there's nothing to
    /// cancel or the channel is already closed.
    fn cancel_active(&mut self) -> bool {
        let Some(job) = self.queue.take_cancel() else {
            return false;
        };
        let reason = format!("superseded by a newer trigger for key '{}'", job.key);
        self.injector
            .as_ref()
            .is_some_and(|injector| injector.inject(Event::cancel(&reason)).is_ok())
    }

    /// Write this key's current pending set to the store — a no-op
    /// without one. Persist failures are logged, not fatal: the
    /// in-memory queue stays authoritative for this process's
    /// lifetime; durability degrades rather than killing runs.
    fn persist_key(&self, key: &str) {
        let Some(store) = &self.store else {
            return;
        };
        let jobs: Vec<Job> = self
            .queue
            .pending()
            .filter(|job| job.key == key)
            .cloned()
            .collect();
        if let Err(e) = store.persist_key(key, &jobs) {
            tracing::error!("queue store persist_key({key:?}) failed: {e}");
        }
    }

    /// Write the in-flight marker (or clear it) — same failure
    /// policy as [`Self::persist_key`].
    fn persist_running(&self) {
        let Some(store) = &self.store else {
            return;
        };
        if let Err(e) = store.set_running(self.queue.active()) {
            tracing::error!("queue store set_running failed: {e}");
        }
    }

    /// Drain the queue into a `PendingRun` when no run is active —
    /// creates the run's ingress channel and pre-loads the trigger
    /// event on it.
    fn start_next(&mut self) -> Option<PendingRun> {
        if self.injector.is_some() {
            return None;
        }
        let job = self.queue.drain()?;
        let (injector, ingress) = ingress();
        // Cannot fail — `ingress` is alive inside the PendingRun.
        let _ = injector.inject(job.event.clone());
        self.injector = Some(injector.clone());
        Some(PendingRun {
            job,
            injector,
            ingress,
        })
    }
}

// ---------------------------------------------------------------------------
// The supervised campaign — the production driving loop.
// ---------------------------------------------------------------------------

/// Per-spawned-run materials [`supervise`] needs. `RunOptions` is
/// `!Send` (its `secret_stores` hold `Rc` stores) and spawned runs
/// execute on their own threads, so the caller supplies the `Send`
/// parts plus a factory each run thread invokes for its own chain.
pub struct SupervisedOptions {
    /// `RunOptions.secrets` for every spawned run.
    pub secrets: BTreeMap<String, String>,
    /// `RunOptions.secret_stores` factory — invoked inside each run
    /// thread ([`campaign::default_secret_stores`] for the stock
    /// `record → env → keyring → file` chain).
    pub secret_stores: Arc<dyn Fn() -> Vec<SecretSource> + Send + Sync>,
    /// The durable history every spawned run appends its events to —
    /// the `campaign run --history` JSONL `peregrin runs`/`log`/
    /// `explain` read back. `None` keeps run logs ephemeral.
    pub history: Option<PathBuf>,
    /// The durable queue store (`campaign run --queue-store`) —
    /// pending triggers and the in-flight marker survive a restart:
    /// on boot the supervisor replays the snapshot before accepting
    /// new triggers. `None` keeps the queue in-memory only.
    pub queue_store: Option<Arc<dyn QueueStore>>,
    /// The shared webhook demux (`campaign serve`) — when set, the
    /// listener run's `github-webhook` nodes register their repo
    /// watches into it via [`SharedWebhookDriver`] instead of binding
    /// their own socket (one listener for every hosted campaign).
    /// `None` keeps the solo [`GithubWebhookDriver`] behavior.
    pub shared_webhook: Option<Arc<peregrin_driver_github_webhook::SharedWebhook>>,
    /// Shutdown flag (`campaign serve`'s SIGTERM path) — once set the
    /// loop stops accepting triggers and drains exactly like a
    /// trigger-channel disconnect: the active run finishes, pending
    /// jobs run, the listener run is cancelled. Polled on the loop's
    /// tick, so an idle supervisor notices within ~100ms.
    pub shutdown: Option<Arc<AtomicBool>>,
    /// Called with each finished run's event log, in finish order —
    /// the CLI prints it. Runs on the supervisor thread.
    pub on_run: Option<OnRunDone>,
}

/// The per-finished-run callback [`SupervisedOptions::on_run`]
/// takes — the run's event log, in finish order.
pub type OnRunDone = Box<dyn FnMut(&[Event]) + Send>;

impl Default for SupervisedOptions {
    fn default() -> Self {
        Self {
            secrets: BTreeMap::new(),
            secret_stores: Arc::new(campaign::default_secret_stores),
            history: None,
            queue_store: None,
            shared_webhook: None,
            shutdown: None,
            on_run: None,
        }
    }
}

/// A supervised campaign in flight — [`supervise`]'s return.
pub struct SupervisedRun {
    /// The trigger channel's producer half — the ingress listeners
    /// already hold clones; push trigger events here for out-of-band
    /// feeds (tests, a daemon surface). Drop every clone to signal
    /// the end of triggers: the supervisor drains its queue and
    /// exits.
    pub injector: Injector,
    /// The supervisor loop — join for every spawned run's event log
    /// concatenated in finish order, or the first run failure.
    pub join: JoinHandle<Result<Vec<Event>, CampaignError>>,
}

/// The `Send`-able half of [`SupervisedOptions`] shared across the
/// listener and worker threads.
struct RunMaterials {
    secrets: BTreeMap<String, String>,
    secret_stores: Arc<dyn Fn() -> Vec<SecretSource> + Send + Sync>,
    history: Option<PathBuf>,
}

impl RunMaterials {
    fn run_options(&self) -> RunOptions {
        RunOptions {
            secrets: self.secrets.clone(),
            secret_stores: (self.secret_stores)(),
            ..RunOptions::default()
        }
    }
}

/// A spawned run's join handle — the finished log or the run's
/// failure (a store-open [`CampaignError::Store`] or an engine
/// [`CampaignError::Run`]).
type RunHandle = JoinHandle<Result<Vec<Event>, CampaignError>>;

/// Drive a `supersede:` campaign — the composition
/// docs/reference/queue-supersede.md specifies: the campaign's
/// ingress-driver nodes (`github-webhook`, `sturn-poll`) fire once
/// inside a persistent *listener* `Runner` bound to a
/// supervisor-facing channel, every trigger the channel delivers
/// goes through [`RunSupervisor::submit`], and each [`PendingRun`]
/// spawns a `Runner` over the campaign's non-ingress (worker) nodes
/// with the trigger pre-injected on its channel. Runs are sequential
/// — the gate case: the queue never hands out a second job while one
/// is active.
///
/// Cancellation is cooperative: `cancel_running` lands a
/// `workflow.cancel` on the *active* run's own ingress channel, so
/// the run ends at the next loop iteration — an in-flight driver
/// call (a `shell` inner command, a `git` materialization) finishes
/// first; there is no forced kill.
///
/// The loop ends when every [`Injector`] clone is gone (listeners
/// exited, callers dropped their handle): it drains the queue —
/// pending jobs still run — then cancels the listener run and
/// returns.
#[must_use]
pub fn supervise(campaign: &Campaign, options: SupervisedOptions) -> SupervisedRun {
    let SupervisedOptions {
        secrets,
        secret_stores,
        history,
        queue_store,
        shared_webhook,
        shutdown,
        on_run,
    } = options;
    let materials = Arc::new(RunMaterials {
        secrets,
        secret_stores,
        history,
    });

    // The queue-facing channel: ingress drivers push triggers here —
    // never into a run's own channel.
    let (trigger_injector, trigger_ingress) = ingress();
    let external = trigger_injector.clone();

    // Ingress nodes fire inside the listener run; everything else is
    // the per-trigger worker graph. `shared_webhook` swaps the solo
    // `github-webhook` driver for the demux-registering one — every
    // hosted campaign's webhook nodes then share the daemon's single
    // listener instead of racing for the default port.
    let listener_registry = match &shared_webhook {
        Some(demux) => campaign::registry_with_ingress_shared(&trigger_injector, demux),
        None => campaign::registry_with_ingress(&trigger_injector),
    };
    let (listener_campaign, worker_campaign) =
        campaign::split_ingress(campaign, &listener_registry);

    // The persistent listener run — `with_ingress` gives it the
    // park/cancel channel its serve semantics need (its drivers push
    // to the queue channel; nothing else arrives). Spawned only when
    // the campaign actually declares ingress nodes — a `supersede:`
    // campaign fed purely through the returned injector needs no
    // listener.
    let listener = (!listener_campaign.nodes.is_empty()).then(|| {
        let (cancel_injector, cancel_ingress) = ingress();
        let materials = Arc::clone(&materials);
        let thread = thread::spawn(move || {
            let mut runner = Runner::new(listener_registry)
                .with_dlq(Box::<MemoryDlq>::default())
                .with_ingress(cancel_ingress);
            runner.run(&listener_campaign, &materials.run_options())
        });
        (thread, cancel_injector)
    });
    let (listener_thread, listener_injector) = listener.unzip();

    let worker_campaign = Arc::new(worker_campaign);
    let policy = Supersede::from(campaign.supersede.clone().unwrap_or_default());
    let join = thread::spawn(move || {
        supervise_loop(
            policy,
            trigger_ingress,
            worker_campaign,
            listener_thread,
            listener_injector,
            materials,
            queue_store,
            shutdown,
            on_run,
        )
    });
    SupervisedRun {
        injector: external,
        join,
    }
}

/// Spawn a worker run for `pending` — the `Runner` is `!Send` (Rc
/// internals), so it's constructed inside the thread. The trigger
/// event is already queued on `pending.ingress` — the run publishes
/// it on its first iteration.
fn spawn_worker(
    pending: PendingRun,
    campaign: &Arc<Campaign>,
    materials: &Arc<RunMaterials>,
) -> RunHandle {
    let campaign = Arc::clone(campaign);
    let materials = Arc::clone(materials);
    thread::spawn(move || {
        let mut runner = campaign::worker_runner(pending.ingress, materials.history.as_deref())?;
        runner
            .run(&campaign, &materials.run_options())
            .map_err(CampaignError::Run)
    })
}

/// The supervisor event loop — runs on [`supervise`]'s thread.
/// Parks on the trigger channel while it's live, harvesting finished
/// runs between events; when every producer is gone it joins the
/// in-flight run, drains the queue to empty, then cancels the
/// listener run. Run failures don't kill the loop — the first error
/// is recorded and returned after shutdown.
// `needless_pass_by_value`: every argument must be *owned* — the
// loop runs on a freshly-spawned thread — even though the body only
// borrows them. `too_many_arguments`: the loop's inputs are the
// supervise() destructuring, one param each.
#[allow(clippy::needless_pass_by_value, clippy::too_many_arguments)]
fn supervise_loop(
    policy: Supersede,
    trigger_ingress: Ingress,
    worker_campaign: Arc<Campaign>,
    listener: Option<JoinHandle<Result<Vec<Event>, DriverError>>>,
    listener_injector: Option<Injector>,
    materials: Arc<RunMaterials>,
    queue_store: Option<Arc<dyn QueueStore>>,
    shutdown: Option<Arc<AtomicBool>>,
    mut on_run: Option<OnRunDone>,
) -> Result<Vec<Event>, CampaignError> {
    /// How long the loop parks on the trigger channel while a run is
    /// active — a finished run can't wake the channel, so the loop
    /// ticks to harvest it. Irrelevant while idle: `wait(None)`
    /// parks until the next event or disconnect.
    const TICK: Duration = Duration::from_millis(100);

    // Boot: with a store, load + replay the persisted queue *before*
    // the loop accepts any trigger — restored pending jobs (the
    // interrupted run re-queued at the front) drain ahead of anything
    // that arrives from here on.
    let mut supervisor = match queue_store {
        Some(store) => RunSupervisor::with_store(policy, store)?,
        None => RunSupervisor::new(policy),
    };
    let mut worker: Option<RunHandle> = None;
    let mut triggers_done = false;
    let mut logs = Vec::new();
    let mut first_error: Option<CampaignError> = None;
    let mut buf = Vec::new();

    loop {
        if !triggers_done {
            // `campaign serve`'s shutdown flag takes the same path as
            // a trigger-channel disconnect — stop reading, drain.
            // With a flag configured the loop always ticks so an
            // idle supervisor still notices; without one the old
            // park-forever-when-idle behavior is unchanged. The flag
            // path still does a final `drain_into`: triggers already
            // queued when shutdown lands are in-flight work, not new
            // triggers — they submit before the drain begins.
            if shutdown
                .as_ref()
                .is_some_and(|flag| flag.load(Ordering::Relaxed))
            {
                triggers_done = true;
                trigger_ingress.drain_into(&mut buf);
            } else {
                let timeout = if worker.is_some() || shutdown.is_some() {
                    Some(TICK)
                } else {
                    None
                };
                if trigger_ingress.wait(timeout, &mut buf) == WaitOutcome::Disconnected {
                    triggers_done = true;
                }
            }
        }
        for event in buf.drain(..) {
            let outcome = supervisor.submit(event);
            if let Some(pending) = outcome.start {
                worker = Some(spawn_worker(pending, &worker_campaign, &materials));
            }
        }
        // Harvest the finished run — `finish` hands the queue slot to
        // the next pending job, which spawns the same way. After the
        // channel dies a worker can't be re-woken any other way, so
        // block on `join` (sequential drain).
        if let Some(handle) = worker.take() {
            if triggers_done || handle.is_finished() {
                let result = handle.join().unwrap_or_else(|_| {
                    Err(CampaignError::Run(DriverError::new(
                        "spawned run thread panicked",
                    )))
                });
                match result {
                    Ok(log) => {
                        if let Some(on_run) = on_run.as_deref_mut() {
                            on_run(&log);
                        }
                        logs.extend(log);
                    }
                    Err(e) => {
                        tracing::error!("supervised run failed: {e}");
                        if first_error.is_none() {
                            first_error = Some(e);
                        }
                    }
                }
                worker = supervisor
                    .finish()
                    .map(|pending| spawn_worker(pending, &worker_campaign, &materials));
            } else {
                worker = Some(handle);
            }
        }
        if triggers_done && worker.is_none() {
            break;
        }
    }

    // Shut the listener run down — its serve loop honours an injected
    // `workflow.cancel` like any other event.
    if let Some(injector) = listener_injector {
        let _ = injector.inject(Event::cancel("supervisor shutdown"));
    }
    if let Some(handle) = listener {
        match handle.join() {
            Ok(Ok(_)) => {}
            Ok(Err(e)) => {
                tracing::error!("ingress listener run failed: {e}");
                if first_error.is_none() {
                    first_error = Some(CampaignError::Run(e));
                }
            }
            Err(_) => {
                tracing::error!("ingress listener thread panicked");
                if first_error.is_none() {
                    first_error = Some(CampaignError::Run(DriverError::new(
                        "ingress listener thread panicked",
                    )));
                }
            }
        }
    }

    match first_error {
        Some(error) => Err(error),
        None => Ok(logs),
    }
}
