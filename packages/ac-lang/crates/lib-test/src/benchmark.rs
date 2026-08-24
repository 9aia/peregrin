use std::time::Instant;

use ac_runtime::{Context, Function};
use ac_runtime_modules::Module;
use ac_runtime_values::{Error, NonFunctionValue};

use crate::error::TestError;
use crate::shared::{call_function, obj, require_function, require_integer};

/// The `benchmark` module — simple wall-clock timing for functions.
///
/// Functions: `benchmark.run`.
#[derive(Clone, Copy)]
pub struct BenchmarkModule;

impl Module for BenchmarkModule {
    fn name(&self) -> &str {
        "benchmark"
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
        let iterations = require_integer(&args[1])?;
        if iterations <= 0 {
            return Err(TestError::AssertionFailed {
                message: "iterations must be positive".to_string(),
            }
            .into());
        }

        let bench_args: Vec<NonFunctionValue> = if args.len() >= 3 {
            match args[2].as_array() {
                Some(arr) => arr.iter().map(|v| NonFunctionValue::new(v.clone())).collect(),
                None => {
                    return Err(TestError::TypeMismatch {
                        expected: "array",
                        got: crate::shared::type_name(&args[2]),
                    }
                    .into());
                }
            }
        } else {
            Vec::new()
        };

        let start = Instant::now();
        let mut last_result = None;
        for _ in 0..iterations {
            last_result = Some(call_function(ctx, &func, &bench_args)?);
        }
        let total = start.elapsed().as_millis() as f64;

        Ok(obj(&[
            ("iterations", serde_json::Value::from(iterations)),
            ("total_ms", serde_json::Value::from(total)),
            ("mean_ms", serde_json::Value::from(total / iterations as f64)),
            (
                "result",
                last_result
                    .map(|v| v.into_inner())
                    .unwrap_or(serde_json::Value::Null),
            ),
        ]))
    }
}
