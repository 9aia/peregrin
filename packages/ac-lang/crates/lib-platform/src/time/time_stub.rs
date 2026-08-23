//! Fixed-time [`Time`] stub for testing.

use crate::error::PlatformError;
use crate::time::Time;

/// A time service that always returns the same fixed time.
pub struct StubTime {
    millis: i64,
    seconds: i64}

impl StubTime {
    pub fn new(millis: i64, seconds: i64) -> Self {
        StubTime { millis, seconds }
    }
}

impl Default for StubTime {
    fn default() -> Self {
        StubTime {
            millis: 1_700_000_000_000,
            seconds: 1_700_000_000}
    }
}

impl Time for StubTime {
    fn now_millis(&self) -> Result<i64, PlatformError> {
        Ok(self.millis)
    }

    fn now_seconds(&self) -> Result<i64, PlatformError> {
        Ok(self.seconds)
    }
}
