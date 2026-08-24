//! Canned-path [`Tmp`] stub for testing.

use std::sync::atomic::{AtomicUsize, Ordering};

use crate::error::OsError;
use crate::tmp::Tmp;

/// A stub that returns sequential temporary paths.
#[derive(Default)]
pub struct StubTmp {
    counter: AtomicUsize,
}

impl Tmp for StubTmp {
    fn file(&self) -> Result<String, OsError> {
        let n = self.counter.fetch_add(1, Ordering::SeqCst);
        Ok(format!("/tmp/stub-file-{n}"))
    }

    fn dir(&self) -> Result<String, OsError> {
        let n = self.counter.fetch_add(1, Ordering::SeqCst);
        Ok(format!("/tmp/stub-dir-{n}"))
    }
}
