use std::sync::Arc;

use ac_runtime::RuntimeValue;
use ac_runtime_core::Module;
use ac_vm::{Context, Error, Function};

use crate::error::PlatformError;

/// Abstract temporary file/directory service.
///
/// Implementations:
/// - `ac-lib-platform-std` — backed by `std::env::temp_dir` + `uuid`
/// - [`tmp_stub::StubTmp`] — canned paths for testing
pub trait Tmp: Send + Sync {
    fn file(&self) -> Result<String, PlatformError>;
    fn dir(&self) -> Result<String, PlatformError>;
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
    fn call(&self, _args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
        self.tmp.file().map(RuntimeValue::new).map_err(Into::into)
    }
}

struct DirFn {
    tmp: Arc<dyn Tmp>,
}

impl Function for DirFn {
    fn call(&self, _args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
        self.tmp.dir().map(RuntimeValue::new).map_err(Into::into)
    }
}
