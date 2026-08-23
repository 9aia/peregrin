use std::time::{SystemTime, UNIX_EPOCH};

use ac_lib_platform::{PlatformError, Time};

/// Time implementation backed by [`std::time::SystemTime`].
#[derive(Default)]
pub struct StdTime;

impl Time for StdTime {
    fn now_millis(&self) -> Result<i64, PlatformError> {
        let duration =
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|e| PlatformError::Io {
                    message: e.to_string()})?;
        Ok(duration.as_millis() as i64)
    }

    fn now_seconds(&self) -> Result<i64, PlatformError> {
        let duration =
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|e| PlatformError::Io {
                    message: e.to_string()})?;
        Ok(duration.as_secs() as i64)
    }
}
