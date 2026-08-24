//! Frontend trait — the abstract interface for format parsers.
//!
//! Each frontend (JSON, YAML, TOML, etc.) implements this trait.
//! The trait is generic over the error type, allowing each frontend
//! to expose its own error type while sharing a common interface.
//!
//! ## Architectural rule
//!
//! > **Formats translate external representations.** The `Frontend`
//! > trait formalizes this contract — any format that can parse into
//! > `NonFunctionValue` and serialize back can implement it.

use ac_runtime_config::NonFunctionValue;
use ac_source::SourceMap;

/// A format frontend that parses and serializes `NonFunctionValue`s.
pub trait Frontend {
    /// The error type returned by parse and serialize operations.
    type Error;

    /// Parses a string in this format into a [`NonFunctionValue`].
    fn parse(&self, input: &str) -> Result<NonFunctionValue, Self::Error>;

    /// Serializes a [`NonFunctionValue`] to a string in this format.
    fn serialize(&self, value: &NonFunctionValue) -> Result<String, Self::Error>;

    /// Parses a string and builds a [`SourceMap`] mapping dotted paths
    /// to [`ac_source::SourceLocation`]s.
    ///
    /// Default implementation calls [`parse`](Self::parse) and returns
    /// an empty `SourceMap`. Frontends override this to provide
    /// format-specific location tracking.
    fn parse_with_locations(
        &self,
        input: &str,
    ) -> Result<(NonFunctionValue, SourceMap), Self::Error> {
        let value = self.parse(input)?;
        Ok((value, SourceMap::new()))
    }
}
