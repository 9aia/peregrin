//! Unified error type for passes.
//!
//! [`PassError`] replaces `String` as the error type in [`Pass::run`](crate::Pass::run).
//! It carries structured diagnostics and a pass name, giving callers
//! actionable information instead of opaque strings.

use crate::diagnostic::{Diagnostic, Severity};

/// Error returned by a pass.
///
/// Carries the pass name and one or more diagnostics. Callers can
/// inspect severity, source locations, and diagnostic codes.
#[derive(Clone, Debug)]
pub struct PassError {
    /// Name of the pass that produced this error.
    pub pass: &'static str,
    /// Diagnostics describing the error.
    pub diagnostics: Vec<Diagnostic>,
}

impl PassError {
    /// Create a single-error `PassError` from a message.
    pub fn new(pass: &'static str, message: impl Into<String>) -> Self {
        PassError {
            pass,
            diagnostics: vec![Diagnostic::error(message)],
        }
    }

    /// Create a `PassError` from a diagnostic.
    pub fn from_diagnostic(pass: &'static str, diagnostic: Diagnostic) -> Self {
        PassError {
            pass,
            diagnostics: vec![diagnostic],
        }
    }

    /// Create a `PassError` from multiple diagnostics.
    pub fn from_diagnostics(pass: &'static str, diagnostics: Vec<Diagnostic>) -> Self {
        PassError { pass, diagnostics }
    }

    /// Whether this error contains any error-severity diagnostics.
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
    }

    /// Iterator over error-severity diagnostics.
    pub fn errors(&self) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
    }

    /// Convert to a human-readable string (for backwards compatibility).
    pub fn to_string(&self) -> String {
        self.diagnostics
            .iter()
            .map(|d| d.message.as_str())
            .collect::<Vec<_>>()
            .join("; ")
    }
}

impl std::fmt::Display for PassError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.pass, self.to_string())
    }
}

impl std::error::Error for PassError {}

impl From<String> for PassError {
    fn from(s: String) -> Self {
        PassError::new("unknown", s)
    }
}

impl From<&str> for PassError {
    fn from(s: &str) -> Self {
        PassError::new("unknown", s)
    }
}
