use ac_runtime::{Context, Function};
use ac_runtime_modules::Module;
use ac_runtime_values::{Error, NonFunctionValue};
use rand::Rng;
use rand::SeedableRng;
use rand::rngs::StdRng;

use crate::error::TestError;
use crate::shared::{call_function, require_function, require_integer};

/// The `fuzz` module — structure-agnostic random-input stress testing.
///
/// Functions: `fuzz.run`.
#[derive(Clone, Copy)]
pub struct FuzzModule;

impl Module for FuzzModule {
    fn name(&self) -> &str {
        "fuzz"
    }

    fn function(&self, name: &str) -> Option<&dyn Function<NonFunctionValue>> {
        match name {
            "run" => Some(&Run),
            _ => None,
        }
    }
}

#[derive(Clone, Copy)]
struct Run;
impl Function<NonFunctionValue> for Run {
    fn call(
        &self,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(TestError::ArgumentCount {
                expected: 2,
                got: args.len(),
            }
            .into());
        }

        let func = require_function(&args[0])?;
        let count = require_integer(&args[1])?;
        if count <= 0 {
            return Err(TestError::AssertionFailed {
                message: "count must be positive".to_string(),
            }
            .into());
        }

        let seed: u64 = if args.len() >= 3 {
            require_integer(&args[2])? as u64
        } else {
            let mut system_rng = rand::rng();
            system_rng.random::<u64>()
        };

        let mut rng = StdRng::seed_from_u64(seed);
        let mut failures = 0usize;
        let mut first_error = None;

        for _ in 0..count {
            let inputs: Vec<NonFunctionValue> = (0..func.params.len())
                .map(|_| NonFunctionValue::new(rng.random::<f64>()))
                .collect();

            if let Err(e) = call_function(ctx, &func, &inputs) {
                failures += 1;
                if first_error.is_none() {
                    first_error = Some(e.to_string());
                }
            }
        }

        let mut report = serde_json::Map::new();
        report.insert("runs".to_string(), serde_json::Value::from(count));
        report.insert("failures".to_string(), serde_json::Value::from(failures));
        if let Some(msg) = first_error {
            report.insert("first_error".to_string(), serde_json::Value::String(msg));
        }

        Ok(NonFunctionValue::new(serde_json::Value::Object(report)))
    }
}
