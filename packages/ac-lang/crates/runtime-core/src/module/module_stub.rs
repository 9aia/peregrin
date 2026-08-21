//! Shared test stub for the module interface.

use ac_runtime::RuntimeValue;
use ac_vm::{Context, Error, Function};

use crate::module::Module;

/// A minimal module implementation for testing.
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
struct Echo;

impl Function for Echo {
    fn call(&self, args: &[RuntimeValue], _ctx: &mut Context) -> Result<RuntimeValue, Error> {
        Ok(args.first().cloned().unwrap_or_default())
    }
}

/// A static [`StubModule`] instance for convenience.
pub static STUB_MODULE: StubModule = StubModule;
