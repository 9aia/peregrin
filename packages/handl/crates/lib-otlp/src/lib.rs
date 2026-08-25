//! # handl-lib-otlp
//!
//! OpenTelemetry/OTLP module for the handl runtime.
//!
//! Defines an abstract [`OtlpState`] and a [`Module`] adapter that
//! exposes `otlp.*` functions to handl programs. Concrete exporters live
//! in `handl-lib-otlp-std`.

pub mod error;
pub mod id;
pub mod otlp_module;
pub mod state;
pub mod stub;

pub use error::{OtlpError, OtlpResult};
pub use id::{gen_span_id, gen_trace_id, now_ns};
pub use otlp_module::OtlpModule;
pub use state::{MetricKind, MetricOptions, OtlpConfig, OtlpState, StartSpanOptions};
pub use stub::{OtlpEvent, RecordedEvent, RecordedSpan, StubOtlpState};
