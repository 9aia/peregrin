//! The `--tracing`/`--metrics` flag model — ADR-0040's
//! `provider:dest` shorthand plus the `--<signal>-provider`
//! selector.

use std::fmt;

/// The implementation a signal's destination is interpreted by.
/// `otlp` is the only exporting provider today; `noop` is the
/// explicit off-switch (also the implicit default when the flag is
/// omitted entirely).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    /// OTLP/HTTP export — spans to `/v1/traces`, metrics to
    /// `/v1/metrics`.
    Otlp,
    /// No output — the observer still composes but every hook is a
    /// no-op.
    Noop,
}

impl Provider {
    fn parse(name: &str) -> Option<Self> {
        match name {
            "otlp" => Some(Self::Otlp),
            "noop" => Some(Self::Noop),
            _ => None,
        }
    }
}

/// One resolved signal — which provider interprets `dest`.
#[derive(Debug, Clone)]
pub struct SignalSpec {
    pub provider: Provider,
    /// The raw destination — an OTLP endpoint (base URL or full
    /// signal URL) for `otlp`, ignored for `noop`. `None` lets the
    /// exporter resolve `OTEL_EXPORTER_OTLP_*` itself.
    pub dest: Option<String>,
}

impl SignalSpec {
    /// The destination trimmed to `None` when empty/whitespace.
    pub fn endpoint(&self) -> Option<&str> {
        self.dest
            .as_deref()
            .map(str::trim)
            .filter(|d| !d.is_empty())
    }
}

/// Flag-resolution failure — an unknown `--<signal>-provider` name
/// or a `provider:` prefix that isn't recognized.
#[derive(Debug, thiserror::Error)]
pub enum TelemetryError {
    /// `--<signal>-provider` (or a `provider:` flag prefix) named a
    /// provider this build doesn't ship.
    #[error("unknown {signal} provider '{provider}' (supported: otlp, noop)")]
    UnknownProvider {
        /// Which signal flag was being resolved.
        signal: &'static str,
        /// The unrecognized provider name.
        provider: String,
    },
    /// The OTLP exporter failed to build (bad endpoint, missing
    /// client feature, …).
    #[error("{signal} otlp exporter: {source}")]
    Exporter {
        /// Which signal's exporter failed.
        signal: &'static str,
        /// The underlying build error.
        source: opentelemetry_otlp::ExporterBuildError,
    },
}

impl fmt::Display for SignalSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let provider = match self.provider {
            Provider::Otlp => "otlp",
            Provider::Noop => "noop",
        };
        match self.endpoint() {
            Some(dest) => write!(f, "{provider}:{dest}"),
            None => write!(f, "{provider}"),
        }
    }
}

/// The two observability signals after flag resolution — `None`
/// means the flag was omitted (implicit `noop`).
#[derive(Debug, Clone, Default)]
pub struct Observability {
    tracing: Option<SignalSpec>,
    metrics: Option<SignalSpec>,
}

impl Observability {
    /// Resolve the four CLI flags into signal specs.
    ///
    /// `flag` is the `--<signal>` value, `provider_flag` the
    /// `--<signal>-provider` value. A `provider:` prefix inside
    /// `flag` wins over `provider_flag`; a bare destination defaults
    /// to the `otlp` provider; `noop` as either the prefix or the
    /// provider name disables the signal. `provider_flag` without
    /// `flag` is ignored — there is no destination to interpret.
    ///
    /// # Errors
    /// [`TelemetryError::UnknownProvider`] on an unrecognized
    /// provider name.
    pub fn resolve(
        tracing: Option<&str>,
        tracing_provider: Option<&str>,
        metrics: Option<&str>,
        metrics_provider: Option<&str>,
    ) -> Result<Self, TelemetryError> {
        Ok(Self {
            tracing: resolve_signal("tracing", tracing, tracing_provider)?,
            metrics: resolve_signal("metrics", metrics, metrics_provider)?,
        })
    }

    /// The resolved tracing spec, when `--tracing` was given.
    #[must_use]
    pub fn tracing(&self) -> Option<&SignalSpec> {
        self.tracing.as_ref()
    }

    /// The resolved metrics spec, when `--metrics` was given.
    #[must_use]
    pub fn metrics(&self) -> Option<&SignalSpec> {
        self.metrics.as_ref()
    }
}

/// One signal's flag triple → [`SignalSpec`].
fn resolve_signal(
    signal: &'static str,
    flag: Option<&str>,
    provider_flag: Option<&str>,
) -> Result<Option<SignalSpec>, TelemetryError> {
    let Some(value) = flag.map(str::trim).filter(|v| !v.is_empty()) else {
        return Ok(None);
    };
    // `provider:dest` shorthand — the prefix must name a known
    // provider, otherwise the whole value is the destination (a
    // URL's own `scheme:` prefix must not be mistaken for one).
    if let Some((prefix, rest)) = value.split_once(':')
        && let Some(provider) = Provider::parse(prefix)
    {
        return Ok(Some(SignalSpec {
            provider,
            dest: nonempty(rest),
        }));
    }
    // A bare provider name (`--tracing noop`) selects that provider
    // with no destination.
    if let Some(provider) = Provider::parse(value) {
        return Ok(Some(SignalSpec {
            provider,
            dest: None,
        }));
    }
    let provider = match provider_flag.map(str::trim).filter(|p| !p.is_empty()) {
        Some(name) => Provider::parse(name).ok_or_else(|| TelemetryError::UnknownProvider {
            signal,
            provider: name.to_string(),
        })?,
        None => Provider::Otlp,
    };
    Ok(Some(SignalSpec {
        provider,
        dest: nonempty(value),
    }))
}

fn nonempty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn omitted_flags_resolve_to_none() {
        let obs = Observability::resolve(None, None, None, None).expect("resolve");
        assert!(obs.tracing().is_none());
        assert!(obs.metrics().is_none());
    }

    #[test]
    fn provider_flag_without_flag_is_ignored() {
        let obs = Observability::resolve(None, Some("otlp"), None, Some("noop")).expect("resolve");
        assert!(obs.tracing().is_none());
        assert!(obs.metrics().is_none());
    }

    #[test]
    fn bare_destination_defaults_to_otlp() {
        // `prometheus:` isn't a known provider prefix → the whole
        // value is the destination; with no provider flag it also
        // resolves as otlp (whether that destination is meaningful
        // is the exporter's problem, not the parser's).
        let obs = Observability::resolve(
            Some("http://collector:4318"),
            None,
            Some("prometheus:9090"),
            None,
        )
        .expect("resolve");
        let tracing = obs.tracing().expect("tracing spec");
        assert_eq!(tracing.provider, Provider::Otlp);
        assert_eq!(tracing.endpoint(), Some("http://collector:4318"));
    }

    #[test]
    fn provider_prefix_shorthand() {
        let obs = Observability::resolve(
            Some("otlp:http://collector:4318/v1/traces"),
            None,
            Some("noop:"),
            None,
        )
        .expect("resolve");
        let tracing = obs.tracing().expect("tracing spec");
        assert_eq!(tracing.provider, Provider::Otlp);
        assert_eq!(tracing.endpoint(), Some("http://collector:4318/v1/traces"));
        let metrics = obs.metrics().expect("metrics spec");
        assert_eq!(metrics.provider, Provider::Noop);
        assert_eq!(metrics.endpoint(), None);
    }

    #[test]
    fn bare_provider_name_selects_provider() {
        let obs = Observability::resolve(Some("noop"), None, None, None).expect("resolve");
        assert_eq!(obs.tracing().expect("spec").provider, Provider::Noop);
    }

    #[test]
    fn explicit_provider_flag_wins_over_default() {
        let obs = Observability::resolve(None, None, Some("/tmp/m.jsonl"), Some("noop"))
            .expect("resolve");
        let metrics = obs.metrics().expect("spec");
        assert_eq!(metrics.provider, Provider::Noop);
        assert_eq!(metrics.endpoint(), Some("/tmp/m.jsonl"));
    }

    #[test]
    fn unknown_provider_errors() {
        let err = Observability::resolve(None, None, Some("9090"), Some("prometheus")).unwrap_err();
        assert!(matches!(
            err,
            TelemetryError::UnknownProvider {
                signal: "metrics",
                ..
            }
        ));
        let err = Observability::resolve(Some("/tmp/x"), Some("bogus"), None, None).unwrap_err();
        assert!(matches!(err, TelemetryError::UnknownProvider { .. }));
    }

    #[test]
    fn url_scheme_is_not_a_provider_prefix() {
        // `http://…` must not parse `http` as a provider.
        let obs = Observability::resolve(Some("http://c:4318"), None, None, None).expect("resolve");
        assert_eq!(obs.tracing().expect("spec").provider, Provider::Otlp);
        assert_eq!(
            obs.tracing().expect("spec").endpoint(),
            Some("http://c:4318")
        );
    }

    #[test]
    fn signal_spec_display() {
        let spec = SignalSpec {
            provider: Provider::Otlp,
            dest: Some("http://c:4318".into()),
        };
        assert_eq!(spec.to_string(), "otlp:http://c:4318");
        let spec = SignalSpec {
            provider: Provider::Noop,
            dest: None,
        };
        assert_eq!(spec.to_string(), "noop");
    }
}
