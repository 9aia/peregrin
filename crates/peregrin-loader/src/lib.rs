//! peregrin-loader — the loader port. Impls: `peregrin-loader-yaml`.

/// Where a campaign came from — error messages carry it.
pub trait Loader: Send + Sync {
    /// Load a campaign by path/uri.
    ///
    /// # Errors
    /// `String` for now — typed errors land with the yaml impl.
    fn load(&self, source: &str) -> Result<peregrin_core::Campaign, String>;
}
