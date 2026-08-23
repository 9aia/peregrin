use rand::Rng;
use serde_json::Value;

use ac_runtime::NonFunctionValue;
use ac_runtime::{Context, Error, Function};

use super::common::{array, require_int, s};
use crate::error::RandError;

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
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
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
                });
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
                .into());
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
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
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
    fn call(
        &self,
        _args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let mut rng = rand::rng();
        let mut bytes = [0u8; 16];
        rng.fill(&mut bytes);

        bytes[6] = (bytes[6] & 0x0F) | 0x40;
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
