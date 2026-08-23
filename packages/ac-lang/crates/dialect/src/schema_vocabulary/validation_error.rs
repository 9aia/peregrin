use ac_source::SourceLocation;

/// A validation error.
///
/// Carries a human-readable message, a dotted path to the
/// offending value within the document (empty string for root),
/// and an optional source location (line:column).
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationError {
    /// Human-readable description of the violation.
    pub message: String,

    /// Dotted path to the offending value within the document.
    /// Empty string for the root.
    pub path: String,

    /// Source location (line:column) of the offending value, if known.
    /// Populated by the compiler after validation, using the original
    /// source text.
    pub location: Option<SourceLocation>}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(loc) = self.location {
            if self.path.is_empty() {
                write!(f, "[{loc}] {}", self.message)
            } else {
                write!(f, "[{loc}] {}: {}", self.path, self.message)
            }
        } else if self.path.is_empty() {
            write!(f, "{}", self.message)
        } else {
            write!(f, "{}: {}", self.path, self.message)
        }
    }
}

impl std::error::Error for ValidationError {}
