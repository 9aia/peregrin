//! The node-side composition of the two coupled P1 pieces — the
//! `peregrin-queue` supersede policy and the engine's ingress
//! injection channel. [`RunSupervisor`] owns the [`Queue`], hands out
//! one [`PendingRun`] at a time (each carrying a freshly-created
//! ingress pair), and turns `cancel_running` supersede signals into a
//! `workflow.cancel` event injected into the active run's channel —
//! the cancellation surface the single-threaded runner actually
//! honours (the run ends at the next loop iteration).
//!
//! The daemon's event loop (a later phase) drives it like:
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

use peregrin_core::Event;
use peregrin_engine::{Ingress, Injector, ingress};
use peregrin_queue::{Decision, Job, Queue, Supersede};

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
}

impl RunSupervisor {
    /// A supervisor over the campaign's `supersede:` policy.
    #[must_use]
    pub fn new(policy: Supersede) -> Self {
        Self {
            queue: Queue::new(policy),
            injector: None,
        }
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
        self.start_next()
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
