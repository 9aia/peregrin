use std::fs;
use std::path::Path as StdPath;

use ac_lib_platform::{Fs, PlatformError};

/// Filesystem implementation backed by [`std::fs`].
#[derive(Default)]
pub struct StdFs;

impl Fs for StdFs {
    fn read_file(&self, path: &str) -> Result<String, PlatformError> {
        fs::read_to_string(path).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => PlatformError::NotFound { path: path.into() },
            std::io::ErrorKind::PermissionDenied => {
                PlatformError::PermissionDenied { path: path.into() }
            }
            _ => PlatformError::Io {
                message: e.to_string()}})
    }

    fn write_file(&self, path: &str, content: &str) -> Result<(), PlatformError> {
        fs::write(path, content).map_err(|e| match e.kind() {
            std::io::ErrorKind::PermissionDenied => {
                PlatformError::PermissionDenied { path: path.into() }
            }
            std::io::ErrorKind::AlreadyExists => PlatformError::AlreadyExists { path: path.into() },
            _ => PlatformError::Io {
                message: e.to_string()}})
    }

    fn exists(&self, path: &str) -> Result<bool, PlatformError> {
        Ok(StdPath::new(path).exists())
    }

    fn remove(&self, path: &str) -> Result<(), PlatformError> {
        let meta = fs::metadata(path).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => PlatformError::NotFound { path: path.into() },
            _ => PlatformError::Io {
                message: e.to_string()}})?;

        if meta.is_dir() {
            fs::remove_dir(path)
        } else {
            fs::remove_file(path)
        }
        .map_err(|e| PlatformError::Io {
            message: e.to_string()})
    }

    fn list_dir(&self, path: &str) -> Result<Vec<String>, PlatformError> {
        let entries = fs::read_dir(path).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => PlatformError::NotFound { path: path.into() },
            _ => PlatformError::Io {
                message: e.to_string()}})?;

        let mut names = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| PlatformError::Io {
                message: e.to_string()})?;
            names.push(entry.file_name().to_string_lossy().into_owned());
        }
        Ok(names)
    }
}
