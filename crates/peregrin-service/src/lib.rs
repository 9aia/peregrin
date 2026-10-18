//! peregrin-service — the service port: install/start/stop the node
//! or hub as an OS service. Impls: `peregrin-systemd` (sturn shape).

/// An installed service unit.
pub trait Service: Send + Sync {
    /// Human-readable status line(s) for `peregrin service status`.
    fn status(&self) -> std::io::Result<String>;
    /// Restart the unit.
    fn restart(&self) -> std::io::Result<()>;
}
