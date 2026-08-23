use ac_runtime::NonFunctionValue;
use ac_runtime::{Context, Error, Function};
use md5::Md5 as Md5Hasher;
use sha2::{Digest, Sha256 as Sha256Hasher};

use super::common::{extract_bytes, s};

/// `crypto.sha256(data: string | array<number>) → string`
///
/// Computes the SHA-256 hash of the input and returns it as a
/// lowercase hex-encoded string.
///
/// The input may be a string (UTF-8 bytes) or an array of integers
/// in the range 0–255.
///
/// Examples:
/// - `crypto.sha256("hello")` → `"2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"`
/// - `crypto.sha256([104, 101, 108, 108, 111])` → same as above
#[derive(Clone, Copy)]
pub struct Sha256;

impl Function for Sha256 {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let data = extract_bytes(args, 0)?;
        let hash = Sha256Hasher::digest(&data);
        Ok(s(hex::encode(hash)))
    }
}

/// `crypto.md5(data: string | array<number>) → string`
///
/// Computes the MD5 hash of the input and returns it as a
/// lowercase hex-encoded string.
///
/// MD5 is cryptographically broken and must not be used for security
/// purposes. It remains useful for checksums, content addressing, and
/// legacy interoperability.
///
/// Examples:
/// - `crypto.md5("hello")` → `"5d41402abc4b2a76b9719d911017c592"`
/// - `crypto.md5([104, 101, 108, 108, 111])` → same as above
#[derive(Clone, Copy)]
pub struct Md5;

impl Function for Md5 {
    fn call(
        &self,
        args: &[NonFunctionValue],
        _ctx: &mut dyn Context,
    ) -> Result<NonFunctionValue, Error> {
        let data = extract_bytes(args, 0)?;
        let hash = Md5Hasher::digest(&data);
        Ok(s(hex::encode(hash)))
    }
}
