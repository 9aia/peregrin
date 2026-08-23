//! Canned-output [`Sig`] stub for testing.

use std::sync::Mutex;

use crate::error::OsError;
use crate::sig::Sig;

/// A signal service with a fixed set of supported signals.
pub struct StubSig {
    supported: Vec<String>,
    log: Mutex<Vec<(i64, String)>>}

impl StubSig {
    pub fn new(signals: Vec<String>) -> Self {
        StubSig {
            supported: signals,
            log: Mutex::new(Vec::new())}
    }
}

impl Default for StubSig {
    fn default() -> Self {
        StubSig::new(vec![
            "SIGINT".into(),
            "SIGTERM".into(),
            "SIGKILL".into(),
            "SIGHUP".into()])
    }
}

impl Sig for StubSig {
    fn send(&self, process_id: i64, signal: &str) -> Result<(), OsError> {
        if !self.supported.iter().any(|s| s == signal) {
            return Err(OsError::InvalidArgument {
                message: format!("unknown signal: {signal}")});
        }
        self.log.lock().unwrap().push((process_id, signal.into()));
        Ok(())
    }

    fn list(&self) -> Result<Vec<String>, OsError> {
        Ok(self.supported.clone())
    }
}
