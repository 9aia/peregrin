//! Shared test stub for the module interface.

use std::pin::Pin;

use ac_runtime::NonFunctionValue;
use ac_runtime::{AsyncFunction, Context, Error, Function, Operation, OperationSig};

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

    fn async_function(&self, name: &str) -> Option<&dyn AsyncFunction> {
        match name {
            "async_echo" => Some(&AsyncEcho),
            _ => None,
        }
    }

    fn operation(&self, name: &str) -> Option<&dyn Operation> {
        match name {
            "raise" => Some(&Raise),
            _ => None,
        }
    }
}

#[derive(Clone)]
#[allow(dead_code)]
struct Echo;

impl Function for Echo {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        Ok(args.first().cloned().unwrap_or_default())
    }
}

#[derive(Clone)]
#[allow(dead_code)]
struct AsyncEcho;

impl AsyncFunction for AsyncEcho {
    fn call<'a>(
        &'a self,
        args: &'a [NonFunctionValue],
        _ctx: &'a mut (dyn Context + 'a),
    ) -> Pin<Box<dyn std::future::Future<Output = Result<NonFunctionValue, Error>> + 'a>> {
        Box::pin(async { Ok(args.first().cloned().unwrap_or_default()) })
    }
}

#[derive(Clone)]
#[allow(dead_code)]
struct Raise;

impl Operation for Raise {
    fn signature(&self) -> OperationSig {
        OperationSig {
            name: "raise".to_string(),
            params: Vec::new(),
            return_schema: None,
        }
    }

    fn perform(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
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
