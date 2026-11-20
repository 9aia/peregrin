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

/// One delegate's span inside a [`CompositeSpan`], shared between
/// the span the runner holds and the per-node slot
/// [`CompositeObserver::on_node_end`] looks up — `set_attribute` /
/// `record_error` / `traceparent` fan out through the composite
/// while `on_node_end` hands each delegate *its own* span back (the
/// default impl ends it; ending the composite would end them all).
type SharedSpan = std::sync::Arc<std::sync::Mutex<Box<dyn Span>>>;

/// A span fanning attribute/error/traceparent calls out to several
/// child spans — [`CompositeObserver::on_node_start`] wraps the
/// per-delegate spans it collected.
struct CompositeSpan {
    spans: Vec<SharedSpan>,
}

impl Span for CompositeSpan {
    fn end(&mut self) {
        for span in &self.spans {
            span.lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .end();
        }
    }

    fn set_attribute(&mut self, key: &str, value: &Value) {
        for span in &self.spans {
            span.lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .set_attribute(key, value);
        }
    }

    fn record_error(&mut self, message: &str) {
        for span in &self.spans {
            span.lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .record_error(message);
        }
    }

    fn traceparent(&self) -> String {
        self.spans
            .iter()
            .find_map(|span| {
                let tp = span
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .traceparent();
                (!tp.is_empty()).then_some(tp)
            })
            .unwrap_or_default()
    }
}

/// Fan-out observer — ADR-0040's `CompositeObserver`: when both
/// `--tracing` and `--metrics` select providers, the CLI composes
/// them here so every hook reaches both delegates. `on_node_start`
/// collects each delegate's span (kept per node id so
/// `on_node_end` can hand delegate *i* its own span back — the
/// trait's `span` argument is a single object); every other hook
/// forwards to all delegates in order.
pub struct CompositeObserver {
    delegates: Vec<std::sync::Arc<dyn Observer>>,
    /// node id → the delegate spans `on_node_start` produced for its
    /// in-flight fire. Entries are removed at `on_node_end`; a node
    /// refiring before its end (impossible in the sequential runner,
    /// but the port doesn't forbid it) replaces the stale entry.
    spans: std::sync::Mutex<std::collections::HashMap<String, Vec<SharedSpan>>>,
}

impl CompositeObserver {
    /// Compose `delegates` — an empty list behaves like
    /// [`NoopObserver`].
    #[must_use]
    pub fn new(delegates: Vec<std::sync::Arc<dyn Observer>>) -> Self {
        Self {
            delegates,
            spans: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }
}

impl Observer for CompositeObserver {
    fn on_workflow_start(&self, campaign: &Campaign) {
        for observer in &self.delegates {
            observer.on_workflow_start(campaign);
        }
    }

    fn on_workflow_end(&self, event_count: usize, cancelled: bool, cycle: bool, failed: bool) {
        for observer in &self.delegates {
            observer.on_workflow_end(event_count, cancelled, cycle, failed);
        }
    }

    fn on_node_start(&self, node: &Node, triggers: &[Event]) -> Box<dyn Span> {
        let spans: Vec<SharedSpan> = self
            .delegates
            .iter()
            .map(|observer| {
                std::sync::Arc::new(std::sync::Mutex::new(
                    observer.on_node_start(node, triggers),
                ))
            })
            .collect();
        self.spans
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(node.id.clone(), spans.clone());
        Box::new(CompositeSpan { spans })
    }

    fn on_node_end(&self, node: &Node, events: &[Event], error: Option<&str>, span: &mut dyn Span) {
        let children = self
            .spans
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(&node.id);
        if let Some(children) = children {
            for (observer, child) in self.delegates.iter().zip(children) {
                let mut guard = child
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                observer.on_node_end(node, events, error, &mut **guard);
            }
        }
        // Fallback end for delegates that didn't end their own span
        // (already-ended spans ignore a second `end`).
        span.end();
    }

    fn on_event(&self, event: &Event) {
        for observer in &self.delegates {
            observer.on_event(event);
        }
    }

    fn on_inject(&self, node_specs: &[Value]) {
        for observer in &self.delegates {
            observer.on_inject(node_specs);
        }
    }

    fn on_retire(&self, node_id: &str) {
        for observer in &self.delegates {
            observer.on_retire(node_id);
        }
    }
}
