use std::sync::Arc;

use ac_runtime::NonFunctionValue;
use ac_runtime::{Context, Error, Function};
use ac_runtime_modules::Module;

use crate::error::OsError;

/// Abstract temporary file/directory service.
///
/// Implementations:
/// - `ac-lib-os-std` — backed by `std::env::temp_dir`
/// - [`tmp_stub::StubTmp`] — canned paths for testing
pub trait Tmp: Send + Sync {
    fn file(&self) -> Result<String, OsError>;
    fn dir(&self) -> Result<String, OsError>;
}

/// Module adapter for [`Tmp`] — exposes `tmp.*` functions.
pub struct TmpModule {
    file: FileFn,
    dir: DirFn,
}

impl TmpModule {
    pub fn new(tmp: impl Tmp + 'static) -> Self {
        let tmp = Arc::new(tmp);
        TmpModule {
            file: FileFn { tmp: tmp.clone() },
            dir: DirFn { tmp },
        }
    }
}

impl Module for TmpModule {
    fn name(&self) -> &str {
        "tmp"
    }

    fn function(&self, name: &str) -> Option<&dyn Function> {
        match name {
            "file" => Some(&self.file),
            "dir" => Some(&self.dir),
            _ => None,
        }
    }
}

struct FileFn {
    tmp: Arc<dyn Tmp>,
}

impl Function for FileFn {
    fn call(
        &self,
        _args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        self.tmp
            .file()
            .map(NonFunctionValue::new)
            .map_err(Into::into)
    }
}

struct DirFn {
    tmp: Arc<dyn Tmp>,
}

impl Function for DirFn {
    fn call(
        &self,
        _args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        self.tmp
            .dir()
            .map(NonFunctionValue::new)
            .map_err(Into::into)
    }
}
