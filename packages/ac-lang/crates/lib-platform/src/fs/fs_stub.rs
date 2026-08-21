//! In-memory [`Fs`] stub for testing.

use std::collections::HashMap;
use std::sync::Mutex;

use crate::error::PlatformError;
use crate::fs::Fs;

/// A simple in-memory filesystem for tests.
///
/// Stores file contents in a `HashMap<String, String>`. Directories are
/// implicit — `list_dir` returns files whose path starts with the given
/// prefix.
#[derive(Default)]
pub struct StubFs {
    pub(crate) files: Mutex<HashMap<String, String>>,
}

impl StubFs {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_file(path: impl Into<String>, content: impl Into<String>) -> Self {
        let mut files = HashMap::new();
        files.insert(path.into(), content.into());
        StubFs {
            files: Mutex::new(files),
        }
    }
}

impl Fs for StubFs {
    fn read_file(&self, path: &str) -> Result<String, PlatformError> {
        self.files
            .lock()
            .unwrap()
            .get(path)
            .cloned()
            .ok_or(PlatformError::NotFound { path: path.into() })
    }

    fn write_file(&self, path: &str, content: &str) -> Result<(), PlatformError> {
        self.files
            .lock()
            .unwrap()
            .insert(path.into(), content.into());
        Ok(())
    }

    fn exists(&self, path: &str) -> Result<bool, PlatformError> {
        Ok(self.files.lock().unwrap().contains_key(path))
    }

    fn remove(&self, path: &str) -> Result<(), PlatformError> {
        self.files
            .lock()
            .unwrap()
            .remove(path)
            .map(|_| ())
            .ok_or(PlatformError::NotFound { path: path.into() })
    }

    fn list_dir(&self, prefix: &str) -> Result<Vec<String>, PlatformError> {
        let entries: Vec<String> = self
            .files
            .lock()
            .unwrap()
            .keys()
            .filter(|k| k.starts_with(prefix))
            .cloned()
            .collect();
        Ok(entries)
    }
}
