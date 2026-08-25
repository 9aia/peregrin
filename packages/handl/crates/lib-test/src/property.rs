use handl_vm::{Context, Function};
use handl_vm_common::Module;
use handl_vm_values_json::{Error, NonFunctionValue};
use rand::Rng;
use rand::SeedableRng;
use rand::rngs::StdRng;

use crate::error::TestError;
use crate::shared::{b, call_function, require_function, require_integer};

/// The `property` module — property-based testing with seeded RNG.
///
/// Functions: `property.for_all`.
#[derive(Clone, Copy)]
pub struct PropertyModule;

impl Module for PropertyModule {
    fn name(&self) -> &str {
        "property"
    }

    fn function(&self, name: &str) -> Option<&dyn Function<NonFunctionValue>> {
        match name {
            "for_all" => Some(&ForAll),
            _ => None,
        }
    }
}

#[derive(Clone, Copy)]
struct ForAll;
impl Function<NonFunctionValue> for ForAll {
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

        for _ in 0..count {
            let inputs: Vec<NonFunctionValue> = (0..func.params.len())
                .map(|_| NonFunctionValue::new(rng.random::<f64>()))
                .collect();

            match call_function(ctx, &func, &inputs) {
                Ok(result) => {
                    if !result.as_bool().unwrap_or(false) {
                        let counterexample: Vec<_> =
                            inputs.iter().map(|v| v.inner().clone()).collect();
                        return Err(TestError::PropertyFailed {
                            counterexample: serde_json::Value::Array(counterexample),
                            message: "property returned false".to_string(),
                        }
                        .into());
                    }
                }
                Err(e) => {
                    let counterexample: Vec<_> = inputs.iter().map(|v| v.inner().clone()).collect();
                    return Err(TestError::PropertyFailed {
                        counterexample: serde_json::Value::Array(counterexample),
                        message: e.to_string(),
                    }
                    .into());
                }
            }
        }

        Ok(b(true))
    }
}
