//! peregrin-queue — the campaign-level run scheduler: node-side policy
//! deciding *which* runs happen when trigger events outpace the
//! engine (the engine itself executes exactly one run at a time).
//!
//! The [`Queue`] applies a per-campaign [`Supersede`] policy:
//!
//! - **`key`** — a `{dotted.path}` template resolved over each
//!   trigger event's `payload` (`{repo}:{ref}` = one logical target
//!   per branch). Events resolving to the same key are alternative
//!   triggers for the same run.
//! - **`mode`** — `latest-wins` replaces a pending job sharing the
//!   key (only the newest trigger runs); `fifo` appends (every
//!   trigger runs).
//! - **`cancel_running`** — when set, any new event signals that the
//!   *active* run is superseded and should be cancelled (how the
//!   cancel reaches the run is the caller's concern — the engine's
//!   ingress channel accepts a `workflow.cancel` event); when unset,
//!   new events queue behind the active run.
//!
//! The crate is pure logic — no I/O, no threads, no event loop. The
//! caller (the node daemon) owns driving it: `submit` on every
//! trigger event, `drain` when a run slot frees, `finish` when a run
//! ends, `take_cancel` to read the pending cancellation signal. See
//! `docs/reference/queue-supersede.md`.
//!
//! Durability is a port, not a feature: [`QueueStore`] persists the
//! pending submissions per supersede key plus the in-flight marker
//! so a restarted supervisor replays what a dead process left
//! behind ([`Queue::snapshot`] / [`Queue::restore`] are the
//! serde-shaped boundary). `peregrin-queue-sqlite` ships the
//! production sqlite impl; [`MemoryQueueStore`] covers tests.

mod policy;
mod queue;
mod store;

pub use policy::{Mode, Supersede, resolve_key};
pub use queue::{
    Decision, Disposition, Job, MAX_PENDING_PER_KEY, Queue, QueueSnapshot, RestoreOutcome,
};
pub use store::{MemoryQueueStore, QueueStore, QueueStoreError};
