use handl_vm::{Context, Function};
use handl_vm_common::Module;
use handl_vm_values_json::{Error, NonFunctionValue};

use crate::error::TestError;
use crate::shared::{b, call_function, require_bool, require_function, require_len};

/// The `assert` module — assertion primitives for test expressions.
///
/// Functions: `assert.equal`, `assert.not_equal`, `assert.true`,
/// `assert.false`, `assert.contains`, `assert.throws`.
#[derive(Clone, Copy)]
pub struct AssertModule;

impl Module for AssertModule {
    fn name(&self) -> &str {
        "assert"
    }

    fn function(&self, name: &str) -> Option<&dyn Function<NonFunctionValue>> {
        match name {
            "equal" => Some(&Equal),
            "not_equal" => Some(&NotEqual),
            "true" => Some(&True),
            "false" => Some(&False),
            "contains" => Some(&Contains),
            "throws" => Some(&Throws),
            _ => None,
        }
    }
}

#[derive(Clone, Copy)]
struct Equal;
impl Function<NonFunctionValue> for Equal {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        require_len(args, 2)?;
        if args[0].inner() != args[1].inner() {
            return Err(TestError::AssertionFailed {
                message: format!(
                    "expected {} to equal {}",
                    serde_json::to_string(args[0].inner()).unwrap_or_default(),
                    serde_json::to_string(args[1].inner()).unwrap_or_default()
                ),
            }
            .into());
        }
        Ok(b(true))
    }
}

#[derive(Clone, Copy)]
struct NotEqual;
impl Function<NonFunctionValue> for NotEqual {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        require_len(args, 2)?;
        if args[0].inner() == args[1].inner() {
            return Err(TestError::AssertionFailed {
                message: format!(
                    "expected {} to not equal {}",
                    serde_json::to_string(args[0].inner()).unwrap_or_default(),
                    serde_json::to_string(args[1].inner()).unwrap_or_default()
                ),
            }
            .into());
        }
        Ok(b(true))
    }
}

#[derive(Clone, Copy)]
struct True;
impl Function<NonFunctionValue> for True {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        require_len(args, 1)?;
        let value = require_bool(&args[0])?;
        if !value {
            return Err(TestError::AssertionFailed {
                message: "expected true".to_string(),
            }
            .into());
        }
        Ok(b(true))
    }
}

#[derive(Clone, Copy)]
struct False;
impl Function<NonFunctionValue> for False {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        require_len(args, 1)?;
        let value = require_bool(&args[0])?;
        if value {
            return Err(TestError::AssertionFailed {
                message: "expected false".to_string(),
            }
            .into());
        }
        Ok(b(true))
    }
}

#[derive(Clone, Copy)]
struct Contains;
impl Function<NonFunctionValue> for Contains {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        require_len(args, 2)?;
        let haystack = args[0].inner();
        let needle = args[1].inner();

        let found = match (haystack, needle) {
            (serde_json::Value::Array(arr), _) => arr.contains(needle),
            (serde_json::Value::Object(map), serde_json::Value::String(key)) => {
                map.contains_key(key)
            }
            (serde_json::Value::String(s), serde_json::Value::String(part)) => s.contains(part),
            _ => false,
        };

        if !found {
            return Err(TestError::AssertionFailed {
                message: format!("expected {haystack} to contain {needle}"),
            }
            .into());
        }
        Ok(b(true))
    }
}

#[derive(Clone, Copy)]
struct Throws;
impl Function<NonFunctionValue> for Throws {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        require_len(args, 1)?;
        let func = require_function(&args[0])?;
        if call_function(ctx, &func, &[]).is_err() {
            return Ok(b(true));
        }
        Err(TestError::AssertionFailed {
            message: "expected function to throw, but it did not".to_string(),
        }
        .into())
    }
}
