//! peregrin-config — the config port. Impls: `peregrin-config-toml`.
//! Same shape as the sturn/bupha port.

/// A config source — layered reads merge, first hit wins per key.
pub trait ConfigSource: Send + Sync {
    /// Read one key; None means "not here" — the caller tries the
    /// next source in the chain.
    fn get(&self, key: &str) -> Option<serde_json::Value>;
}
