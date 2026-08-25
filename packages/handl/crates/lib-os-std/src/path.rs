use std::path::Path as StdPathBuf;

use handl_lib_os::{OsError, Path};

/// Path manipulation backed by [`std::path`].
#[derive(Default)]
pub struct StdPath;

impl Path for StdPath {
    fn join(&self, parts: &[&str]) -> Result<String, OsError> {
        if parts.is_empty() {
            return Err(OsError::InvalidArgument {
                message: "join requires at least one path".into(),
            });
        }
        let mut path = std::path::PathBuf::from(parts[0]);
        for part in &parts[1..] {
            path.push(part);
        }
        Ok(path.to_string_lossy().into_owned())
    }

    fn resolve(&self, parts: &[&str]) -> Result<String, OsError> {
        if parts.is_empty() {
            return Err(OsError::InvalidArgument {
                message: "resolve requires at least one path".into(),
            });
        }
        let mut path = std::path::PathBuf::from(parts[0]);
        for part in &parts[1..] {
            path.push(part);
        }
        let canonical = path.canonicalize().map_err(|e| OsError::Io {
            message: e.to_string(),
        })?;
        Ok(canonical.to_string_lossy().into_owned())
    }

    fn basename(&self, path: &str) -> Result<String, OsError> {
        Ok(StdPathBuf::new(path)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default())
    }

    fn dirname(&self, path: &str) -> Result<String, OsError> {
        Ok(StdPathBuf::new(path)
            .parent()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default())
    }

    fn extname(&self, path: &str) -> Result<String, OsError> {
        Ok(StdPathBuf::new(path)
            .extension()
            .map(|e| format!(".{}", e.to_string_lossy()))
            .unwrap_or_default())
    }
}
