//! peregrin-secret — named-secret resolution from a backing store.
//! Impls: `peregrin-keyring` (OS keyring), `peregrin-dotenv`
//! (KEY=VAL file). Same shape as `sturn-secret`.

/// A secret store. `get` returns the value or None (missing is not
/// an error — the caller tries the next store in the chain).
pub trait Secrets: Send + Sync {
    /// Look up a secret by name; None means "not here" — the
    /// caller tries the next store, not an error.
    fn get(&self, name: &str) -> Option<String>;
    /// Persist a value into this store (deploy-time migration).
    /// Stores that cannot write return an error.
    ///
    /// # Errors
    /// `std::io::Error` when the store is read-only or the write fails.
    fn set(&self, name: &str, value: &str) -> std::io::Result<()>;
}
