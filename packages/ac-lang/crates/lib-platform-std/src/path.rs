use std::path::Path as StdPathBuf;

use ac_lib_platform::{Path, PlatformError};

/// Path manipulation backed by [`std::path`].
#[derive(Default)]
pub struct StdPath;

impl Path for StdPath {
    fn join(&self, parts: &[&str]) -> Result<String, PlatformError> {
        if parts.is_empty() {
            return Err(PlatformError::InvalidArgument {
                message: "join requires at least one path".into()});
        }
        let mut path = std::path::PathBuf::from(parts[0]);
        for part in &parts[1..] {
            path.push(part);
        }
        Ok(path.to_string_lossy().into_owned())
    }

    fn resolve(&self, parts: &[&str]) -> Result<String, PlatformError> {
        if parts.is_empty() {
            return Err(PlatformError::InvalidArgument {
                message: "resolve requires at least one path".into()});
        }
        let mut path = std::path::PathBuf::from(parts[0]);
        for part in &parts[1..] {
            path.push(part);
        }
        let canonical = path.canonicalize().map_err(|e| PlatformError::Io {
            message: e.to_string()})?;
        Ok(canonical.to_string_lossy().into_owned())
    }

    fn basename(&self, path: &str) -> Result<String, PlatformError> {
        Ok(StdPathBuf::new(path)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default())
    }

    fn dirname(&self, path: &str) -> Result<String, PlatformError> {
        Ok(StdPathBuf::new(path)
            .parent()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default())
    }

    fn extname(&self, path: &str) -> Result<String, PlatformError> {
        Ok(StdPathBuf::new(path)
            .extension()
            .map(|e| format!(".{}", e.to_string_lossy()))
            .unwrap_or_default())
    }
}
