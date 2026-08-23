use std::sync::Arc;

use ac_runtime::NonFunctionValue;
use ac_runtime::{Context, Error, Function};
use ac_runtime_modules::Module;

use crate::error::OsError;

/// Abstract time service.
///
/// Implementations:
/// - `ac-lib-os-std` — backed by `std::time::SystemTime`
/// - [`time_stub::StubTime`] — fixed time for testing
pub trait Time: Send + Sync {
    /// Returns the current time as epoch milliseconds.
    fn now_millis(&self) -> Result<i64, OsError>;
    /// Returns the current time as epoch seconds.
    fn now_seconds(&self) -> Result<i64, OsError>;
}

/// Module adapter for [`Time`] — exposes `time.*` functions.
pub struct TimeModule {
    now: NowFn,
    epoch: EpochFn,
}

impl TimeModule {
    pub fn new(time: impl Time + 'static) -> Self {
        let time = Arc::new(time);
        TimeModule {
            now: NowFn { time: time.clone() },
            epoch: EpochFn { time },
        }
    }
}

impl Module for TimeModule {
    fn name(&self) -> &str {
        "time"
    }

    fn function(&self, name: &str) -> Option<&dyn Function> {
        match name {
            "now" => Some(&self.now),
            "epoch" => Some(&self.epoch),
            _ => None,
        }
    }
}

struct NowFn {
    time: Arc<dyn Time>,
}

impl Function for NowFn {
    fn call(
        &self,
        _args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        self.time
            .now_millis()
            .map(|ms| NonFunctionValue::new(serde_json::json!(ms)))
            .map_err(Into::into)
    }
}

struct EpochFn {
    time: Arc<dyn Time>,
}

impl Function for EpochFn {
    fn call(
        &self,
        _args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        self.time
            .now_seconds()
            .map(|s| NonFunctionValue::new(serde_json::json!(s)))
            .map_err(Into::into)
    }
}
