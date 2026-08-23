use std::fs;

use ac_lib_platform::{PlatformError, Tmp};

/// Temporary file/directory implementation backed by `std::env::temp_dir`.
#[derive(Default)]
pub struct StdTmp;

impl Tmp for StdTmp {
    fn file(&self) -> Result<String, PlatformError> {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("ac-{}-{}", std::process::id(), unique_suffix()));
        Ok(path.to_string_lossy().into_owned())
    }

    fn dir(&self) -> Result<String, PlatformError> {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("ac-dir-{}-{}", std::process::id(), unique_suffix()));
        fs::create_dir_all(&path).map_err(|e| PlatformError::Io {
            message: e.to_string()})?;
        Ok(path.to_string_lossy().into_owned())
    }
}

fn unique_suffix() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("{n}")
}
