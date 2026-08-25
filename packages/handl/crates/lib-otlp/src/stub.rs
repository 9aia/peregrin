use std::collections::HashMap;
use std::sync::Mutex;

use serde_json::Value;

use crate::error::OtlpResult;
use crate::id;
use crate::state::{MetricOptions, OtlpConfig, OtlpState, StartSpanOptions};

/// A span that was recorded by the stub.
#[derive(Debug, Clone)]
pub struct RecordedSpan {
    /// Span id.
    pub id: String,
    /// Trace id.
    pub trace_id: String,
    /// Parent span id, if any.
    pub parent_id: Option<String>,
    /// Span name.
    pub name: String,
    /// Span attributes.
    pub attributes: HashMap<String, Value>,
    /// Events recorded on the span.
    pub events: Vec<RecordedEvent>,
}

/// A span event that was recorded by the stub.
#[derive(Debug, Clone)]
pub struct RecordedEvent {
    /// Event name.
    pub name: String,
    /// Event attributes.
    pub attributes: HashMap<String, Value>,
}

/// Events recorded by [`StubOtlpState`].
#[derive(Debug, Clone)]
pub enum OtlpEvent {
    /// `configure` was called.
    Configured(OtlpConfig),
    /// A span was started.
    SpanStarted(RecordedSpan),
    /// A span was ended.
    SpanEnded { id: String },
    /// A span attribute was set.
    SpanAttribute {
        id: String,
        key: String,
        value: Value,
    },
    /// A span event was added.
    SpanEvent {
        id: String,
        name: String,
        attributes: HashMap<String, Value>,
    },
    /// A log was recorded.
    LogRecorded {
        level: String,
        message: String,
        attributes: HashMap<String, Value>,
    },
    /// A metric was recorded.
    MetricRecorded {
        name: String,
        value: f64,
        options: MetricOptions,
    },
    /// `flush` was called.
    Flushed,
    /// `shutdown` was called.
    Shutdown,
}

/// In-memory OTLP state for testing.
///
/// Records every operation as an [`OtlpEvent`]. No network is performed.
#[derive(Default)]
pub struct StubOtlpState {
    events: Mutex<Vec<OtlpEvent>>,
}

impl StubOtlpState {
    /// Create a new stub.
    pub fn new() -> Self {
        Self::default()
    }

    /// All recorded events, in order.
    pub fn events(&self) -> Vec<OtlpEvent> {
        self.events.lock().unwrap().clone()
    }

    fn record(&self, event: OtlpEvent) {
        self.events.lock().unwrap().push(event);
    }
}

impl OtlpState for StubOtlpState {
    fn configure(&self, config: OtlpConfig) -> OtlpResult<()> {
        self.record(OtlpEvent::Configured(config));
        Ok(())
    }

    fn start_span(&self, name: &str, options: StartSpanOptions) -> OtlpResult<String> {
        let trace_id = options
            .parent
            .as_ref()
            .map(|_| id::gen_trace_id())
            .unwrap_or_else(id::gen_trace_id);
        let id = id::gen_span_id();
        self.record(OtlpEvent::SpanStarted(RecordedSpan {
            id: id.clone(),
            trace_id,
            parent_id: options.parent,
            name: name.to_string(),
            attributes: options.attributes,
            events: Vec::new(),
        }));
        Ok(id)
    }

    fn end_span(&self, id: &str) -> OtlpResult<()> {
        self.record(OtlpEvent::SpanEnded { id: id.to_string() });
        Ok(())
    }

    fn set_attribute(&self, id: &str, key: &str, value: Value) -> OtlpResult<()> {
        self.record(OtlpEvent::SpanAttribute {
            id: id.to_string(),
            key: key.to_string(),
            value,
        });
        Ok(())
    }

    fn add_event(
        &self,
        id: &str,
        name: &str,
        attributes: HashMap<String, Value>,
    ) -> OtlpResult<()> {
        self.record(OtlpEvent::SpanEvent {
            id: id.to_string(),
            name: name.to_string(),
            attributes,
        });
        Ok(())
    }

    fn record_log(
        &self,
        level: &str,
        message: &str,
        attributes: HashMap<String, Value>,
    ) -> OtlpResult<()> {
        self.record(OtlpEvent::LogRecorded {
            level: level.to_string(),
            message: message.to_string(),
            attributes,
        });
        Ok(())
    }

    fn record_metric(&self, name: &str, value: f64, options: MetricOptions) -> OtlpResult<()> {
        self.record(OtlpEvent::MetricRecorded {
            name: name.to_string(),
            value,
            options,
        });
        Ok(())
    }

    fn flush(&self) -> OtlpResult<()> {
        self.record(OtlpEvent::Flushed);
        Ok(())
    }

    fn shutdown(&self) -> OtlpResult<()> {
        self.record(OtlpEvent::Shutdown);
        Ok(())
    }
}
