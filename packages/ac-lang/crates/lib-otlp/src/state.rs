use serde_json::Value;
use std::collections::HashMap;

use crate::error::OtlpResult;

/// Configuration for an OTLP exporter.
#[derive(Debug, Clone, Default)]
pub struct OtlpConfig {
    /// Base OTLP endpoint, e.g. `http://localhost:4318`.
    pub endpoint: String,
    /// Service name added as a `service.name` resource attribute.
    pub service_name: String,
    /// Additional resource attributes.
    pub resource_attributes: HashMap<String, Value>,
    /// Headers added to every OTLP/HTTP request.
    pub headers: HashMap<String, String>,
}

impl OtlpConfig {
    /// Create a minimal config from an endpoint and a service name.
    pub fn new(endpoint: impl Into<String>, service_name: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            service_name: service_name.into(),
            ..Default::default()
        }
    }
}

/// Options passed to [`OtlpState::start_span`].
#[derive(Debug, Clone, Default)]
pub struct StartSpanOptions {
    /// Parent span id, if any.
    pub parent: Option<String>,
    /// Initial span attributes.
    pub attributes: HashMap<String, Value>,
}

/// Kind of metric recorded by [`OtlpState::record_metric`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricKind {
    /// Monotonically increasing counter.
    Counter,
    /// Gauge value.
    Gauge,
}

impl MetricKind {
    /// Parse a metric kind from a string.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "counter" | "Counter" => Some(Self::Counter),
            "gauge" | "Gauge" => Some(Self::Gauge),
            _ => None,
        }
    }
}

/// Options passed to [`OtlpState::record_metric`].
#[derive(Debug, Clone)]
pub struct MetricOptions {
    /// Counter or gauge.
    pub kind: MetricKind,
    /// Metric attributes.
    pub attributes: HashMap<String, Value>,
}

impl Default for MetricOptions {
    fn default() -> Self {
        Self {
            kind: MetricKind::Counter,
            attributes: HashMap::new(),
        }
    }
}

impl MetricOptions {
    /// Counter with the given attributes.
    pub fn counter(attributes: HashMap<String, Value>) -> Self {
        Self {
            kind: MetricKind::Counter,
            attributes,
        }
    }
    /// Gauge with the given attributes.
    pub fn gauge(attributes: HashMap<String, Value>) -> Self {
        Self {
            kind: MetricKind::Gauge,
            attributes,
        }
    }
}

/// Abstract OpenTelemetry state that supports spans, logs and metrics.
pub trait OtlpState: Send + Sync {
    /// Configure the exporter.
    fn configure(&self, config: OtlpConfig) -> OtlpResult<()>;

    /// Start a new span and return its span id.
    fn start_span(&self, name: &str, options: StartSpanOptions) -> OtlpResult<String>;

    /// End the span with the given id.
    fn end_span(&self, id: &str) -> OtlpResult<()>;

    /// Set an attribute on an existing span.
    fn set_attribute(&self, id: &str, key: &str, value: Value) -> OtlpResult<()>;

    /// Add a named event to an existing span.
    fn add_event(
        &self,
        id: &str,
        name: &str,
        attributes: HashMap<String, Value>,
    ) -> OtlpResult<()>;

    /// Record a log entry.
    fn record_log(
        &self,
        level: &str,
        message: &str,
        attributes: HashMap<String, Value>,
    ) -> OtlpResult<()>;

    /// Record a metric data point.
    fn record_metric(&self, name: &str, value: f64, options: MetricOptions) -> OtlpResult<()>;

    /// Flush all buffered telemetry to the configured exporter.
    fn flush(&self) -> OtlpResult<()>;

    /// Flush and release the exporter.
    fn shutdown(&self) -> OtlpResult<()>;
}
