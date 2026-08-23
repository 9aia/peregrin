use std::sync::Arc;

use ac_runtime::NonFunctionValue;
use ac_runtime_core::Module;
use ac_vm::{Context, Error, Function};

use crate::error::PlatformError;

/// Abstract path manipulation service.
///
/// Implementations:
/// - `ac-runtime-platform-std` (future) — backed by `std::path`
/// - [`path_stub::StubPath`] — POSIX-like for testing
pub trait Path: Send + Sync {
    fn join(&self, parts: &[&str]) -> Result<String, PlatformError>;
    fn resolve(&self, parts: &[&str]) -> Result<String, PlatformError>;
    fn basename(&self, path: &str) -> Result<String, PlatformError>;
    fn dirname(&self, path: &str) -> Result<String, PlatformError>;
    fn extname(&self, path: &str) -> Result<String, PlatformError>;
}

/// Module adapter for [`Path`] — exposes `path.*` functions.
pub struct PathModule {
    join: JoinFn,
    resolve: ResolveFn,
    basename: BasenameFn,
    dirname: DirnameFn,
    extname: ExtnameFn}

impl PathModule {
    pub fn new(path: impl Path + 'static) -> Self {
        let path = Arc::new(path);
        PathModule {
            join: JoinFn { path: path.clone() },
            resolve: ResolveFn { path: path.clone() },
            basename: BasenameFn { path: path.clone() },
            dirname: DirnameFn { path: path.clone() },
            extname: ExtnameFn { path }}
    }
}

impl Module for PathModule {
    fn name(&self) -> &str {
        "path"
    }

    fn function(&self, name: &str) -> Option<&dyn Function> {
        match name {
            "join" => Some(&self.join),
            "resolve" => Some(&self.resolve),
            "basename" => Some(&self.basename),
            "dirname" => Some(&self.dirname),
            "extname" => Some(&self.extname),
            _ => None}
    }
}

fn collect_string_args(args: &[NonFunctionValue]) -> Result<Vec<&str>, Error> {
    if args.is_empty() {
        return Err(Error::ArgumentCount {
            expected: 1,
            got: 0});
    }
    args.iter()
        .map(|v| {
            v.as_str().ok_or(Error::TypeMismatch {
                expected: "string",
                got: "non-string"})
        })
        .collect()
}

fn require_one_string(args: &[NonFunctionValue]) -> Result<&str, Error> {
    let arg = args.first().ok_or(Error::ArgumentCount {
        expected: 1,
        got: 0})?;
    arg.as_str().ok_or(Error::TypeMismatch {
        expected: "string",
        got: "non-string"})
}

struct JoinFn {
    path: Arc<dyn Path>}

impl Function for JoinFn {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut dyn Context) -> Result<NonFunctionValue, Error> {
        let parts = collect_string_args(args)?;
        self.path
            .join(&parts)
            .map(NonFunctionValue::new)
            .map_err(Into::into)
    }
}

struct ResolveFn {
    path: Arc<dyn Path>}

impl Function for ResolveFn {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut dyn Context) -> Result<NonFunctionValue, Error> {
        let parts = collect_string_args(args)?;
        self.path
            .resolve(&parts)
            .map(NonFunctionValue::new)
            .map_err(Into::into)
    }
}

struct BasenameFn {
    path: Arc<dyn Path>}

impl Function for BasenameFn {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut dyn Context) -> Result<NonFunctionValue, Error> {
        let path = require_one_string(args)?;
        self.path
            .basename(path)
            .map(NonFunctionValue::new)
            .map_err(Into::into)
    }
}

struct DirnameFn {
    path: Arc<dyn Path>}

impl Function for DirnameFn {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut dyn Context) -> Result<NonFunctionValue, Error> {
        let path = require_one_string(args)?;
        self.path
            .dirname(path)
            .map(NonFunctionValue::new)
            .map_err(Into::into)
    }
}

struct ExtnameFn {
    path: Arc<dyn Path>}

impl Function for ExtnameFn {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut dyn Context) -> Result<NonFunctionValue, Error> {
        let path = require_one_string(args)?;
        self.path
            .extname(path)
            .map(NonFunctionValue::new)
            .map_err(Into::into)
    }
}
