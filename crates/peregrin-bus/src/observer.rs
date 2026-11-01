//! The observer port — the TS `Observer`/`Span` pair. All hooks are
//! no-ops by default so partial impls override only what they need;
//! [`NoopObserver`] is the runner's default.

use serde_json::Value;

use peregrin_core::{Campaign, Event, Node};

/// A node-execution span (TS `Span`). Every method is optional.
pub trait Span: Send {
    /// Close the span (TS `end`).
    fn end(&mut self) {}
    /// Attach an attribute (TS `setAttribute`).
    fn set_attribute(&mut self, _key: &str, _value: &Value) {}
    /// Record an error message (TS `recordError`).
    fn record_error(&mut self, _message: &str) {}
    /// The W3C traceparent propagated into `ctx.traceparent`.
    fn traceparent(&self) -> String {
        String::new()
    }
}

/// The no-op span [`NoopObserver`] hands out.
pub struct NoopSpan;

impl Span for NoopSpan {}

/// Run-lifecycle hooks (TS `Observer`). Default impls do nothing —
/// except [`Observer::on_node_end`], which ends the span like the TS
/// `NoopObserver`.
pub trait Observer: Send + Sync {
    /// A run started (TS `onWorkflowStart`).
    fn on_workflow_start(&self, _campaign: &Campaign) {}
    /// A run finished (TS `onWorkflowEnd`).
    fn on_workflow_end(&self, _event_count: usize, _cancelled: bool, _cycle: bool, _failed: bool) {}
    /// A node fired; returns its span (TS `onNodeStart`).
    fn on_node_start(&self, _node: &Node, _triggers: &[Event]) -> Box<dyn Span> {
        Box::new(NoopSpan)
    }
    /// A node finished firing (TS `onNodeEnd` — the TS impl ends the
    /// span, so the default does too).
    fn on_node_end(
        &self,
        _node: &Node,
        _events: &[Event],
        _error: Option<&str>,
        span: &mut dyn Span,
    ) {
        span.end();
    }
    /// An event was appended to the log (TS `onEvent`).
    fn on_event(&self, _event: &Event) {}
    /// Node specs were injected mid-run (TS `onInject`).
    fn on_inject(&self, _node_specs: &[Value]) {}
    /// A node retired via `retire_on` (TS `onRetire`).
    fn on_retire(&self, _node_id: &str) {}
}

/// The do-nothing observer — the TS `NoopObserver` equivalent.
pub struct NoopObserver;

impl Observer for NoopObserver {}
