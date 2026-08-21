use std::sync::Arc;

use ac_runtime::RuntimeValue;
use ac_runtime_core::Module;
use ac_vm::{Context, Error, Function};

use crate::error::PlatformError;

/// Abstract filesystem service.
///
/// Defines file I/O operations without tying code to a concrete platform.
/// Implementations:
/// - `ac-runtime-platform-std` (future) — backed by `std::fs`
/// - [`fs_stub::StubFs`] — in-memory for testing
pub trait Fs: Send + Sync {
    fn read_file(&self, path: &str) -> Result<String, PlatformError>;
    fn write_file(&self, path: &str, content: &str) -> Result<(), PlatformError>;
    fn exists(&self, path: &str) -> Result<bool, PlatformError>;
    fn remove(&self, path: &str) -> Result<(), PlatformError>;
    fn list_dir(&self, path: &str) -> Result<Vec<String>, PlatformError>;
}

/// Module adapter for [`Fs`] — exposes `fs.*` functions.
pub struct FsModule {
    read: ReadFileFn,
    write: WriteFileFn,
    exists: ExistsFn,
    remove: RemoveFn,
    list: ListDirFn,
}

impl FsModule {
    pub fn new(fs: impl Fs + 'static) -> Self {
        let fs = Arc::new(fs);
        FsModule {
            read: ReadFileFn { fs: fs.clone() },
            write: WriteFileFn { fs: fs.clone() },
            exists: ExistsFn { fs: fs.clone() },
            remove: RemoveFn { fs: fs.clone() },
            list: ListDirFn { fs },
        }
    }
}

impl Module for FsModule {
    fn name(&self) -> &str {
        "fs"
    }

    fn function(&self, name: &str) -> Option<&dyn Function> {
        match name {
            "read" => Some(&self.read),
            "write" => Some(&self.write),
            "exists" => Some(&self.exists),
            "remove" => Some(&self.remove),
            "list" => Some(&self.list),
            _ => None,
        }
    }
}

fn require_string_arg(args: &[RuntimeValue], expected: usize) -> Result<Vec<&str>, Error> {
    if args.len() < expected {
        return Err(Error::ArgumentCount {
            expected,
            got: args.len(),
        });
    }
    args.iter()
        .take(expected)
        .map(|v| {
            v.as_str().ok_or(Error::TypeMismatch {
                expected: "string",
                got: "non-string",
            })
        })
        .collect()
}

struct ReadFileFn {
    fs: Arc<dyn Fs>,
}

impl Function for ReadFileFn {
    fn call(&self, args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
        let path = require_string_arg(args, 1)?[0];
        self.fs
            .read_file(path)
            .map(RuntimeValue::new)
            .map_err(Into::into)
    }
}

struct WriteFileFn {
    fs: Arc<dyn Fs>,
}

impl Function for WriteFileFn {
    fn call(&self, args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
        let path = require_string_arg(args, 2)?[0];
        let content = args[1].as_str().ok_or(Error::TypeMismatch {
            expected: "string",
            got: "non-string",
        })?;
        self.fs.write_file(path, content)?;
        Ok(RuntimeValue::default())
    }
}

struct ExistsFn {
    fs: Arc<dyn Fs>,
}

impl Function for ExistsFn {
    fn call(&self, args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
        let path = require_string_arg(args, 1)?[0];
        self.fs
            .exists(path)
            .map(RuntimeValue::new)
            .map_err(Into::into)
    }
}

struct RemoveFn {
    fs: Arc<dyn Fs>,
}

impl Function for RemoveFn {
    fn call(&self, args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
        let path = require_string_arg(args, 1)?[0];
        self.fs.remove(path)?;
        Ok(RuntimeValue::default())
    }
}

struct ListDirFn {
    fs: Arc<dyn Fs>,
}

impl Function for ListDirFn {
    fn call(&self, args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
        let path = require_string_arg(args, 1)?[0];
        let entries = self.fs.list_dir(path)?;
        let arr: Vec<serde_json::Value> =
            entries.into_iter().map(serde_json::Value::String).collect();
        Ok(RuntimeValue::new(serde_json::Value::Array(arr)))
    }
}
