//! The [`Queue`] — sequential run scheduler over a [`Supersede`]
//! policy. Pure logic: the caller owns the event loop, runs jobs one
//! at a time, and turns `cancel_active` signals into whatever
//! cancellation surface the runner exposes (an injected
//! `workflow.cancel` event on the engine's ingress channel).

use std::collections::VecDeque;

use peregrin_core::Event;
use serde::{Deserialize, Serialize};

use crate::policy::{Mode, Supersede};

/// The `fifo`-mode bound — at most this many pending jobs per
/// supersede key. bupha's `MAX_PENDING` analog: a key whose backlog
/// hits the bound evicts its *oldest* pending job on the next submit
/// (the new trigger still queues at the back). `latest-wins` never
/// exceeds one pending job per key, so the bound only applies to
/// `fifo`. Per-key rather than global so one hot key can't starve
/// unrelated keys — and so a [`crate::QueueStore`] snapshot's
/// per-key rows stay small.
pub const MAX_PENDING_PER_KEY: usize = 32;

/// A scheduled run — the trigger `event`, its resolved supersede
/// `key`, and `seq`, the queue-order token.
///
/// `seq` is submit order for ordinary submits; a `latest-wins`
/// replacement *inherits* the displaced job's `seq` so the pending
/// deque stays sorted by `seq` — which is exactly drain order. That
/// invariant is what lets a [`crate::QueueStore`] persist jobs
/// per-key and reconstruct global order on restore.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Job {
    /// Queue-order token — monotonically increasing across submits,
    /// except `latest-wins` replacements inherit the displaced job's.
    pub seq: u64,
    /// The supersede key resolved from the event's payload.
    pub key: String,
    /// The trigger event the run is for.
    pub event: Event,
}

/// What `submit` did with the event.
#[derive(Clone, Debug, PartialEq)]
pub enum Disposition {
    /// Appended to the pending queue.
    Queued,
    /// A pending job was displaced — the `latest-wins` same-key
    /// replacement (which keeps the displaced job's queue position
    /// and `seq` token) or the `fifo` per-key bound eviction (the
    /// key's *oldest* pending job drops; the new one queues at the
    /// back). Carries the displaced job, whose event never runs.
    ReplacedPending(Job),
}

/// The outcome of one [`Queue::submit`] — the resolved key, where the
/// event landed, and whether the active run was marked superseded.
#[derive(Clone, Debug, PartialEq)]
pub struct Decision {
    /// The supersede key the event resolved to.
    pub key: String,
    /// Where the event landed.
    pub disposition: Disposition,
    /// `true` when this submit superseded the active run
    /// (`cancel_running: true` while a job is active). The superseded
    /// job is handed out via [`Queue::take_cancel`] — turning the
    /// signal into an actual cancellation is the caller's job.
    pub cancel_active: bool,
}

/// The run scheduler. One active run at a time (the gate case):
/// [`Queue::drain`] refuses to hand out a job while one is active and
/// [`Queue::finish`] releases the slot.
#[derive(Debug)]
pub struct Queue {
    policy: Supersede,
    pending: VecDeque<Job>,
    active: Option<Job>,
    /// The active job at the moment a submit marked it superseded —
    /// set once and held until `take_cancel` or `finish`.
    cancel: Option<Job>,
    seq: u64,
}

impl Queue {
    /// An empty queue under `policy`.
    #[must_use]
    pub fn new(policy: Supersede) -> Self {
        Self {
            policy,
            pending: VecDeque::new(),
            active: None,
            cancel: None,
            seq: 0,
        }
    }

    /// The policy this queue runs.
    #[must_use]
    pub fn policy(&self) -> &Supersede {
        &self.policy
    }

    /// Resolve the supersede key `event` would get — [`Supersede::key_of`].
    #[must_use]
    pub fn key_of(&self, event: &Event) -> String {
        self.policy.key_of(event)
    }

    /// Feed a trigger event: resolve its key, dedupe against pending
    /// per `mode`, and flag the active run superseded when
    /// `cancel_running` is set.
    pub fn submit(&mut self, event: Event) -> Decision {
        let key = self.policy.key_of(&event);
        self.seq += 1;
        let job = Job {
            seq: self.seq,
            key: key.clone(),
            event,
        };
        let cancel_active = self.policy.cancel_running && self.active.is_some();
        if cancel_active && self.cancel.is_none() {
            self.cancel = self.active.clone();
        }
        let displaced = self.place(job);
        let disposition = if let Some(displaced) = displaced {
            Disposition::ReplacedPending(displaced)
        } else {
            Disposition::Queued
        };
        Decision {
            key,
            disposition,
            cancel_active,
        }
    }

    /// Hand out the next pending job and mark it active — `None`
    /// while a run is active (sequential drain; the gate case) or the
    /// queue is empty. Same-key pending jobs that a `latest-wins`
    /// submit displaced never reach this point.
    pub fn drain(&mut self) -> Option<Job> {
        if self.active.is_some() {
            return None;
        }
        let job = self.pending.pop_front()?;
        self.active = Some(job.clone());
        Some(job)
    }

    /// The active run ended — release the slot, drop a stale
    /// cancellation signal (the run is gone either way), and return
    /// the finished job.
    pub fn finish(&mut self) -> Option<Job> {
        self.cancel = None;
        self.active.take()
    }

    /// Take the pending cancellation signal — the active job a
    /// `cancel_running` submit superseded. `Some` at most once per
    /// active run; `finish` clears an unread signal.
    pub fn take_cancel(&mut self) -> Option<Job> {
        self.cancel.take()
    }

    /// The job currently running, if any.
    #[must_use]
    pub fn active(&self) -> Option<&Job> {
        self.active.as_ref()
    }

    /// Pending jobs in drain order.
    pub fn pending(&self) -> impl Iterator<Item = &Job> {
        self.pending.iter()
    }

    /// Number of pending jobs.
    #[must_use]
    pub fn pending_len(&self) -> usize {
        self.pending.len()
    }

    /// No active run and nothing pending — the scheduler is quiet.
    #[must_use]
    pub fn is_idle(&self) -> bool {
        self.active.is_none() && self.pending.is_empty()
    }

    /// Insert `job` into pending under the mode's dedupe/bound rules —
    /// shared by [`Queue::submit`] and [`Queue::restore`]. Returns the
    /// displaced job when one was evicted/replaced.
    fn place(&mut self, mut job: Job) -> Option<Job> {
        match self.policy.mode {
            Mode::LatestWins => {
                if let Some(existing) = self.pending.iter_mut().find(|j| j.key == job.key) {
                    // The newest trigger inherits the displaced job's
                    // queue position *and* `seq` token — pending stays
                    // sorted by `seq`, which is drain order.
                    job.seq = existing.seq;
                    return Some(std::mem::replace(existing, job));
                }
            }
            Mode::Fifo => {
                let for_key = self.pending.iter().filter(|j| j.key == job.key).count();
                if for_key >= MAX_PENDING_PER_KEY {
                    if let Some(pos) = self.pending.iter().position(|j| j.key == job.key) {
                        if let Some(evicted) = self.pending.remove(pos) {
                            self.pending.push_back(job);
                            return Some(evicted);
                        }
                    }
                }
            }
        }
        self.pending.push_back(job);
        None
    }

    /// The persistable view of this queue — pending jobs in drain
    /// order plus the in-flight marker. [`Queue::restore`] is the
    /// inverse.
    #[must_use]
    pub fn snapshot(&self) -> QueueSnapshot {
        QueueSnapshot {
            pending: self.pending.iter().cloned().collect(),
            running: self.active.clone(),
        }
    }

    /// Rebuild a queue from a persisted [`QueueSnapshot`] — the boot
    /// path a [`crate::QueueStore`] drives. Every entry is
    /// re-validated against the policy before enqueueing: a job whose
    /// stored `key` doesn't match `policy.key_of(&job.event)` (the
    /// campaign's `supersede:` block changed, or the row is corrupt)
    /// is dropped into [`RestoreOutcome::dropped`] for the caller to
    /// log. Accepted entries go through the same dedupe/bound path as
    /// [`Queue::submit`], so a restored queue can never hold a state
    /// live submits couldn't produce.
    ///
    /// The `running` marker re-queues at the *front*: a well-formed
    /// snapshot's running job carries the smallest `seq` (it was
    /// drained before everything still pending), so merging it into
    /// the pending set and sorting by `seq` lands it first — the
    /// interrupted run gets another shot (at-least-once). Under
    /// `latest-wins` a newer same-key pending entry still supersedes
    /// it, exactly as it would have live.
    #[must_use]
    pub fn restore(policy: Supersede, snapshot: QueueSnapshot) -> RestoreOutcome {
        let mut queue = Queue::new(policy);
        let mut entries = snapshot.pending;
        if let Some(running) = snapshot.running {
            entries.push(running);
        }
        // Continue the submit counter past every persisted token so
        // post-restore submits can't collide with restored `seq`s.
        queue.seq = entries.iter().map(|job| job.seq).max().unwrap_or(0);
        // Pending is `seq`-sorted by invariant (see `Job::seq`); the
        // sort is stable so equal tokens keep snapshot order.
        entries.sort_by_key(|job| job.seq);
        let mut dropped = Vec::new();
        for job in entries {
            if job.key != queue.policy.key_of(&job.event) {
                dropped.push(job);
                continue;
            }
            queue.place(job);
        }
        RestoreOutcome { queue, dropped }
    }
}

/// The persistable queue state — see [`Queue::snapshot`] /
/// [`Queue::restore`]. Serde-shaped so store impls can move it as one
/// document or per-key rows.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct QueueSnapshot {
    /// Pending submissions in drain order.
    pub pending: Vec<Job>,
    /// The in-flight job — re-queues at the front on restore.
    pub running: Option<Job>,
}

/// What [`Queue::restore`] produced — the rebuilt queue plus the
/// entries validation rejected (already-logged-worthy; the caller
/// decides how loudly to report them).
#[derive(Debug)]
pub struct RestoreOutcome {
    /// The restored queue.
    pub queue: Queue,
    /// Persisted jobs dropped because their stored key didn't match
    /// the policy's resolution — corrupt rows or a `supersede:`
    /// config change since they were written.
    pub dropped: Vec<Job>,
}
