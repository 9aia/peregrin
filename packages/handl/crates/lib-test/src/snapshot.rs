use handl_vm::{Context, Function};
use handl_vm_common::Module;
use handl_vm_values_json::{Error, NonFunctionValue};

use crate::error::TestError;
use crate::shared::{b, require_len, require_string, str};

/// The `snapshot` module — snapshot formatting and matching.
///
/// Functions: `snapshot.format`, `snapshot.match`.
#[derive(Clone, Copy)]
pub struct SnapshotModule;

impl Module for SnapshotModule {
    fn name(&self) -> &str {
        "snapshot"
    }

    fn function(&self, name: &str) -> Option<&dyn Function<NonFunctionValue>> {
        match name {
            "format" => Some(&Format),
            "match" => Some(&Match),
            _ => None,
        }
    }
}

#[derive(Clone, Copy)]
struct Format;
impl Function<NonFunctionValue> for Format {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        require_len(args, 1)?;
        let serialized =
            serde_json::to_string_pretty(args[0].inner()).unwrap_or_else(|_| "null".to_string());
        Ok(str(serialized))
    }
}

#[derive(Clone, Copy)]
struct Match;
impl Function<NonFunctionValue> for Match {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        require_len(args, 3)?;
        let name = require_string(&args[0])?;
        let actual = args[1].inner();
        let expected = args[2].inner();

        if actual != expected {
            return Err(TestError::SnapshotMismatch {
                name: name.to_string(),
                actual: actual.clone(),
                expected: expected.clone(),
            }
            .into());
        }

        Ok(b(true))
    }
}
