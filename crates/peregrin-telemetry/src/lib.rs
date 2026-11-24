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
//! The flag model ([`Observability`]/[`SignalSpec`]/[`Provider`]) and
//! the OTLP provider construction live in **lampy** (P-042/PL-023 —
//! telemetry expertise consolidated in 9aia/lampy); this crate keeps
//! only the engine-event → OTEL [`Observer`] impls, which are
//! peregrin-domain by nature.

mod otel;

use std::sync::Arc;

use opentelemetry::metrics::MeterProvider as _;
use opentelemetry::trace::TracerProvider as _;
use peregrin_bus::{CompositeObserver, NoopObserver, Observer};

pub use lampy_telemetry::{Observability, Provider, SignalSpec, TelemetryError};
pub use lampy_telemetry_otel::TelemetryGuard;

/// The env contract flag-driven providers resolve against —
/// canonical `LAMPY_OTEL_*`, then the legacy `PEREGRIN_OTEL_*` and
/// `HYPHA_OTEL_*` prefixes (peregrin rode hypha-telemetry's env
/// contract before PL-023), then the standard `OTEL_*` vars.
fn telemetry_spec() -> lampy_telemetry::TelemetrySpec {
    lampy_telemetry::TelemetrySpec::service("peregrin")
        .with_alias("PEREGRIN")
        .with_alias("HYPHA")
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
    let spec = telemetry_spec();
    let mut delegates: Vec<Arc<dyn Observer>> = Vec::new();
    let mut tracer = None;
    let mut meter = None;

    if let Some(signal) = observability.tracing()
        && signal.provider == Provider::Otlp
    {
        let provider = lampy_telemetry_otel::tracer_provider(&spec, signal.endpoint())?;
        delegates.push(Arc::new(otel::OtelObserver::new(
            provider.tracer("peregrin-engine"),
        )));
        tracer = Some(provider);
    }
    if let Some(signal) = observability.metrics()
        && signal.provider == Provider::Otlp
    {
        let provider = lampy_telemetry_otel::meter_provider(&spec, signal.endpoint())?;
        delegates.push(Arc::new(otel::OtelMetricsObserver::new(
            provider.meter("peregrin"),
        )));
        meter = Some(provider);
    }

    let observer: Arc<dyn Observer> = match delegates.len() {
        0 => Arc::new(NoopObserver),
        1 => delegates.pop().unwrap_or_else(|| Arc::new(NoopObserver)),
        _ => Arc::new(CompositeObserver::new(delegates)),
    };
    Ok((observer, TelemetryGuard::from_providers(tracer, meter)))
}
