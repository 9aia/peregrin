use ac_runtime_values::NonFunctionValue;
use std::sync::Arc;

use ac_runtime::{Context, Function};
use ac_runtime_modules::Module;
use ac_runtime_values::Error;

use crate::error::OsError;

/// Abstract filesystem service.
///
/// Defines file I/O operations without tying code to a concrete platform.
/// Implementations:
/// - `ac-lib-os-std` — backed by `std::fs`
/// - [`fs_stub::StubFs`] — in-memory for testing
pub trait Fs: Send + Sync {
    fn read_file(&self, path: &str) -> Result<String, OsError>;
    fn write_file(&self, path: &str, content: &str) -> Result<(), OsError>;
    fn exists(&self, path: &str) -> Result<bool, OsError>;
    fn remove(&self, path: &str) -> Result<(), OsError>;
    fn list_dir(&self, path: &str) -> Result<Vec<String>, OsError>;
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

    fn function(&self, name: &str) -> Option<&dyn Function<NonFunctionValue>> {
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

fn require_string_arg(args: &[NonFunctionValue], expected: usize) -> Result<Vec<&str>, Error> {
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

impl Function<NonFunctionValue> for ReadFileFn {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let path = require_string_arg(args, 1)?[0];
        self.fs
            .read_file(path)
            .map(NonFunctionValue::new)
            .map_err(Into::into)
    }
}

struct WriteFileFn {
    fs: Arc<dyn Fs>,
}

impl Function<NonFunctionValue> for WriteFileFn {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let path = require_string_arg(args, 2)?[0];
        let content = args[1].as_str().ok_or(Error::TypeMismatch {
            expected: "string",
            got: "non-string",
        })?;
        self.fs.write_file(path, content)?;
        Ok(NonFunctionValue::default())
    }
}

struct ExistsFn {
    fs: Arc<dyn Fs>,
}

impl Function<NonFunctionValue> for ExistsFn {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let path = require_string_arg(args, 1)?[0];
        self.fs
            .exists(path)
            .map(NonFunctionValue::new)
            .map_err(Into::into)
    }
}

struct RemoveFn {
    fs: Arc<dyn Fs>,
}

impl Function<NonFunctionValue> for RemoveFn {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let path = require_string_arg(args, 1)?[0];
        self.fs.remove(path)?;
        Ok(NonFunctionValue::default())
    }
}

struct ListDirFn {
    fs: Arc<dyn Fs>,
}

impl Function<NonFunctionValue> for ListDirFn {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let path = require_string_arg(args, 1)?[0];
        let entries = self.fs.list_dir(path)?;
        let arr: Vec<serde_json::Value> =
            entries.into_iter().map(serde_json::Value::String).collect();
        Ok(NonFunctionValue::new(serde_json::Value::Array(arr)))
    }
}
