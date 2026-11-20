//! The OTEL [`Observer`] impls — engine lifecycle events → OTLP
//! spans ([`OtelObserver`], the `--tracing` signal) and metrics
//! ([`OtelMetricsObserver`], the `--metrics` signal), plus the
//! provider builders [`crate::build_observer`] wires them through.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use opentelemetry::trace::{Span as _, SpanKind, Status, TraceContextExt, Tracer as _};
use opentelemetry::{Context, KeyValue};
use opentelemetry_sdk::metrics::SdkMeterProvider;
use opentelemetry_sdk::trace::{SdkTracer, SdkTracerProvider};
use opentelemetry_sdk::{Resource, trace::Span as SdkSpan};
use peregrin_bus::{Observer, Span};
use peregrin_core::{Campaign, Event, Node};
use serde_json::Value;

use crate::TelemetryError;

const SERVICE_NAME: &str = "peregrin";
/// OTLP/HTTP signal paths appended to base-URL endpoints (mirrors
/// `OTEL_EXPORTER_OTLP_ENDPOINT` semantics, same as hypha-telemetry).
const TRACES_PATH: &str = "/v1/traces";
const METRICS_PATH: &str = "/v1/metrics";

/// `base` → the signal's full URL: verbatim when it already names
/// the path, else the path appended to the trimmed base.
fn signal_url(base: &str, path: &str) -> String {
    let base = base.trim().trim_end_matches('/');
    if base.ends_with(path) {
        base.to_string()
    } else {
        format!("{base}{path}")
    }
}

fn resource() -> Resource {
    Resource::builder().with_service_name(SERVICE_NAME).build()
}

/// Build the tracing provider for an `otlp` tracing spec —
/// `endpoint` is the flag's raw destination (`None` lets the
/// exporter resolve `OTEL_EXPORTER_OTLP_*` itself).
///
/// # Errors
/// [`TelemetryError::Exporter`] when the OTLP exporter fails to
/// build.
pub fn tracer_provider(endpoint: Option<&str>) -> Result<SdkTracerProvider, TelemetryError> {
    use opentelemetry_otlp::WithExportConfig;
    let mut builder = opentelemetry_otlp::SpanExporter::builder().with_http();
    if let Some(dest) = endpoint {
        builder = builder.with_endpoint(signal_url(dest, TRACES_PATH));
    }
    let exporter = builder.build().map_err(|source| TelemetryError::Exporter {
        signal: "tracing",
        source,
    })?;
    Ok(SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(resource())
        .build())
}

/// Build the metrics provider for an `otlp` metrics spec.
///
/// # Errors
/// [`TelemetryError::Exporter`] when the OTLP exporter fails to
/// build.
pub fn meter_provider(endpoint: Option<&str>) -> Result<SdkMeterProvider, TelemetryError> {
    use opentelemetry_otlp::WithExportConfig;
    let mut builder = opentelemetry_otlp::MetricExporter::builder().with_http();
    if let Some(dest) = endpoint {
        builder = builder.with_endpoint(signal_url(dest, METRICS_PATH));
    }
    let exporter = builder.build().map_err(|source| TelemetryError::Exporter {
        signal: "metrics",
        source,
    })?;
    let reader = opentelemetry_sdk::metrics::PeriodicReader::builder(exporter).build();
    Ok(SdkMeterProvider::builder()
        .with_reader(reader)
        .with_resource(resource())
        .build())
}

/// `usize` → `i64` attribute value, saturating (a count can never
/// legitimately exceed `i64::MAX`).
fn count(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

/// `serde_json::Value` → an OTEL attribute; objects/arrays/null
/// collapse to their JSON string form (attributes are scalar).
fn attribute(key: &str, value: &Value) -> KeyValue {
    // `Key` is `Cow<'static, str>` — the caller's `&str` isn't
    // 'static, so own it.
    let key = key.to_string();
    match value {
        Value::Bool(b) => KeyValue::new(key, *b),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                KeyValue::new(key, i)
            } else if let Some(u) = n.as_u64() {
                KeyValue::new(key, i64::try_from(u).unwrap_or(i64::MAX))
            } else {
                KeyValue::new(key, n.as_f64().unwrap_or_default())
            }
        }
        Value::String(s) => KeyValue::new(key, s.clone()),
        Value::Null => KeyValue::new(key, "null"),
        other => KeyValue::new(key, other.to_string()),
    }
}

/// The W3C `traceparent` for a span context — `00-<trace>-<span>-<flags>`.
/// Empty when the context isn't valid (unsampled/noop spans).
fn traceparent_of(context: &opentelemetry::trace::SpanContext) -> String {
    if !context.is_valid() {
        return String::new();
    }
    let flags = if context.is_sampled() { "01" } else { "00" };
    format!(
        "00-{:032x}-{:016x}-{flags}",
        context.trace_id(),
        context.span_id(),
    )
}

/// The `--tracing` observer — engine lifecycle → OTLP spans.
///
/// - `on_workflow_start` opens a `campaign.run` root span (the run's
///   parent context node spans hang off).
/// - `on_node_start` opens a `node.<id>` child span carrying
///   `node.id`/`node.driver`/`trigger.count` attributes; its
///   [`Span::traceparent`] feeds `ctx.traceparent` so drivers can
///   propagate the trace into subprocesses/HTTP calls.
/// - `on_node_end` records the produced-event count and any error,
///   then ends the span (also the default when unoverridden).
/// - `on_workflow_end` ends the root span with an error status on
///   failure/cancel.
///
/// `on_event` stays a no-op — per-event span events would flood the
/// trace; the metrics observer owns event counting.
pub struct OtelObserver {
    tracer: SdkTracer,
    /// The in-flight run's root span, held as the parent
    /// [`Context`] node spans are built against. `None` between
    /// runs (and for observers shared across concurrent runs — each
    /// `on_workflow_start` replaces it, matching the sequential
    /// runner's one-run-at-a-time shape).
    workflow: Mutex<Option<Context>>,
}

impl OtelObserver {
    /// An observer exporting through `tracer` (a
    /// [`SdkTracerProvider::tracer`] handle).
    #[must_use]
    pub fn new(tracer: SdkTracer) -> Self {
        Self {
            tracer,
            workflow: Mutex::new(None),
        }
    }

    fn parent_context(&self) -> Context {
        self.workflow
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
            .unwrap_or_default()
    }
}

impl Observer for OtelObserver {
    fn on_workflow_start(&self, campaign: &Campaign) {
        let mut span = self.tracer.start("campaign.run");
        span.set_attributes(vec![
            KeyValue::new("campaign.name", campaign.name.clone()),
            KeyValue::new("campaign.nodes", count(campaign.nodes.len())),
        ]);
        *self
            .workflow
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) =
            Some(Context::new().with_span(span));
    }

    fn on_workflow_end(&self, _event_count: usize, cancelled: bool, _cycle: bool, failed: bool) {
        let cx = self
            .workflow
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        if let Some(cx) = cx {
            if failed {
                cx.span().set_status(Status::error("workflow failed"));
            } else if cancelled {
                cx.span().set_status(Status::error("workflow cancelled"));
            }
            cx.span().end();
        }
    }

    fn on_node_start(&self, node: &Node, triggers: &[Event]) -> Box<dyn Span> {
        let parent = self.parent_context();
        let mut span = self.tracer.build_with_context(
            opentelemetry::trace::SpanBuilder::from_name(format!("node.{}", node.id))
                .with_kind(SpanKind::Internal)
                .with_attributes(vec![
                    KeyValue::new("node.id", node.id.clone()),
                    KeyValue::new("node.driver", node.driver.clone()),
                    KeyValue::new("trigger.count", count(triggers.len())),
                ]),
            &parent,
        );
        if !node.handler.is_empty() {
            span.set_attribute(KeyValue::new("node.handler", node.handler.clone()));
        }
        Box::new(OtelSpan { inner: span })
    }

    fn on_node_end(
        &self,
        _node: &Node,
        events: &[Event],
        error: Option<&str>,
        span: &mut dyn Span,
    ) {
        span.set_attribute("node.events", &Value::from(events.len() as u64));
        if let Some(message) = error {
            span.record_error(message);
        }
        span.end();
    }

    fn on_inject(&self, node_specs: &[Value]) {
        if let Some(cx) = self
            .workflow
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_ref()
        {
            cx.span().add_event(
                "inject",
                vec![KeyValue::new("node_specs", count(node_specs.len()))],
            );
        }
    }

    fn on_retire(&self, node_id: &str) {
        if let Some(cx) = self
            .workflow
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_ref()
        {
            cx.span().add_event(
                "retire",
                vec![KeyValue::new("node.id", node_id.to_string())],
            );
        }
    }
}

/// The OTEL span half of [`OtelObserver::on_node_start`] — wraps the
/// SDK span and translates the port's calls onto it.
struct OtelSpan {
    inner: SdkSpan,
}

impl Span for OtelSpan {
    fn end(&mut self) {
        self.inner.end();
    }

    fn set_attribute(&mut self, key: &str, value: &Value) {
        self.inner.set_attribute(attribute(key, value));
    }

    fn record_error(&mut self, message: &str) {
        self.inner.set_status(Status::error(message.to_string()));
        self.inner.add_event(
            "exception",
            vec![KeyValue::new("exception.message", message.to_string())],
        );
    }

    fn traceparent(&self) -> String {
        traceparent_of(self.inner.span_context())
    }
}

/// The `--metrics` observer — engine lifecycle → OTLP metrics.
///
/// Counters: `peregrin.runs` (by `outcome`), `peregrin.nodes.fired`
/// (by `outcome`), `peregrin.events` (by `kind`). Histograms:
/// `peregrin.run.events`, `peregrin.run.duration_seconds`. Node/run
/// ids stay out of metric attributes — `node.driver` and `kind` are
/// the bounded label sets.
pub struct OtelMetricsObserver {
    meter: opentelemetry::metrics::Meter,
    started: Mutex<Option<Instant>>,
}

impl OtelMetricsObserver {
    /// An observer recording through `meter` (a
    /// [`SdkMeterProvider::meter`] handle).
    #[must_use]
    pub fn new(meter: opentelemetry::metrics::Meter) -> Self {
        Self {
            meter,
            started: Mutex::new(None),
        }
    }

    fn counter(&self, name: &'static str) -> opentelemetry::metrics::Counter<u64> {
        self.meter.u64_counter(name).build()
    }

    fn histogram(&self, name: &'static str) -> opentelemetry::metrics::Histogram<f64> {
        self.meter.f64_histogram(name).build()
    }
}

impl Observer for OtelMetricsObserver {
    fn on_workflow_start(&self, _campaign: &Campaign) {
        *self
            .started
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(Instant::now());
    }

    fn on_workflow_end(&self, event_count: usize, cancelled: bool, _cycle: bool, failed: bool) {
        let outcome = if failed {
            "failed"
        } else if cancelled {
            "cancelled"
        } else {
            "completed"
        };
        self.counter("peregrin.runs")
            .add(1, &[KeyValue::new("outcome", outcome)]);
        self.histogram("peregrin.run.events")
            .record(event_count as f64, &[]);
        let elapsed = self
            .started
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take()
            .map_or(Duration::ZERO, |started| started.elapsed());
        self.histogram("peregrin.run.duration_seconds")
            .record(elapsed.as_secs_f64(), &[]);
    }

    fn on_node_end(
        &self,
        node: &Node,
        _events: &[Event],
        error: Option<&str>,
        span: &mut dyn Span,
    ) {
        let outcome = if error.is_some() {
            "failed"
        } else {
            "completed"
        };
        self.counter("peregrin.nodes.fired").add(
            1,
            &[
                KeyValue::new("outcome", outcome),
                KeyValue::new("node.driver", node.driver.clone()),
            ],
        );
        // The default `on_node_end` contract ends the span — the
        // metrics observer returns `NoopSpan`s, but honor it anyway.
        span.end();
    }

    fn on_event(&self, event: &Event) {
        self.counter("peregrin.events")
            .add(1, &[KeyValue::new("kind", event.kind.clone())]);
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::items_after_statements
    )]
    use super::*;
    use opentelemetry::metrics::MeterProvider as _;
    use opentelemetry::trace::TracerProvider as _;
    use opentelemetry_sdk::metrics::PeriodicReader;
    use opentelemetry_sdk::trace::InMemorySpanExporterBuilder;
    use peregrin_bus::NoopSpan;

    fn campaign() -> Campaign {
        Campaign {
            name: "test-campaign".to_string(),
            nodes: vec![Node {
                id: "n1".to_string(),
                driver: "shell".to_string(),
                ..Node::default()
            }],
            ..Campaign::default()
        }
    }

    fn node() -> Node {
        Node {
            id: "n1".to_string(),
            driver: "shell".to_string(),
            ..Node::default()
        }
    }

    #[test]
    fn signal_url_appends_path_once() {
        assert_eq!(
            signal_url("http://c:4318/", TRACES_PATH),
            "http://c:4318/v1/traces"
        );
        assert_eq!(
            signal_url("http://c:4318/v1/traces", TRACES_PATH),
            "http://c:4318/v1/traces"
        );
        assert_eq!(
            signal_url("http://c:4318", METRICS_PATH),
            "http://c:4318/v1/metrics"
        );
    }

    #[test]
    fn otel_observer_emits_parented_spans() {
        let exporter = InMemorySpanExporterBuilder::default().build();
        let provider = SdkTracerProvider::builder()
            .with_simple_exporter(exporter.clone())
            .build();
        let observer = OtelObserver::new(provider.tracer("test"));
        let campaign = campaign();
        let node = node();
        let triggers = vec![Event::new("start", Value::Null, "__runner__")];

        observer.on_workflow_start(&campaign);
        let mut span = observer.on_node_start(&node, &triggers);
        let traceparent = span.traceparent();
        assert!(traceparent.starts_with("00-"), "traceparent: {traceparent}");
        span.set_attribute("custom", &Value::from("v"));
        span.record_error("boom");
        observer.on_node_end(&node, &[], Some("boom"), &mut *span);
        observer.on_workflow_end(0, false, false, true);
        provider.force_flush().expect("flush");

        let spans = exporter.get_finished_spans().expect("spans");
        assert_eq!(spans.len(), 2, "{spans:?}");
        let root = spans
            .iter()
            .find(|s| s.name == "campaign.run")
            .expect("root span");
        let child = spans
            .iter()
            .find(|s| s.name == "node.n1")
            .expect("node span");
        assert_eq!(child.parent_span_id, root.span_context.span_id());
        assert_eq!(child.span_context.trace_id(), root.span_context.trace_id());
        assert!(matches!(
            root.status,
            opentelemetry::trace::Status::Error { .. }
        ));
        assert!(child.attributes.iter().any(|kv| {
            kv.key.as_str() == "node.driver" && kv.value == opentelemetry::Value::from("shell")
        }));
        assert!(child.attributes.iter().any(|kv| {
            kv.key.as_str() == "custom" && kv.value == opentelemetry::Value::from("v")
        }));
        assert!(child.events.events.iter().any(|e| e.name == "exception"));
        // traceparent carried the node span's own ids.
        let tp = traceparent_of(&child.span_context);
        assert_eq!(traceparent, tp);
    }

    #[test]
    fn otel_observer_without_workflow_still_spans_nodes() {
        let exporter = InMemorySpanExporterBuilder::default().build();
        let provider = SdkTracerProvider::builder()
            .with_simple_exporter(exporter.clone())
            .build();
        let observer = OtelObserver::new(provider.tracer("test"));
        let node = node();
        let mut span = observer.on_node_start(&node, &[]);
        observer.on_node_end(&node, &[], None, &mut *span);
        provider.force_flush().expect("flush");
        let spans = exporter.get_finished_spans().expect("spans");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].name, "node.n1");
    }

    #[test]
    fn otel_metrics_observer_records_counters_and_histograms() {
        let exporter = opentelemetry_sdk::metrics::InMemoryMetricExporter::default();
        let reader = PeriodicReader::builder(exporter.clone()).build();
        let provider = SdkMeterProvider::builder().with_reader(reader).build();
        let observer = OtelMetricsObserver::new(provider.meter("peregrin"));
        let campaign = campaign();
        let node = node();
        let event = Event::new("node.done", Value::Null, "n1");

        observer.on_workflow_start(&campaign);
        observer.on_event(&event);
        observer.on_event(&Event::new("node.failed", Value::Null, "n1"));
        let mut noop = NoopSpan;
        observer.on_node_end(&node, &[], Some("boom"), &mut noop);
        observer.on_workflow_end(2, false, false, true);
        provider.force_flush().expect("flush");

        let metrics = exporter.get_finished_metrics().expect("metrics");
        let names: Vec<&str> = metrics
            .iter()
            .flat_map(opentelemetry_sdk::metrics::data::ResourceMetrics::scope_metrics)
            .flat_map(opentelemetry_sdk::metrics::data::ScopeMetrics::metrics)
            .map(opentelemetry_sdk::metrics::data::Metric::name)
            .collect();
        for expected in [
            "peregrin.runs",
            "peregrin.nodes.fired",
            "peregrin.events",
            "peregrin.run.events",
            "peregrin.run.duration_seconds",
        ] {
            assert!(names.contains(&expected), "missing {expected} in {names:?}");
        }
    }
}
