//! # handl-lib-otlp-std
//!
//! Concrete OTLP/HTTP exporter using `ureq`.
//!
//! Provides [`HttpOtlpState`] and a preconfigured
//! [`StdOtlpModule`] ready for use with `ModuleHost`.

mod http_state;

pub use handl_lib_otlp::{
    MetricKind, MetricOptions, OtlpConfig, OtlpError, OtlpEvent, OtlpModule, OtlpResult, OtlpState,
    RecordedEvent, RecordedSpan, StartSpanOptions, StubOtlpState,
};
pub use http_state::{HttpOtlpState, std_otlp_module};
