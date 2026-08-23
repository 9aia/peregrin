use rand::Rng;

use ac_runtime::NonFunctionValue;
use ac_runtime::{Context, Error, Function};

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
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
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

/// `rand.exponential(lambda: number) → number`
///
/// Returns a random float sampled from an exponential distribution
/// with rate parameter `lambda`. The result is non-negative.
///
/// `lambda` must be positive.
///
/// Examples:
/// - `rand.exponential(1)` → mean 1.0
/// - `rand.exponential(0.5)` → mean 2.0
#[derive(Clone, Copy)]
pub struct Exponential;

impl Function for Exponential {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let lambda = require_number(args, 0)?;
        if !lambda.is_finite() || lambda <= 0.0 {
            return Err(RandError::InvalidRange {
                min: lambda,
                max: lambda,
            }
            .into());
        }

        let mut rng = rand::rng();
        let u: f64 = rng.random::<f64>().max(f64::MIN_POSITIVE);
        Ok(num(-u.ln() / lambda))
    }
}

/// `rand.poisson(lambda: number) → number`
///
/// Returns a random non-negative integer sampled from a Poisson
/// distribution with rate parameter `lambda`.
///
/// `lambda` must be non-negative.
///
/// Examples:
/// - `rand.poisson(3)` → e.g. 2, 3, 4, 5
/// - `rand.poisson(0)` → always 0
#[derive(Clone, Copy)]
pub struct Poisson;

impl Function for Poisson {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let lambda = require_number(args, 0)?;
        if !lambda.is_finite() || lambda < 0.0 {
            return Err(RandError::InvalidRange {
                min: lambda,
                max: lambda,
            }
            .into());
        }

        if lambda == 0.0 {
            return Ok(num(0.0));
        }

        let mut rng = rand::rng();
        let l = (-lambda).exp();
        let mut k: i64 = 0;
        let mut p: f64 = 1.0;

        loop {
            k += 1;
            p *= rng.random::<f64>();
            if p <= l {
                break;
            }
        }

        Ok(num((k - 1) as f64))
    }
}

/// `rand.binomial(n: number, p: number) → number`
///
/// Returns a random non-negative integer sampled from a binomial
/// distribution with `n` trials and success probability `p`.
///
/// `n` must be a non-negative integer and `p` must be in `[0, 1]`.
///
/// Examples:
/// - `rand.binomial(10, 0.5)` → e.g. 4, 5, 6
/// - `rand.binomial(100, 0.1)` → e.g. 8, 9, 10, 11
#[derive(Clone, Copy)]
pub struct Binomial;

impl Function for Binomial {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let n_trials = require_number(args, 0)?;
        let p = require_number(args, 1)?;

        if !n_trials.is_finite() || !p.is_finite() {
            return Err(RandError::InvalidRange {
                min: n_trials,
                max: p,
            }
            .into());
        }
        if n_trials < 0.0 {
            return Err(RandError::InvalidRange {
                min: n_trials,
                max: p,
            }
            .into());
        }
        if !(0.0..=1.0).contains(&p) {
            return Err(RandError::InvalidRange { min: p, max: p }.into());
        }

        let n = n_trials as u64;
        let mut rng = rand::rng();
        let mut successes: u64 = 0;

        for _ in 0..n {
            if rng.random::<f64>() < p {
                successes += 1;
            }
        }

        Ok(num(successes as f64))
    }
}
