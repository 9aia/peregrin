//! POSIX-like [`Path`] stub for testing.

use crate::error::OsError;
use crate::path::Path;

/// A simple POSIX-like path implementation for tests.
#[derive(Default)]
pub struct StubPath;

impl StubPath {
    pub fn new() -> Self {
        Self
    }
}

impl Path for StubPath {
    fn join(&self, parts: &[&str]) -> Result<String, OsError> {
        let joined = parts.join("/");
        let normalized = joined.replace("//", "/");
        Ok(normalized)
    }

    fn resolve(&self, parts: &[&str]) -> Result<String, OsError> {
        if parts.is_empty() {
            return Err(OsError::InvalidArgument {
                message: "resolve requires at least one path".into(),
            });
        }
        let joined = parts.join("/");
        Ok(if joined.starts_with('/') {
            joined
        } else {
            format!("/{joined}")
        })
    }

    fn basename(&self, path: &str) -> Result<String, OsError> {
        Ok(path
            .rsplit('/')
            .next()
            .filter(|s| !s.is_empty())
            .unwrap_or(path)
            .to_string())
    }

    fn dirname(&self, path: &str) -> Result<String, OsError> {
        match path.rfind('/') {
            Some(0) => Ok("/".into()),
            Some(idx) => Ok(path[..idx].to_string()),
            None => Ok("".into()),
        }
    }

    fn extname(&self, path: &str) -> Result<String, OsError> {
        let basename = path.rsplit('/').next().unwrap_or(path);
        match basename.rfind('.') {
            Some(idx) if idx > 0 => Ok(basename[idx..].to_string()),
            _ => Ok("".into()),
        }
    }
}
