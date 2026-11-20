//! peregrin-telemetry — the ADR-0040 provider-based observability
//! bridge: `--tracing`/`--metrics` CLI flags resolve to [`Observer`]
//! impls that turn engine lifecycle events into OTLP spans and
//! metrics.
//!
//! Two signals, each a [`SignalSpec`]:
//!
//! - `--tracing <dest>` / `--tracing-provider <provider>` — live
//!   spans. `otlp` exports OTLP/HTTP trace spans (node fires become
//!   `node.<id>` spans under a `campaign.run` root; the node's
//!   `Span::traceparent` propagates `ctx.traceparent` into drivers).
//! - `--metrics <dest>` / `--metrics-provider <provider>` — counters
//!   and histograms over the same lifecycle hooks (runs by outcome,
//!   node fires by outcome, event/run-size histograms).
//!
//! The `provider:dest` shorthand is accepted on the flag itself
//! (`--tracing otlp:http://collector:4318/v1/traces`); a bare `--<signal>`
//! value defaults to the `otlp` provider, and `--<signal>-provider
//! noop` (or a `noop:` prefix) disables that signal. When both
//! signals resolve to real providers the observers compose through
//! [`peregrin_bus::CompositeObserver`]. Absent flags →
//! [`NoopObserver`], exactly the pre-flag behavior.
//!
//! This complements — not replaces — `hypha-telemetry`, which is the
//! `tracing`-log → OTLP *span* layer for the daemons. This crate owns
//! the *engine-event* → OTEL bridge the `Observer` port describes.

mod otel;
mod spec;

use std::sync::Arc;

use opentelemetry::metrics::MeterProvider as _;
use opentelemetry::trace::TracerProvider as _;
use peregrin_bus::{CompositeObserver, NoopObserver, Observer};

pub use spec::{Observability, Provider, SignalSpec, TelemetryError};

/// Keeps the SDK providers alive for the process's export lifetime;
/// `Drop` shuts each down (batch flush + exporter close). Inert when
/// both signals resolved to `noop`.
#[derive(Debug)]
pub struct TelemetryGuard {
    tracer: Option<opentelemetry_sdk::trace::SdkTracerProvider>,
    meter: Option<opentelemetry_sdk::metrics::SdkMeterProvider>,
}

impl TelemetryGuard {
    /// `true` when at least one signal is exporting.
    pub fn is_enabled(&self) -> bool {
        self.tracer.is_some() || self.meter.is_some()
    }
}

impl Drop for TelemetryGuard {
    fn drop(&mut self) {
        if let Some(provider) = self.tracer.take() {
            if let Err(e) = provider.shutdown() {
                tracing::warn!("peregrin-telemetry: tracer shutdown failed: {e:?}");
            }
        }
        if let Some(provider) = self.meter.take() {
            if let Err(e) = provider.shutdown() {
                tracing::warn!("peregrin-telemetry: meter shutdown failed: {e:?}");
            }
        }
    }
}

/// The observer half of [`Observability`] — the composed
/// `Arc<dyn Observer>` the runner/bus wiring takes, plus the guard
/// that must outlive it (dropping the guard flushes and shuts the
/// providers down).
///
/// # Errors
/// [`TelemetryError`] when an `otlp` provider's exporter fails to
/// build.
pub fn build_observer(
    observability: &Observability,
) -> Result<(Arc<dyn Observer>, TelemetryGuard), TelemetryError> {
    let mut delegates: Vec<Arc<dyn Observer>> = Vec::new();
    let mut tracer = None;
    let mut meter = None;

    if let Some(spec) = observability.tracing() {
        if spec.provider == Provider::Otlp {
            let provider = otel::tracer_provider(spec.endpoint())?;
            delegates.push(Arc::new(otel::OtelObserver::new(
                provider.tracer("peregrin-engine"),
            )));
            tracer = Some(provider);
        }
    }
    if let Some(spec) = observability.metrics() {
        if spec.provider == Provider::Otlp {
            let provider = otel::meter_provider(spec.endpoint())?;
            delegates.push(Arc::new(otel::OtelMetricsObserver::new(
                provider.meter("peregrin"),
            )));
            meter = Some(provider);
        }
    }

    let observer: Arc<dyn Observer> = match delegates.len() {
        0 => Arc::new(NoopObserver),
        1 => delegates.pop().unwrap_or_else(|| Arc::new(NoopObserver)),
        _ => Arc::new(CompositeObserver::new(delegates)),
    };
    Ok((observer, TelemetryGuard { tracer, meter }))
}
