use std::fmt;

/// A byte range in source text — half-open `[start, end)`.
///
/// Spans are byte offsets (0-indexed). Convert to [`SourceLocation`]
/// (1-indexed line:column) when reporting to users.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Span {
    /// Start byte offset (inclusive).
    pub start: usize,
    /// End byte offset (exclusive).
    pub end: usize}

impl Span {
    /// Creates a span from `[start, end)`.
    ///
    /// # Panics (debug only)
    ///
    /// Panics if `start > end` (inverted span). In release builds the
    /// invariant is unchecked for performance.
    pub fn new(start: usize, end: usize) -> Self {
        debug_assert!(
            start <= end,
            "Span::new: start ({start}) > end ({end}) — inverted span"
        );
        Span { start, end }
    }

    /// Creates a zero-length span at `pos`.
    pub fn at(pos: usize) -> Self {
        Span { start: pos, end: pos }
    }

    /// Returns the length of the span in bytes.
    pub fn len(&self) -> usize {
        self.end - self.start
    }

    /// Returns `true` if the span is zero-length.
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{}", self.start, self.end)
    }
}
