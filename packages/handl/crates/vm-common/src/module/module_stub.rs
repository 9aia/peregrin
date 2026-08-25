//! Shared test stub for the module interface.

use std::pin::Pin;

use handl_vm::{AsyncFunction, Context, Function, Operation, OperationSig};
use handl_vm_values_json::{Error, NonFunctionValue};

use crate::module::Module;

/// A minimal module implementation for testing.
#[allow(dead_code)]
pub struct StubModule;

impl Module for StubModule {
    fn name(&self) -> &str {
        "stub"
    }

    fn function(&self, name: &str) -> Option<&dyn Function<NonFunctionValue>> {
        match name {
            "echo" => Some(&Echo),
            _ => None,
        }
    }

    fn async_function(&self, name: &str) -> Option<&dyn AsyncFunction<NonFunctionValue>> {
        match name {
            "async_echo" => Some(&AsyncEcho),
            _ => None,
        }
    }

    fn operation(&self, name: &str) -> Option<&dyn Operation<NonFunctionValue>> {
        match name {
            "raise" => Some(&Raise),
            _ => None,
        }
    }
}

#[derive(Clone)]
#[allow(dead_code)]
struct Echo;

impl Function<NonFunctionValue> for Echo {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        Ok(args.first().cloned().unwrap_or_default())
    }
}

#[derive(Clone)]
#[allow(dead_code)]
struct AsyncEcho;

impl AsyncFunction<NonFunctionValue> for AsyncEcho {
    fn call<'a>(
        &'a self,
        args: &'a [NonFunctionValue],
        _ctx: &'a mut (dyn Context<NonFunctionValue> + 'a),
    ) -> Pin<Box<dyn std::future::Future<Output = Result<NonFunctionValue, Error>> + 'a>> {
        Box::pin(async { Ok(args.first().cloned().unwrap_or_default()) })
    }
}

#[derive(Clone)]
#[allow(dead_code)]
struct Raise;

impl Operation<NonFunctionValue> for Raise {
    fn signature(&self) -> OperationSig<NonFunctionValue> {
        OperationSig {
            name: "raise".to_string(),
            params: Vec::new(),
            return_schema: None,
        }
    }

    fn perform(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        Err(Error::Operation {
            name: "raise".to_string(),
            args: args.to_vec(),
        })
    }
}

/// A static [`StubModule`] instance for convenience.
#[allow(dead_code)]
pub static STUB_MODULE: StubModule = StubModule;
