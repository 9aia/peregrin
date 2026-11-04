//! The [`Queue`] — sequential run scheduler over a [`Supersede`]
//! policy. Pure logic: the caller owns the event loop, runs jobs one
//! at a time, and turns `cancel_active` signals into whatever
//! cancellation surface the runner exposes (an injected
//! `workflow.cancel` event on the engine's ingress channel).

use std::collections::VecDeque;

use peregrin_core::Event;

use crate::policy::{Mode, Supersede};

/// A scheduled run — the trigger `event`, its resolved supersede
/// `key`, and a monotonically increasing `seq` (submit order).
#[derive(Clone, Debug, PartialEq)]
pub struct Job {
    /// Submission sequence — 1, 2, 3… across the queue's lifetime.
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
    /// Replaced the pending job with the same key (`latest-wins`) —
    /// carries the displaced job, whose event never runs.
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
        let displaced = if self.policy.mode == Mode::LatestWins {
            self.pending.iter_mut().find(|job| job.key == key)
        } else {
            None
        };
        let disposition = if let Some(existing) = displaced {
            // Keep the displaced job's slot — the newest trigger
            // inherits its queue position.
            Disposition::ReplacedPending(std::mem::replace(existing, job))
        } else {
            self.pending.push_back(job);
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
}
