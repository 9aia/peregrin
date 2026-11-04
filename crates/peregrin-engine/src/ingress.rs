//! The ingress injection channel — external events into a live run.
//!
//! `LocalBus::publish` takes `&mut self` and the runner is
//! single-threaded and synchronous, so a producer that needs to push
//! events into a running campaign (a webhook listener, a WAL poller,
//! the queue's cancel signal) cannot call the bus directly — a
//! blocking ingress driver would stall the loop forever (audit
//! I-014). The channel splits that: [`Injector`] is the
//! `Send + Sync`-cloneable producer half any thread can hold;
//! [`Ingress`] is the receiver the [`Runner`](crate::Runner) drains
//! once per loop iteration.
//!
//! Ordering: drained events append to the run's `pending` queue, so
//! an injected event serializes *after* every event already pending
//! at that iteration — including the triggers of the step currently
//! executing — and *before* anything nodes emit afterward. A
//! `workflow.cancel` pushed through the injector cancels the run at
//! the next iteration boundary, exactly like a node-emitted one.
//! Events pushed after the run's queues drain (or before `run`
//! starts) still land — the runner drains the channel on the first
//! iteration too — but a quiescent run exits on its own schedule;
//! the channel does not keep a run alive.

use std::fmt;
use std::sync::mpsc::{Receiver, Sender};

use peregrin_core::Event;

/// Create an ingress channel pair: the [`Injector`] producers clone
/// and hand to external event sources, the [`Ingress`] the runner
/// drains via `Runner::with_ingress`.
#[must_use]
pub fn ingress() -> (Injector, Ingress) {
    let (tx, rx) = std::sync::mpsc::channel();
    (Injector { tx }, Ingress { rx })
}

/// The producer half of the ingress channel — `Send + Sync` +
/// cloneable, so any number of external event sources can emit into
/// the run without `&mut` access to the bus.
#[derive(Clone)]
pub struct Injector {
    tx: Sender<Event>,
}

impl Injector {
    /// Queue `event` for the run's next drain. Fails with
    /// [`InjectError`] carrying the event back once the run is gone
    /// (receiver dropped).
    pub fn inject(&self, event: Event) -> Result<(), InjectError> {
        self.tx
            .send(event)
            .map_err(|err| InjectError(Box::new(err.0)))
    }
}

/// The receiver half — the runner's drain point. One per run; not
/// cloneable (draining is the runner's exclusive job).
pub struct Ingress {
    rx: Receiver<Event>,
}

impl Ingress {
    /// Move every queued event into `out`, in arrival order. Returns
    /// the number drained. Non-blocking; a dropped last sender just
    /// means the channel stays empty.
    pub fn drain_into(&self, out: &mut Vec<Event>) -> usize {
        let before = out.len();
        out.extend(self.rx.try_iter());
        out.len() - before
    }
}

/// `Injector::inject` failed — the run already ended (receiver
/// dropped). Carries the undelivered event so the caller can log or
/// requeue it (boxed — `Event` is large and this is the cold path).
#[derive(Debug)]
pub struct InjectError(pub Box<Event>);

impl fmt::Display for InjectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ingress channel closed — run already ended")
    }
}

impl std::error::Error for InjectError {}
