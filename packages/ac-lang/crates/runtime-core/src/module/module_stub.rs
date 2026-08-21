//! Shared test stub for the module interface.

use ac_runtime::NonFunctionValue;
use ac_vm::{Context, Error, Function};

use crate::module::Module;

/// A minimal module implementation for testing.
#[allow(dead_code)]
pub struct StubModule;

impl Module for StubModule {
    fn name(&self) -> &str {
        "stub"
    }

    fn function(&self, name: &str) -> Option<&dyn Function> {
        match name {
            "echo" => Some(&Echo),
            _ => None,
        }
    }
}

#[derive(Clone)]
#[allow(dead_code)]
struct Echo;

impl Function for Echo {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        Ok(args.first().cloned().unwrap_or_default())
    }
}

/// A static [`StubModule`] instance for convenience.
#[allow(dead_code)]
pub static STUB_MODULE: StubModule = StubModule;
