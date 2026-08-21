use rand::Rng;

use ac_runtime::NonFunctionValue;
use ac_vm::{Context, Error, Function};

use super::common::{num, require_number};
use crate::error::RandError;

/// `rand.gauss(mean: number, stddev: number) → number`
///
/// Returns a random float sampled from a normal (Gaussian) distribution
/// with the given `mean` and standard deviation `stddev`.
///
/// Uses the Box-Muller transform. `stddev` must be non-negative.
///
/// Examples:
/// - `rand.gauss(0, 1)` → standard normal (mean 0, stddev 1)
/// - `rand.gauss(100, 15)` → IQ-scale distribution
#[derive(Clone, Copy)]
pub struct Gauss;

impl Function for Gauss {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let mean = require_number(args, 0)?;
        let stddev = require_number(args, 1)?;
        if !stddev.is_finite() || !mean.is_finite() {
            return Err(RandError::InvalidRange {
                min: mean,
                max: stddev,
            }
            .into());
        }
        if stddev < 0.0 {
            return Err(RandError::InvalidRange {
                min: mean,
                max: stddev,
            }
            .into());
        }

        let mut rng = rand::rng();
        let u1: f64 = rng.random::<f64>().max(f64::MIN_POSITIVE);
        let u2: f64 = rng.random::<f64>();
        let z0 = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
        Ok(num(mean + stddev * z0))
    }
}
