use rand::prelude::IndexedRandom;
use rand::Rng;
use rand::seq::SliceRandom;
use serde_json::Value;

use ac_runtime::NonFunctionValue;
use ac_vm::{Context, Error, Function};

use super::common::{array, bool_val, num, require_array, require_int, require_number, s};
use crate::error::RandError;

/// `rand.int(max: number) → number` / `rand.int(min: number, max: number) → number`
///
/// Returns a random integer in the inclusive range `[0, max]` (one-arg)
/// or `[min, max]` (two-arg).
///
/// Both `min` and `max` must be integers and `min` must be <= `max`.
///
/// Examples:
/// - `rand.int(6)` → 0–6 inclusive
/// - `rand.int(1, 6)` → a die roll (1–6)
/// - `rand.int(0, 100)` → 0–100 inclusive
#[derive(Clone, Copy)]
pub struct Int;

impl Function for Int {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let (min, max) = match args.len() {
            1 => (0, require_int(args, 0)?),
            2 => (require_int(args, 0)?, require_int(args, 1)?),
            n => {
                return Err(Error::ArgumentCount {
                    expected: 2,
                    got: n,
                })
            }
        };
        if min > max {
            return Err(RandError::InvalidRange {
                min: min as f64,
                max: max as f64,
            }
            .into());
        }
        let result = rand::rng().random_range(min..=max);
        Ok(num(result as f64))
    }
}

/// `rand.float() → number` / `rand.float(max: number) → number` / `rand.float(min: number, max: number) → number`
///
/// Returns a random float. With no arguments, the result is in `[0, 1)`.
/// With one argument, the result is in `[0, max)`.
/// With two arguments, the result is in `[min, max)`.
///
/// Examples:
/// - `rand.float()` → 0.0 ≤ x < 1.0
/// - `rand.float(10)` → 0.0 ≤ x < 10.0
/// - `rand.float(10, 20)` → 10.0 ≤ x < 20.0
#[derive(Clone, Copy)]
pub struct Float;

impl Function for Float {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        match args.len() {
            0 => {
                let result = rand::rng().random::<f64>();
                Ok(num(result))
            }
            1 => {
                let max = require_number(args, 0)?;
                if !max.is_finite() {
                    return Err(RandError::InvalidRange { min: 0.0, max }.into());
                }
                if max <= 0.0 {
                    return Err(RandError::InvalidRange { min: 0.0, max }.into());
                }
                let result = rand::rng().random_range(0.0..max);
                Ok(num(result))
            }
            2 => {
                let min = require_number(args, 0)?;
                let max = require_number(args, 1)?;
                if !min.is_finite() || !max.is_finite() {
                    return Err(RandError::InvalidRange { min, max }.into());
                }
                if min > max {
                    return Err(RandError::InvalidRange { min, max }.into());
                }
                if min == max {
                    return Ok(num(min));
                }
                let result = rand::rng().random_range(min..max);
                Ok(num(result))
            }
            n => Err(Error::ArgumentCount {
                expected: 2,
                got: n,
            }),
        }
    }
}

/// `rand.bool() → bool`
///
/// Returns a random boolean with equal probability for `true` and `false`.
///
/// Example:
/// - `rand.bool()` → `true` or `false`
#[derive(Clone, Copy)]
pub struct Bool;

impl Function for Bool {
    fn call(&self, _args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        Ok(bool_val(rand::rng().random()))
    }
}

/// `rand.pick(array: array) → value`
///
/// Returns a single random element from the array.
///
/// Returns an error if the array is empty.
///
/// Example:
/// - `rand.pick(["a", "b", "c"])` → one of `"a"`, `"b"`, `"c"`
#[derive(Clone, Copy)]
pub struct Pick;

impl Function for Pick {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let arr = require_array(args, 0)?;
        if arr.is_empty() {
            return Err(RandError::EmptyArray.into());
        }
        let choice = arr.choose(&mut rand::rng()).expect("non-empty array");
        Ok(NonFunctionValue::new(choice.clone()))
    }
}

/// `rand.sample(array: array, count: number) → array`
///
/// Returns `count` elements chosen uniformly at random **without
/// replacement** from the input array.
///
/// `count` must be a non-negative integer and must not exceed the array
/// length.
///
/// Example:
/// - `rand.sample([1, 2, 3, 4, 5], 3)` → 3 distinct elements in random order
#[derive(Clone, Copy)]
pub struct Sample;

impl Function for Sample {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let arr = require_array(args, 0)?;
        let count = require_int(args, 1)?;

        if count < 0 {
            return Err(RandError::InvalidSampleCount {
                requested: count as usize,
                available: arr.len(),
            }
            .into());
        }
        let count = count as usize;
        if count > arr.len() {
            return Err(RandError::InvalidSampleCount {
                requested: count,
                available: arr.len(),
            }
            .into());
        }

        let mut indices: Vec<usize> = (0..arr.len()).collect();
        indices.shuffle(&mut rand::rng());
        let picked: Vec<Value> = indices[..count]
            .iter()
            .map(|&i| arr[i].clone())
            .collect();
        Ok(array(picked))
    }
}

/// `rand.shuffle(array: array) → array`
///
/// Returns a new array with the elements of the input in random order.
/// The original array is not modified (values are immutable in AC).
///
/// Example:
/// - `rand.shuffle([1, 2, 3, 4, 5])` → e.g. `[3, 1, 5, 2, 4]`
#[derive(Clone, Copy)]
pub struct Shuffle;

impl Function for Shuffle {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let arr = require_array(args, 0)?;
        let mut shuffled: Vec<Value> = arr.to_vec();
        shuffled.shuffle(&mut rand::rng());
        Ok(array(shuffled))
    }
}

/// `rand.weighted_pick(items: array, weights: array<number>) → value`
///
/// Returns a single element from `items` chosen with probability
/// proportional to the corresponding weight in `weights`.
///
/// Both arrays must have the same length, and at least one weight must be
/// positive.
///
/// Example:
/// - `rand.weighted_pick(["a", "b", "c"], [1, 1, 2])` → `"c"` twice as likely
#[derive(Clone, Copy)]
pub struct WeightedPick;

impl Function for WeightedPick {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let items = require_array(args, 0)?;
        let weights = require_array(args, 1)?;

        if items.is_empty() {
            return Err(RandError::EmptyArray.into());
        }
        if items.len() != weights.len() {
            return Err(RandError::WeightsLengthMismatch {
                items: items.len(),
                weights: weights.len(),
            }
            .into());
        }

        let weight_values: Vec<f64> = weights
            .iter()
            .enumerate()
            .map(|(i, w)| {
                w.as_f64().ok_or(RandError::NonNumericWeight { index: i })
            })
            .collect::<Result<_, _>>()?;

        let total: f64 = weight_values.iter().sum();
        if total <= 0.0 || !total.is_finite() {
            return Err(RandError::NonPositiveWeights.into());
        }

        let mut rng = rand::rng();
        let threshold = rng.random::<f64>() * total;
        let mut cumulative = 0.0;
        for (i, &w) in weight_values.iter().enumerate() {
            cumulative += w;
            if threshold <= cumulative {
                return Ok(NonFunctionValue::new(items[i].clone()));
            }
        }

        // Fallback to last element (handles floating-point edge cases)
        Ok(NonFunctionValue::new(items[items.len() - 1].clone()))
    }
}

/// `rand.string(length: number, charset?: string) → string`
///
/// Returns a random string of the given length using the specified
/// character set.
///
/// `length` must be a non-negative integer. `charset` defaults to
/// `"alphanumeric"` and must be one of:
///
/// - `"alphanumeric"` — `a-z`, `A-Z`, `0-9` (default)
/// - `"alpha"` — `a-z`, `A-Z`
/// - `"numeric"` — `0-9`
/// - `"hex"` — `0-9`, `a-f`
/// - `"lower"` — `a-z`
/// - `"upper"` — `A-Z`
///
/// Examples:
/// - `rand.string(16)` → e.g. `"aB3xK9mN2pQr7sT1"`
/// - `rand.string(8, "hex")` → e.g. `"3f7a1c9d"`
/// - `rand.string(10, "numeric")` → e.g. `"4729103856"`
#[derive(Clone, Copy)]
pub struct String_;

impl Function for String_ {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let length = require_int(args, 0)?;
        if length < 0 {
            return Err(RandError::InvalidStringLength { length }.into());
        }

        let charset_name = match args.len() {
            1 => "alphanumeric",
            2 => {
                let val = args.get(1).ok_or(Error::ArgumentCount {
                    expected: 2,
                    got: 1,
                })?;
                val.as_str().ok_or(Error::TypeMismatch {
                    expected: "string",
                    got: "non-string",
                })?
            }
            n => {
                return Err(Error::ArgumentCount {
                    expected: 2,
                    got: n,
                })
            }
        };

        let charset: &[u8] = match charset_name {
            "alphanumeric" => b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789",
            "alpha" => b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz",
            "numeric" => b"0123456789",
            "hex" => b"0123456789abcdef",
            "lower" => b"abcdefghijklmnopqrstuvwxyz",
            "upper" => b"ABCDEFGHIJKLMNOPQRSTUVWXYZ",
            _ => {
                return Err(RandError::InvalidCharset {
                    name: charset_name.to_string(),
                }
                .into())
            }
        };

        let mut rng = rand::rng();
        let result: String = (0..length)
            .map(|_| {
                let idx = rng.random_range(0..charset.len());
                charset[idx] as char
            })
            .collect();
        Ok(s(result))
    }
}

/// `rand.bytes(length: number) → array<number>`
///
/// Returns an array of `length` random bytes (integers in the range 0–255).
///
/// `length` must be a non-negative integer.
///
/// Example:
/// - `rand.bytes(8)` → e.g. `[42, 255, 0, 128, 1, 64, 200, 17]`
#[derive(Clone, Copy)]
pub struct Bytes;

impl Function for Bytes {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let length = require_int(args, 0)?;
        if length < 0 {
            return Err(RandError::InvalidByteLength { length }.into());
        }

        let mut rng = rand::rng();
        let result: Vec<Value> = (0..length)
            .map(|_| Value::Number(serde_json::Number::from(rng.random::<u8>())))
            .collect();
        Ok(array(result))
    }
}

/// `rand.uuid() → string`
///
/// Returns a version-4 UUID string in the canonical 8-4-4-4-12 format.
///
/// Version and variant bits are set per RFC 4122. The randomness is
/// sourced from the system CSPRNG via the `rand` crate.
///
/// Example:
/// - `rand.uuid()` → `"f47ac10b-58cc-4372-a567-0e02b2c3d479"`
#[derive(Clone, Copy)]
pub struct Uuid;

impl Function for Uuid {
    fn call(&self, _args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let mut rng = rand::rng();
        let mut bytes = [0u8; 16];
        rng.fill(&mut bytes);

        // Set version 4 (bits 12-15 of byte 6)
        bytes[6] = (bytes[6] & 0x0F) | 0x40;
        // Set variant 1 (bits 6-7 of byte 8)
        bytes[8] = (bytes[8] & 0x3F) | 0x80;

        let hex: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
        let uuid = format!(
            "{}-{}-{}-{}-{}",
            &hex[0..8],
            &hex[8..12],
            &hex[12..16],
            &hex[16..20],
            &hex[20..32],
        );
        Ok(s(uuid))
    }
}

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

// ── Perlin noise ─────────────────────────────────────────────────

const PERM: [u8; 256] = [
    151, 160, 137, 91, 90, 15, 131, 13, 201, 95, 96, 53, 194, 233, 7, 225,
    140, 36, 103, 30, 69, 142, 8, 99, 37, 240, 21, 10, 23, 190, 6, 148,
    247, 120, 234, 75, 0, 26, 197, 62, 94, 252, 219, 203, 117, 35, 11, 32,
    57, 177, 33, 88, 237, 149, 56, 87, 174, 20, 125, 136, 171, 168, 68, 175,
    74, 165, 71, 134, 139, 48, 27, 166, 77, 146, 158, 231, 83, 111, 229, 122,
    60, 211, 133, 230, 220, 105, 92, 41, 55, 46, 245, 40, 244, 102, 143, 54,
    65, 25, 63, 161, 1, 216, 80, 73, 209, 76, 132, 187, 208, 89, 18, 169,
    200, 196, 135, 130, 116, 188, 159, 86, 164, 100, 109, 198, 173, 186, 3, 64,
    52, 217, 226, 250, 124, 123, 5, 202, 38, 147, 118, 126, 255, 82, 85, 212,
    207, 206, 59, 227, 47, 16, 58, 17, 182, 189, 28, 42, 223, 183, 170, 213,
    119, 248, 152, 2, 44, 154, 163, 70, 221, 153, 101, 155, 167, 43, 172, 9,
    129, 22, 39, 253, 19, 98, 108, 110, 79, 113, 224, 232, 178, 185, 112, 104,
    218, 246, 97, 228, 251, 34, 242, 193, 238, 210, 144, 12, 191, 179, 162, 241,
    81, 51, 145, 235, 142, 239, 37, 155, 36, 179, 25, 36, 160, 64, 37, 254,
    155, 116, 86, 245, 21, 166, 140, 146, 234, 197, 111, 208, 191, 39, 213, 153,
    88, 214, 105, 92, 41, 55, 46, 245, 40, 244, 102, 143, 54, 65, 25, 63,
];

fn fade(t: f64) -> f64 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + t * (b - a)
}

fn grad(hash: u8, x: f64, y: f64, z: f64) -> f64 {
    let h = hash & 15;
    let u = if h < 8 { x } else { y };
    let v = if h < 4 { y } else if h == 12 || h == 14 { x } else { z };
    (if h & 1 == 0 { u } else { -u }) + (if h & 2 == 0 { v } else { -v })
}

fn perm_index(i: i32) -> usize {
    let idx = i.rem_euclid(256) as usize;
    PERM[idx] as usize
}

fn perlin_1d(x: f64) -> f64 {
    let xi = x.floor() as i32;
    let xf = x - x.floor();

    let aa = PERM[perm_index(xi)] as f64;
    let ba = PERM[perm_index(xi + 1)] as f64;

    let u = fade(xf);
    lerp(aa, ba, u) / 255.0 * 2.0 - 1.0
}

fn perlin_2d(x: f64, y: f64) -> f64 {
    let xi = x.floor() as i32;
    let yi = y.floor() as i32;
    let xf = x - x.floor();
    let yf = y - y.floor();

    let aa = grad(PERM[(perm_index(xi) + perm_index(yi)) % 256], xf, yf, 0.0);
    let ba = grad(PERM[(perm_index(xi + 1) + perm_index(yi)) % 256], xf - 1.0, yf, 0.0);
    let ab = grad(PERM[(perm_index(xi) + perm_index(yi + 1)) % 256], xf, yf - 1.0, 0.0);
    let bb = grad(PERM[(perm_index(xi + 1) + perm_index(yi + 1)) % 256], xf - 1.0, yf - 1.0, 0.0);

    let u = fade(xf);
    let v = fade(yf);
    lerp(lerp(aa, ba, u), lerp(ab, bb, u), v)
}

fn perlin_3d(x: f64, y: f64, z: f64) -> f64 {
    let xi = x.floor() as i32;
    let yi = y.floor() as i32;
    let zi = z.floor() as i32;
    let xf = x - x.floor();
    let yf = y - y.floor();
    let zf = z - z.floor();

    let p = |i: i32| perm_index(i);

    let aaa = grad(PERM[(p(xi) + p(yi) + p(zi)) % 256], xf, yf, zf);
    let baa = grad(PERM[(p(xi + 1) + p(yi) + p(zi)) % 256], xf - 1.0, yf, zf);
    let aba = grad(PERM[(p(xi) + p(yi + 1) + p(zi)) % 256], xf, yf - 1.0, zf);
    let bba = grad(PERM[(p(xi + 1) + p(yi + 1) + p(zi)) % 256], xf - 1.0, yf - 1.0, zf);
    let aab = grad(PERM[(p(xi) + p(yi) + p(zi + 1)) % 256], xf, yf, zf - 1.0);
    let bab = grad(PERM[(p(xi + 1) + p(yi) + p(zi + 1)) % 256], xf - 1.0, yf, zf - 1.0);
    let abb = grad(PERM[(p(xi) + p(yi + 1) + p(zi + 1)) % 256], xf, yf - 1.0, zf - 1.0);
    let bbb = grad(PERM[(p(xi + 1) + p(yi + 1) + p(zi + 1)) % 256], xf - 1.0, yf - 1.0, zf - 1.0);

    let u = fade(xf);
    let v = fade(yf);
    let w = fade(zf);
    let x1 = lerp(aaa, baa, u);
    let x2 = lerp(aba, bba, u);
    let y1 = lerp(x1, x2, v);
    let x3 = lerp(aab, bab, u);
    let x4 = lerp(abb, bbb, u);
    let y2 = lerp(x3, x4, v);
    lerp(y1, y2, w)
}

/// `rand.perlin_1d(x: number) → number`
///
/// Returns 1D Perlin noise value at coordinate `x`. Output is
/// approximately in the range `[-1, 1]`.
///
/// Uses a fixed permutation table (Ken Perlin's original) for
/// deterministic results across runs.
///
/// Example:
/// - `rand.perlin_1d(0.5)` → noise at x=0.5
#[derive(Clone, Copy)]
pub struct Perlin1D;

impl Function for Perlin1D {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let x = require_number(args, 0)?;
        Ok(num(perlin_1d(x)))
    }
}

/// `rand.perlin_2d(x: number, y: number) → number`
///
/// Returns 2D Perlin noise value at coordinates `(x, y)`. Output is
/// approximately in the range `[-1, 1]`.
///
/// Uses a fixed permutation table (Ken Perlin's original) for
/// deterministic results across runs.
///
/// Example:
/// - `rand.perlin_2d(1.0, 2.0)` → noise at (1, 2)
#[derive(Clone, Copy)]
pub struct Perlin2D;

impl Function for Perlin2D {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let x = require_number(args, 0)?;
        let y = require_number(args, 1)?;
        Ok(num(perlin_2d(x, y)))
    }
}

/// `rand.perlin_3d(x: number, y: number, z: number) → number`
///
/// Returns 3D Perlin noise value at coordinates `(x, y, z)`. Output is
/// approximately in the range `[-1, 1]`.
///
/// Uses a fixed permutation table (Ken Perlin's original) for
/// deterministic results across runs.
///
/// Example:
/// - `rand.perlin_3d(0.1, 0.2, 0.3)` → noise at (0.1, 0.2, 0.3)
#[derive(Clone, Copy)]
pub struct Perlin3D;

impl Function for Perlin3D {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let x = require_number(args, 0)?;
        let y = require_number(args, 1)?;
        let z = require_number(args, 2)?;
        Ok(num(perlin_3d(x, y, z)))
    }
}
