/// Syntax tokens for AC expressions.
///
/// The interpolation markers are string tokens (may be multi-character).
/// The path separator, call-open, and call-close are single-character tokens.
/// In the Rust implementation, string tokens are `&str` and single-character
/// tokens are `char`; in other languages, both may be represented as strings.
///
/// These tokens are the minimal syntax surface. Every parser, interpolator,
/// and type inferencer in the implementation reads them from the dialect
/// rather than hardcoding strings.
pub trait Syntax: Send + Sync {
    /// The interpolation prefix — e.g. `"${"` for the canonical dialect.
    fn interpolation_prefix(&self) -> &str;

    /// The interpolation suffix — e.g. `"}"` for the canonical dialect.
    fn interpolation_suffix(&self) -> &str;

    /// The path separator — e.g. `'.'` for the canonical dialect.
    fn path_separator(&self) -> char;

    /// The call-open character — e.g. `'('` for the canonical dialect.
    fn call_open(&self) -> char;

    /// The call-close character — e.g. `')'` for the canonical dialect.
    fn call_close(&self) -> char;
}
