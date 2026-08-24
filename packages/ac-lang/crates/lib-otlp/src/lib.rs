//! # ac-lib-otlp
//!
//! OpenTelemetry/OTLP module for the AC runtime.
//!
//! Defines an abstract [`OtlpState`] and a [`Module`] adapter that
//! exposes `otlp.*` functions to AC programs. Concrete exporters live
//! in `ac-lib-otlp-std`.

pub mod error;
pub mod id;
pub mod state;
pub mod stub;
pub mod otlp_module;

pub use error::{OtlpError, OtlpResult};
pub use id::{gen_span_id, gen_trace_id, now_ns};
pub use state::{MetricKind, MetricOptions, OtlpConfig, OtlpState, StartSpanOptions};
pub use stub::{OtlpEvent, RecordedEvent, RecordedSpan, StubOtlpState};
pub use otlp_module::OtlpModule;
